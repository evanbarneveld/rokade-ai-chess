use crate::board::evaluator::evaluate_position;
use crate::piece::pieces::{piece_value_cp, Color, PieceType};
use crate::search::core::advanced_search::{find_all_valid_moves, MAX_EVAL_VALUE, MIN_EVAL_VALUE, QUIESCENCE_ENABLED, QSEE_PRUNING_ENABLED, MVV_LVA_ENABLED, SEARCH_ABORTED};
use crate::state::game_state::GameState;
use crate::search::management::see::{see_dest_estimate, QSEE_CAPTURE_TOLERANCE};
use crate::search::integration::time_control::time_is_up;
// Note: Zobrist key is now maintained incrementally in GameState
use crate::search::state::rep_stack::RepetitionStack;
use crate::search::state::tt::MATE_VALUE;

// Tighter margins with a stronger static evaluator
const FUT_MARGIN: i32 = 80;
const DELTA_MARGIN: i32 = 925; // Queen value (900) + pawn promotion buffer (25)
const MAX_QUIET_PUSHES: usize = 2;

// Quiescence Search: consider only tactical continuations (captures) unless in check.
pub fn qsearch(
    game_state: &mut GameState,
    mut alpha: i32,
    mut beta: i32,
    rep_stack: &mut RepetitionStack,
) -> i32 {
    let to_move = game_state.active_color();
    // If quiescence is disabled, return a static evaluation immediately.
    if !QUIESCENCE_ENABLED {
        return evaluate_position(game_state.board(), to_move);
    }
    // Time cutoff: return SEARCH_ABORTED to signal interruption
    if time_is_up() {
        return SEARCH_ABORTED;
    }
    // Draw checks in quiescence as well (only if Zobrist hashing is enabled)
    if crate::search::core::advanced_search::ZOBRIST_HASHING_ENABLED {
        // Use the incrementally maintained Zobrist key from GameState
        let key_here = game_state.zobrist_key();
        if rep_stack.contains(&key_here) {
            return 0;
        }
    }
    if game_state.half_move_clock() >= 100 {
        return 0;
    }

    // Stand-pat (static) evaluation. Suppress stand-pat only when in check.
    let in_check =
        game_state.mutable_board().is_side_in_check(to_move);
    let stand_pat = evaluate_position(game_state.board(), to_move);
    if !in_check {
        if to_move == Color::White {
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
        if to_move == Color::White {
            // White: too far below alpha, even capturing a queen + promotion won't help
            if stand_pat + DELTA_MARGIN <= alpha {
                return stand_pat;
            }
        } else {
            // Black: too far above beta (bad for Black), even capturing a queen + promotion won't help
            if stand_pat - DELTA_MARGIN >= beta {
                return stand_pat;
            }
        }
    }

    // Generate moves. If not in check, restrict to captures (quiescence).
    // NOTE: For speed we currently generate all and filter; consider adding
    // a dedicated capture generator to avoid the extra work.
    let mut moves: Vec<((usize, usize), (usize, usize), Option<char>)> = find_all_valid_moves(game_state);
    if !in_check {
        if QSEE_PRUNING_ENABLED {
            // Keep captures only; additionally filter out clearly losing captures using SEE
            moves.retain(|&(from, to, promo)| {
                if game_state.board().get(to.0, to.1).is_none() && promo.is_none() {
                    return false;
                }
                // SEE pre-check: build post-move board and evaluate destination safety
                // NOTE: Still using a clone here for SEE, but SEE itself might be optimized later
                let mut post = *game_state.board();
                let moved = match game_state.board().get(from.0, from.1) {
                    Some(p) => p,
                    None => return false,
                };
                let captured = game_state.board().get(to.0, to.1);
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
        } else {
            // Keep captures or promotions only, no SEE-based filtering
            moves.retain(|&(_from, to, promo)| game_state.board().get(to.0, to.1).is_some() || promo.is_some());
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
                    if to_move == Color::White {
                        if stand_pat + push_bonus + FUT_MARGIN <= alpha {
                            continue;
                        }
                    } else {
                        if stand_pat - push_bonus - FUT_MARGIN >= beta {
                            continue;
                        }
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
            return if to_move == Color::White {
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

    let mut a = alpha;
    let mut bnd = beta;
    if to_move == Color::White {
        let mut best = MIN_EVAL_VALUE;
        for (from, to, promo) in moves.into_iter() {
            // Futility in qsearch (White to move): if even taking the victim cannot raise alpha, skip
            if !in_check && QSEE_PRUNING_ENABLED && promo.is_none() {
                if let Some(vic) = game_state.board().get(to.0, to.1) {
                    let vic_v = piece_value_cp(vic.get_type());
                    if stand_pat + vic_v + FUT_MARGIN <= a {
                        continue;
                    }
                }
            }
            let u = game_state.make_move_fast(from, to, promo);
            let score = qsearch(game_state, a, bnd, rep_stack);
            game_state.unmake_move_fast(u);
            if score > best {
                best = score;
            }
            if best > a {
                a = best;
            }
            if a >= bnd {
                break;
            }
        }
        best
    } else {
        let mut best = MAX_EVAL_VALUE;
        for (from, to, promo) in moves.into_iter() {
            // Futility in qsearch (Black to move): if even taking the victim cannot drop below beta, skip
            if !in_check && QSEE_PRUNING_ENABLED && promo.is_none() {
                if let Some(vic) = game_state.board().get(to.0, to.1) {
                    let vic_v = piece_value_cp(vic.get_type());
                    if stand_pat - vic_v - FUT_MARGIN >= bnd {
                        continue;
                    }
                }
            }
            let u = game_state.make_move_fast(from, to, promo);
            let score = qsearch(game_state, a, bnd, rep_stack);
            game_state.unmake_move_fast(u);
            if score < best {
                best = score;
            }
            if best < bnd {
                bnd = best;
            }
            if a >= bnd {
                break;
            }
        }
        best
    }
}
