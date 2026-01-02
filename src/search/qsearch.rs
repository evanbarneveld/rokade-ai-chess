use crate::board::Board;
use crate::board::evaluator::evaluate_position;
use crate::piece::pieces::{piece_value_cp, Color, PieceType};
use crate::search::search::{find_all_valid_moves, MAX_EVAL_VALUE, MIN_EVAL_VALUE, QUIESCENCE_ENABLED, QSEE_PRUNING_ENABLED, MVV_LVA_ENABLED};
use crate::state::game_state::GameState;
use crate::search::see::see_dest_estimate;
use crate::search::time_control::time_is_up;
use crate::search::zobrist::compute_zobrist;

// Tighter margins with a stronger static evaluator
const FUT_MARGIN: i32 = 40;
const DELTA_MARGIN: i32 = 120; // centipawns
const MAX_QUIET_PUSHES: usize = 2;

// Quiescence search: consider only tactical continuations (captures) unless in check.
pub fn qsearch(
    board: &mut Board,
    to_move: Color,
    mut alpha: i32,
    beta: i32,
    halfmove_clock: u32,
    rep_stack: &mut Vec<u64>,
) -> i32 {
    // If quiescence is disabled, return a static evaluation immediately.
    if !QUIESCENCE_ENABLED {
        return evaluate_position(&*board, to_move);
    }
    // Time cutoff in quiescence as well: return a quick static eval
    if time_is_up() {
        return evaluate_position(&*board, to_move);
    }
    // Draw checks in quiescence as well (only if Zobrist hashing is enabled)
    if crate::search::search::ZOBRIST_HASHING_ENABLED {
        let key_here = compute_zobrist(&*board, to_move);
        if rep_stack.iter().any(|&k| k == key_here) {
            return 0;
        }
    }
    if halfmove_clock >= 100 {
        return 0;
    }

    // Stand-pat (static) evaluation. Suppress stand-pat only when in check.
    let in_check =
        board.is_side_in_check(to_move);
    let stand_pat = evaluate_position(&*board, to_move);
    if !in_check {
        // Uniform alpha/beta semantics regardless of side-to-move.
        if stand_pat >= beta {
            return stand_pat;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }
    }

    // Generate moves. If not in check, restrict to captures (quiescence).
    // NOTE: For speed we currently generate all and filter; consider adding
    // a dedicated capture generator to avoid the extra work.
    let gs = GameState::from_board_and_side((*board).clone(), to_move);
    let mut moves: Vec<((usize, usize), (usize, usize))> = find_all_valid_moves(&gs)
        .iter()
        .map(|(f, t, _)| (*f, *t))
        .collect();
    if !in_check {
        if QSEE_PRUNING_ENABLED {
            // Keep captures only; additionally filter out clearly losing captures using SEE
            moves.retain(|&(from, to)| {
                if board.get(to.0, to.1).is_none() {
                    return false;
                }
                // SEE pre-check: build post-move board and evaluate destination safety
                let mut post = board.clone();
                let moved = match board.get(from.0, from.1) {
                    Some(p) => p,
                    None => return false,
                };
                let captured = board.get(to.0, to.1);
                post.set(from.0, from.1, None);
                post.set(to.0, to.1, Some(moved));
                let cap_val = captured.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
                let see = see_dest_estimate(&post, to_move, to, cap_val);
                see >= -50 // allow slightly negative to avoid over-pruning, but skip clearly losing captures
            });
        } else {
            // Keep captures only, no SEE-based filtering
            moves.retain(|&(_from, to)| board.get(to.0, to.1).is_some());
        }
    }

    // Selective endgame pawn-push quiescence: allow a few safe passer pushes
    // to stabilize eval around promotion races. Tight gating to avoid explosion.
    if !in_check {
        let phase = board.game_phase_light();
        if phase <= 8 {
            // collect up to N safe quiet pushes
            let mut added: usize = 0;
            'outer: for r in 0..8 {
                for c in 0..8 {
                    if let Some(p) = board.get(r, c) {
                        if p.get_color() != to_move || p.get_type() != PieceType::Pawn {
                            continue;
                        }
                        // only consider passed pawns on 5th–7th ranks (relative to side)
                        let adv: i32 = match to_move {
                            Color::White => r as i32,
                            Color::Black => (7 - r) as i32,
                        };
                        if adv < 4 {
                            continue;
                        }
                        if !board.is_passed_pawn_simple(r, c, to_move) {
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
                        if board.get(nr, c).is_some() {
                            continue;
                        }
                        // simulate and verify safety and legality
                        let from = (r, c);
                        let to = to_sq;
                        let u = board.make_move_simple(from, to);
                        // move must not leave own king in check
                        let illegal = board.is_side_in_check(to_move);
                        // target square should not be immediately attacked by opponent
                        use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
                        let attacked = is_square_attacked_by_opponent(board, to, to_move);
                        board.unmake_move_simple(u);
                        if illegal || attacked {
                            continue;
                        }
                        moves.push((from, to));
                        added += 1;
                        if added >= MAX_QUIET_PUSHES {
                            break 'outer;
                        }
                    }
                }
            }
        }
    }

    // Delta pruning: if not in check and clearly below alpha, prune.
    // Start with a conservative constant margin; tune empirically.
    if !in_check && stand_pat + DELTA_MARGIN <= alpha {
        return stand_pat;
    }

    if moves.is_empty() {
        return stand_pat;
    }

    // Order captures by MVV-LVA to improve cutoffs (only matters for captures branch)
    if !in_check && MVV_LVA_ENABLED {
        let b = &*board;
        moves.sort_by_key(|&(from, to)| -b.move_score_mvv_lva(from, to));
    }

    // Simple capture SEE-like filter: skip obviously losing captures when not in check.
    // Uses basic piece values; this is a cheap approximation, not true SEE.

    let mut a = alpha;
    let mut bnd = beta;
    if to_move == Color::White {
        let mut best = MIN_EVAL_VALUE;
        for (from, to) in moves.into_iter() {
            if !in_check && QSEE_PRUNING_ENABLED {
                if let (Some(att), Some(vic)) = (board.get(from.0, from.1), board.get(to.0, to.1)) {
                    let att_v = piece_value_cp(att.get_type());
                    let vic_v = piece_value_cp(vic.get_type());
                    // Skip "bad" captures where attacker is significantly more valuable than victim
                    if vic_v + 50 < att_v {
                        continue;
                    }
                    // Futility in qsearch (White to move): if even taking the victim cannot raise alpha, skip
                    if stand_pat + vic_v + FUT_MARGIN <= a {
                        continue;
                    }
                }
            }
            let was_capture = board.get(to.0, to.1).is_some();
            let u = board.make_move_simple(from, to);
            let mut child_hmc = halfmove_clock + 1;
            if was_capture {
                child_hmc = 0;
            }
            let score = qsearch(board, Color::Black, a, bnd, child_hmc, rep_stack);
            board.unmake_move_simple(u);
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
        for (from, to) in moves.into_iter() {
            if !in_check && QSEE_PRUNING_ENABLED {
                if let (Some(att), Some(vic)) = (board.get(from.0, from.1), board.get(to.0, to.1)) {
                    let att_v = piece_value_cp(att.get_type());
                    let vic_v = piece_value_cp(vic.get_type());
                    if vic_v + 50 < att_v {
                        continue;
                    }
                    // Futility in qsearch (Black to move): if even taking the victim cannot drop below beta, skip
                    if stand_pat - vic_v - FUT_MARGIN >= bnd {
                        continue;
                    }
                }
            }
            let was_capture = board.get(to.0, to.1).is_some();
            let u = board.make_move_simple(from, to);
            let mut child_hmc = halfmove_clock + 1;
            if was_capture {
                child_hmc = 0;
            }
            let score = qsearch(board, Color::White, a, bnd, child_hmc, rep_stack);
            board.unmake_move_simple(u);
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
