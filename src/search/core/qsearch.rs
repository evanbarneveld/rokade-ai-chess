use crate::board::evaluator::{evaluate_position, MATE_VALUE};
use crate::piece::pieces::{piece_value_cp, Color, PieceType};
use crate::search::core::advanced_search::{MAX_EVAL_VALUE, MIN_EVAL_VALUE, QUIESCENCE_ENABLED, QSEE_PRUNING_ENABLED, MVV_LVA_ENABLED, SEARCH_ABORTED};
use crate::search::management::move_generator::{find_all_capture_moves, find_all_evasion_moves, find_all_valid_moves};
use crate::state::game_state::GameState;
use crate::search::management::see::{see_dest_estimate, QSEE_CAPTURE_TOLERANCE};
// Note: Zobrist key is now maintained incrementally in GameState
use crate::search::state::rep_stack::RepetitionStack;
use crate::search::context::SearchContext;

// Tighter margins with a stronger static evaluator
const FUT_MARGIN: i32 = 80;
const DELTA_MARGIN: i32 = 925; // Queen value (900) + pawn promotion buffer (25)
const MAX_QUIET_PUSHES: usize = 2;

// Track qsearch depth for debug output (thread-local counter)
thread_local! {
    static QSEARCH_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

// Quiescence Search: consider only tactical continuations (captures) unless in check.
pub fn qsearch(
    ctx: &SearchContext,
    game_state: &mut GameState,
    alpha: i32,
    beta: i32,
    rep_stack: &mut RepetitionStack,
) -> i32 {
    qsearch_with_quiescence(ctx, game_state, alpha, beta, rep_stack, QUIESCENCE_ENABLED)
}

pub(crate) fn qsearch_with_quiescence(
    ctx: &SearchContext,
    game_state: &mut GameState,
    mut alpha: i32,
    mut beta: i32,
    rep_stack: &mut RepetitionStack,
    quiescence_enabled: bool,
) -> i32 {
    #[cfg(feature = "debug-search")]
    let qdepth = QSEARCH_DEPTH.with(|d| d.get());
    
    let to_move = game_state.active_color();
    let maximizing = to_move == Color::White;

    #[cfg(feature = "debug-search")] {
        if  qdepth <= 2 {
            let indent = "    ".repeat(qdepth + 1);
            eprintln!("{}[QS] qdepth={} α={} β={} side={:?}", indent, qdepth, alpha, beta, to_move);
        }
    }

    // If quiescence is disabled, check for checkmate/stalemate, then return static evaluation.
    if !quiescence_enabled {
        let moves = find_all_valid_moves(game_state);
        if moves.is_empty() {
            let in_check = game_state.mutable_board().is_side_in_check(to_move);
            if in_check {
                // Checkmate: the side to move is mated
                return if maximizing {
                    -MATE_VALUE + 100  // White is mated (very bad for White)
                } else {
                    MATE_VALUE - 100   // Black is mated (very good for White)
                };
            } else {
                return 0;  // Stalemate is a draw
            }
        }
        return evaluate_position(game_state.board(), to_move);
    }
    // Time cutoff: return SEARCH_ABORTED to signal interruption
    if ctx.time_is_up() {
        return SEARCH_ABORTED;
    }
    // Draw checks in quiescence as well (only if Zobrist hashing is enabled)
    if crate::search::core::advanced_search::ZOBRIST_HASHING_ENABLED {
        // Use the incrementally maintained Zobrist key from GameState
        let key_here = game_state.zobrist_key();
        if rep_stack.contains(&key_here) {
            let count = rep_stack.count(key_here);
            let is_top = rep_stack.last().is_some_and(|k| k == key_here);
            // If current position is already on top (pushed by alpha-beta), only treat as repetition
            // when it appeared earlier in the line (count >= 2). Otherwise, it's a false positive.
            let is_repetition = count >= 2 || !is_top;
            #[cfg(feature = "debug-search")]
            if qdepth <= 2 {
                let indent = "    ".repeat(qdepth + 1);
                if is_repetition {
                    eprintln!("{}[QS] repetition detected -> return 0", indent);
                } else {
                    eprintln!("{}[QS] repetition on top (count=1) -> ignored", indent);
                }
            }
            if is_repetition {
                return 0;
            }
        }
    }
    if game_state.half_move_clock() >= 100 {
        #[cfg(feature = "debug-search")]
        if qdepth <= 2 {
            let indent = "    ".repeat(qdepth + 1);
            eprintln!("{}[QS] 50-move rule -> return 0", indent);
        }
        return 0;
    }

    // Stand-pat (static) evaluation. Suppress stand-pat only when in check.
    let in_check =
        game_state.mutable_board().is_side_in_check(to_move);
    let stand_pat = evaluate_position(game_state.board(), to_move);

    #[cfg(feature = "debug-search")] {
        if qdepth <= 2 {
            let indent = "    ".repeat(qdepth + 1);
            eprintln!("{}[QS] stand_pat={} in_check={}", indent, stand_pat, in_check);
        }
    }

    if !in_check {
        // Stand-pat cutoff: if current position is already good enough, return it
        if maximizing {
            if stand_pat >= beta {
                return stand_pat;
            }
            if stand_pat > alpha {
                alpha = stand_pat;
            }
        } else {
            if stand_pat <= alpha {
                return stand_pat;
            }
            if stand_pat < beta {
                beta = stand_pat;
            }
        }

        // Delta pruning: if we're so far behind that even the best capture can't help, give up early
        // This is checked before move generation to save work
        if maximizing {
            if stand_pat + DELTA_MARGIN <= alpha {
                return stand_pat;
            }
        } else if stand_pat - DELTA_MARGIN >= beta {
            return stand_pat;
        }
    }

    // Generate moves. If not in check, restrict to captures (quiescence).
    // NOTE: For speed we currently generate all and filter; consider adding
    // a dedicated capture generator to avoid the extra work.
    let mut moves: Vec<((usize, usize), (usize, usize), Option<char>)> = if in_check {
        find_all_evasion_moves(game_state)
    } else {
        find_all_capture_moves(game_state)
    };
    let ep_target = game_state.en_passant_target();
    if !in_check {
        if QSEE_PRUNING_ENABLED {
            // Keep captures only; additionally filter out clearly losing captures using SEE
            moves.retain(|&(from, to, promo)| {
                let moved = match game_state.board().get(from.0, from.1) {
                    Some(p) => p,
                    None => return false,
                };
                let is_ep = ep_target.is_some()
                    && to == ep_target.unwrap()
                    && moved.get_type() == PieceType::Pawn
                    && game_state.board().get(to.0, to.1).is_none();
                if game_state.board().get(to.0, to.1).is_none() && promo.is_none() && !is_ep {
                    return false;
                }
                // SEE pre-check: build post-move board and evaluate destination safety
                // NOTE: Still using a clone here for SEE, but SEE itself might be optimized later
                let mut post = *game_state.board();
                let captured = if is_ep {
                    let cap_sq = (from.0, to.1);
                    let cap = game_state.board().get(cap_sq.0, cap_sq.1);
                    post.set(cap_sq.0, cap_sq.1, None);
                    cap
                } else {
                    game_state.board().get(to.0, to.1)
                };
                post.set(from.0, from.1, None);

                let mut p_piece = moved;
                if let Some(pc) = promo {
                    let pt = match pc {
                        'q' => PieceType::Queen,
                        'r' => PieceType::Rook,
                        'b' => PieceType::Bishop,
                        'n' => PieceType::Knight,
                        _ => p_piece.get_type(),
                    };
                    p_piece = crate::piece::pieces::Piece::new(pt, p_piece.get_color());
                }
                post.set(to.0, to.1, Some(p_piece));

                let cap_val = captured.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
                let see = see_dest_estimate(&post, to_move, to, cap_val);
                see >= QSEE_CAPTURE_TOLERANCE
            });
        }
    }

    // Selective endgame pawn-push quiescence: allow a few safe passer pushes
    // to stabilize eval around promotion races. Tight gating to avoid explosion.
    if !in_check {
        let phase = game_state.board().game_phase_light();
        if phase <= 8 {
            let mut pieces_to_check = Vec::new();
            for r in 0..8 {
                for c in 0..8 {
                    if let Some(p) = game_state.board().get(r, c)
                        && p.get_color() == to_move && p.get_type() == PieceType::Pawn {
                            pieces_to_check.push((r, c));
                        }
                }
            }

            // collect up to N safe quiet pushes
            let mut added: usize = 0;
            'outer: for (r, c) in pieces_to_check {
                // only consider passed pawns on 5th–7th ranks (relative to side)
                let adv: i32 = match to_move {
                    Color::White => r as i32,
                    Color::Black => (7 - r) as i32,
                };
                if adv < 4 {
                    continue;
                }
                if !game_state.board().is_passed_pawn_simple(r, c, to_move) {
                    continue;
                }
                // one-step push target
                let (nr_opt, to_sq) = match to_move {
                    Color::White => (
                        if r < 7 { Some(r + 1) } else { None },
                        (r.saturating_add(1), c),
                    ),
                    Color::Black => (
                        if r > 0 { Some(r - 1) } else { None },
                        (r.saturating_sub(1), c),
                    ),
                };
                let nr = if let Some(nr) = nr_opt {
                    nr
                } else {
                    continue;
                };
                if game_state.board().get(nr, c).is_some() {
                    continue;
                }
                // simulate and verify safety and legality
                let from = (r, c);
                let to = to_sq;
                let u = game_state.make_move_fast(from, to, None);
                // move must not leave own king in check
                let illegal = game_state.mutable_board().is_side_in_check(to_move);
                // target square should not be immediately attacked by opponent
                use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
                let attacked = is_square_attacked_by_opponent(&mut *game_state.mutable_board(), to, to_move);
                game_state.unmake_move_fast(u);
                if illegal || attacked {
                    continue;
                }
                // Futility pruning for quiet pawn pushes: skip if cannot improve position
                if QSEE_PRUNING_ENABLED {
                    // Estimate pawn push value at ~100cp (depends on advancement)
                    let push_bonus = 100;
                    if maximizing {
                        if stand_pat + push_bonus + FUT_MARGIN <= alpha {
                            continue;
                        }
                    } else if stand_pat - push_bonus - FUT_MARGIN >= beta {
                        continue;
                    }
                }
                moves.push((from, to, None));
                added += 1;
                if added >= MAX_QUIET_PUSHES {
                    break 'outer;
                }
            }
        }
    }

    if moves.is_empty() {
        // If in check with no legal moves, it's checkmate
        // Use a large offset (100) since we don't track exact ply in qsearch
        if in_check {
            return if maximizing {
                -MATE_VALUE + 100  // White is mated (very bad for White)
            } else {
                MATE_VALUE - 100   // Black is mated (very good for White)
            };
        }
        return stand_pat;
    }

    // Order moves by MVV-LVA to improve cutoffs
    if MVV_LVA_ENABLED {
        moves.sort_by_key(|&(from, to, promo)| {
            let mut score = game_state.board().move_score_mvv_lva(from, to);
            if let Some(p) = promo {
                score += match p {
                    'q' => 900,
                    'r' => 500,
                    'b' => 330,
                    'n' => 320,
                    _ => 0,
                };
            }
            -score
        });
    }

    // Futility pruning in move loop: skip captures that cannot improve the position
    // Note: SEE-based bad capture filtering is already done during move generation

    let mut best = if maximizing { MIN_EVAL_VALUE } else { MAX_EVAL_VALUE };

    for (from, to, promo) in moves.into_iter() {
        // Futility in qsearch: if even taking the victim cannot improve our position, skip
        if !in_check && QSEE_PRUNING_ENABLED && promo.is_none() {
            let is_ep = ep_target.is_some()
                && to == ep_target.unwrap()
                && game_state.board().get(to.0, to.1).is_none()
                && matches!(game_state.board().get(from.0, from.1), Some(p) if p.get_type() == PieceType::Pawn);
            let vic_v = if is_ep {
                piece_value_cp(PieceType::Pawn)
            } else if let Some(vic) = game_state.board().get(to.0, to.1) {
                piece_value_cp(vic.get_type())
            } else {
                0
            };
            if vic_v > 0 {
                if maximizing {
                    if stand_pat + vic_v + FUT_MARGIN <= alpha {
                        continue;
                    }
                } else if stand_pat - vic_v - FUT_MARGIN >= beta {
                    continue;
                }
            }
        }

        let u = game_state.make_move_fast(from, to, promo);
        QSEARCH_DEPTH.with(|d| d.set(d.get() + 1));
        let score = qsearch_with_quiescence(ctx, game_state, alpha, beta, rep_stack, quiescence_enabled);
        QSEARCH_DEPTH.with(|d| d.set(d.get() - 1));
        game_state.unmake_move_fast(u);
        if score == SEARCH_ABORTED {
            return SEARCH_ABORTED;
        }

        #[cfg(feature = "debug-search")] {
            if qdepth <= 2 {
                let indent = "    ".repeat(qdepth + 1);
                let from_sq = crate::piece::as_square_str(from);
                let to_sq = crate::piece::as_square_str(to);
                let promo_str = promo.map(|c| c.to_string()).unwrap_or_default();
                eprintln!("{}  qmove {}{}{}: score={}", indent, from_sq, to_sq, promo_str, score);
            }
        }

        // Update best value
        if maximizing {
            if score > best {
                best = score;
            }
            if best > alpha {
                alpha = best;
            }
        } else {
            if score < best {
                best = score;
            }
            if best < beta {
                beta = best;
            }
        }

        // Alpha-beta cutoff
        if alpha >= beta {
            #[cfg(feature = "debug-search")] {
                if qdepth <= 2 {
                    let indent = "    ".repeat(qdepth + 1);
                    eprintln!("{}[QS] cutoff! α={} ≥ β={}", indent, alpha, beta);
                }
            }
            break;
        }
    }

    #[cfg(feature = "debug-search")] {
        if  qdepth <= 2 {
            let indent = "    ".repeat(qdepth + 1);
            eprintln!("{}[QS] returning best={}", indent, best);
        }
    }

    best
}
