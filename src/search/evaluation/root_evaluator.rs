use crate::board::Board;
use crate::history::history::History;
use crate::piece::pieces::{Color, PieceType};
use crate::search::evaluation::repetition::apply_repetition_avoidance_bias;
use crate::search::management::root_moves::{adjusted_root_eval_for_move, evaluate_after_root_move};
use crate::search::state::tt::{decode_move, TranspositionTable};
use crate::search::state::zobrist::compute_zobrist_full;
use crate::state::castling::CastlingRights;
use crate::search::{is_parallel_search};
use crate::state::game_state::GameState;
use rayon::prelude::*;
pub(crate) use crate::board::evaluator::{MAX_EVAL_VALUE, MIN_EVAL_VALUE};
use crate::search::core::advanced_search::SEARCH_ABORTED;

// Root parallelization settings
const ROOT_PARALLEL_MIN_DEPTH: usize = 6;
const ROOT_PARALLEL_MIN_MOVES: usize = 4;

#[inline]
pub(crate) fn reorder_with_tt_hint(
    ordered: &mut Vec<((usize, usize), (usize, usize), Option<char>)>,
    tt: &TranspositionTable,
    board: &Board,
    side: Color,
    castling: &CastlingRights,
    ep_target: Option<(usize, usize)>,
) {
    if let Some(entry) = tt.probe(compute_zobrist_full(board, side, castling, ep_target)) {
        let bm = decode_move(entry.best_from, entry.best_to);
        if let Some(pos) = ordered.iter().position(|&(f, t, _)| (f, t) == bm) {
            let first = ordered.remove(pos);
            ordered.insert(0, first);
        }
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_root_for_bounds(
    active_color: Color,
    root_moves: &Vec<((usize, usize), (usize, usize), Option<char>)>,
    depth_now: usize,
    a: i32,
    b: i32,
    tt: &mut TranspositionTable,
    game_state: &mut GameState,
    history: &History,
    mut collect_all_scores: Option<&mut Vec<(((usize, usize), (usize, usize), Option<char>), i32, i32)>>,
) -> (((usize, usize), (usize, usize), Option<char>), i32, i32) {
    let mut best_from_to_promo: Option<((usize, usize), (usize, usize), Option<char>)> = None;
    // Adjusted scores are in White-perspective:
    // White maximizes (start from MIN), Black minimizes (start from MAX)
    let mut best_score_raw = MIN_EVAL_VALUE;
    let mut best_adjusted = if active_color == Color::White {
        MIN_EVAL_VALUE
    } else {
        MAX_EVAL_VALUE
    };

    // Order: if TT has a move at root, try to place it first, then apply light opening-aware tie-breakers
    let mut ordered: Vec<((usize, usize), (usize, usize), Option<char>)> = root_moves.to_vec();
    reorder_with_tt_hint(
        &mut ordered,
        tt,
        game_state.board(),
        active_color,
        &game_state.castling_rights(),
        game_state.en_passant_target(),
    );

    // Opening-aware tiny reordering: demote quiet queen moves; promote minor development and castling
    // Keep this extremely small so as not to override tactical ordering.
    // Apply only at very shallow plies (root) and more in opening phase.
    let phase_for_scale = {
        // Phase proxy: count heavy/minors on board similar to evaluator's game_phase; fallback small constant
        let mut phase = 0i32;
        let b = game_state.board();
        for r in 0..8 { for c in 0..8 { if let Some(p)=b.get(r,c) {
            phase += match p.get_type() { PieceType::Knight|PieceType::Bishop => 1, PieceType::Rook => 2, PieceType::Queen => 4, _ => 0 };
        }}}
        if phase < 0 { 0 } else if phase > 24 { 24 } else { phase }
    };
    if depth_now >= 1 {
        let b = game_state.board();
        ordered.sort_by(|&(f1,t1, _), &(f2,t2, _)| {
            let score1 = root_move_order_bias(b, active_color, f1, t1, phase_for_scale);
            let score2 = root_move_order_bias(b, active_color, f2, t2, phase_for_scale);
            score2.cmp(&score1) // higher bias first
        });
    }

    let enable_parallel = is_parallel_search()
        && depth_now >= ROOT_PARALLEL_MIN_DEPTH
        && ordered.len() >= ROOT_PARALLEL_MIN_MOVES;
    if enable_parallel {
        // 1) Search the first (best-ordered) move serially to establish PV and bounds
        let &(pv_from, pv_to, pv_promo) = ordered.first().unwrap();
        let pv_adjusted;
        let pv_score_raw;
        {
            let (score_raw, is_capture, moved_is_pawn) = evaluate_after_root_move(
                game_state,
                pv_from,
                pv_to,
                pv_promo,
                depth_now,
                a,
                b,
                tt,
                history,
            );

            if score_raw == SEARCH_ABORTED {
                return (((0, 0), (0, 0), None), SEARCH_ABORTED, SEARCH_ABORTED);
            }

            let adjusted = adjusted_root_eval_for_move(
                game_state.board(),
                active_color,
                pv_from,
                pv_to,
                game_state.half_move_clock(),
                score_raw,
                is_capture,
                moved_is_pawn,
            );

            best_from_to_promo = Some((pv_from, pv_to, pv_promo));
            best_adjusted = adjusted;
            best_score_raw = score_raw;
            pv_adjusted = adjusted;
            pv_score_raw = score_raw;
        }

        // Collect PV move score if requested
        if let Some(collector) = collect_all_scores.as_deref_mut() {
            collector.push(((pv_from, pv_to, pv_promo), pv_adjusted, pv_score_raw));
        }

        // 2) Search the remaining moves in parallel with per-task local TT to avoid contention
        // reuse shared board reference in parallel (read-only access)
        let a_loc = a;
        let b_loc = b;
        let side = active_color;
        let parallel_results: Vec<_> = ordered[1..]
            .par_iter()
            .map(|&(from, to, promo)| {
                // local TT per task
                let mut local_tt = TranspositionTable::new_with_default_size();
                // WE MUST CLONE game_state for parallel tasks
                let mut local_gs = *game_state;
                let (score_raw, is_capture, moved_is_pawn) = evaluate_after_root_move(
                    &mut local_gs,
                    from,
                    to,
                    promo,
                    depth_now,
                    a_loc,
                    b_loc,
                    &mut local_tt,
                    history,
                );

                if score_raw == SEARCH_ABORTED {
                    return (from, to, promo, SEARCH_ABORTED, SEARCH_ABORTED);
                }

                // Root adjustments
                let mut adjusted = adjusted_root_eval_for_move(
                    local_gs.board(),
                    side,
                    from,
                    to,
                    local_gs.half_move_clock(),
                    score_raw,
                    is_capture,
                    moved_is_pawn,
                );
                // Apply repetition-avoidance bias at root for parallel moves
                adjusted = apply_repetition_avoidance_bias(
                    adjusted,
                    &local_gs,
                    history,
                    side,
                    from,
                    to,
                    promo,
                    score_raw,
                );
                (from, to, promo, adjusted, score_raw)
            })
            .collect();

        // Collect all parallel move scores if requested
        if let Some(collector) = collect_all_scores.as_deref_mut() {
            for &(from, to, promo, adj, raw) in &parallel_results {
                if raw != SEARCH_ABORTED {
                    collector.push(((from, to, promo), adj, raw));
                }
            }
        }

        // Find best move from parallel results
        // Adjustments are in White-perspective: White maximizes, Black minimizes
        let results = parallel_results.into_iter()
            .reduce(|acc, x| {
                if x.4 == SEARCH_ABORTED {
                    return x;
                }
                if acc.4 == SEARCH_ABORTED {
                    return acc;
                }
                let better = if side == Color::White {
                    x.3 > acc.3  // White wants higher scores
                } else {
                    x.3 < acc.3  // Black wants lower scores
                };
                if better { x } else { acc }
            })
            .unwrap_or((
                (0usize, 0usize),
                (0usize, 0usize),
                None,
                MIN_EVAL_VALUE,
                MIN_EVAL_VALUE,
            ));

        // Update best with parallel results if better
        let (pf, pt, ppromo, padj, praw) = results;
        if praw == SEARCH_ABORTED {
            return (((0, 0), (0, 0), None), SEARCH_ABORTED, SEARCH_ABORTED);
        }
        // Ignore identity placeholder
        if !(pf == (0, 0) && pt == (0, 0)) {
            let better = if side == Color::White {
                padj > best_adjusted  // White wants higher scores
            } else {
                padj < best_adjusted  // Black wants lower scores
            };
            if better {
                best_from_to_promo = Some((pf, pt, ppromo));
                best_adjusted = padj;
                best_score_raw = praw;
            }
        }
    } else {
        // Search sequentially over root moves
        for &(from, to, promo) in &ordered {
            let (score_raw, is_capture, moved_is_pawn) = evaluate_after_root_move(
                game_state,
                from,
                to,
                promo,
                depth_now,
                a,
                b,
                tt,
                history,
            );

            if score_raw == SEARCH_ABORTED {
                return (((0, 0), (0, 0), None), SEARCH_ABORTED, SEARCH_ABORTED);
            }

            // Adjust score for root-only heuristics
            let mut adjusted = adjusted_root_eval_for_move(
                game_state.board(),
                active_color,
                from,
                to,
                game_state.half_move_clock(),
                score_raw,
                is_capture,
                moved_is_pawn,
            );
            // repetition-avoidance at root
            adjusted = apply_repetition_avoidance_bias(
                adjusted,
                game_state,
                history,
                active_color,
                from,
                to,
                promo,
                score_raw,
            );

            // Collect move score if requested
            if let Some(collector) = collect_all_scores.as_deref_mut() {
                collector.push(((from, to, promo), adjusted, score_raw));
            }

            // Track best
            // Adjustments are in White-perspective: White maximizes, Black minimizes
            let better = if active_color == Color::White {
                adjusted > best_adjusted  // White wants higher scores
            } else {
                adjusted < best_adjusted  // Black wants lower scores
            };

            if better || best_from_to_promo.is_none() {
                best_from_to_promo = Some((from, to, promo));
                best_adjusted = adjusted;
                best_score_raw = score_raw;
            }

            // Aspiration cutoffs help ordering mid-loop too
            if active_color == Color::White && score_raw >= b {
                break;
            }
            if active_color == Color::Black && score_raw <= a {
                break;
            }
        }
    }

    (best_from_to_promo.unwrap(), best_adjusted, best_score_raw)
}

// Tiny heuristic score for root ordering; positive favors earlier search
#[inline]
pub(crate) fn root_move_order_bias(board: &Board, side: Color, from: (usize, usize), to: (usize, usize), phase: i32) -> i32 {
    // scale 0..24 -> 0..24
    let scale = phase.clamp(0,24);
    let mut bias: i32 = 0;
    // Identify moved piece
    let piece = match board.get(from.0, from.1) { Some(p) if p.get_color()==side => p, _ => return 0 };
    let is_capture = board.get(to.0, to.1).is_some();

    // Prefer castling
    if piece.get_type()==PieceType::King {
        let dr = if side==Color::White { 0usize } else { 7usize };
        if from.0==dr && (to.1==6 || to.1==2) { bias += (10 * scale) / 24; }
    }

    // Prefer minor development from back rank
    if piece.get_type()==PieceType::Knight || piece.get_type()==PieceType::Bishop {
        let back = if side==Color::White { 0usize } else { 7usize };
        if from.0 == back {
            bias += (8 * scale) / 24; // slightly stronger nudge
        }
    }

    // Demote quiet queen moves in opening at root (unless capture)
    if piece.get_type()==PieceType::Queen && !is_capture {
        // Light demotion; guards: if move gives check we'll discover tactically later
        let mut demote = 9; // base demotion strength
        // Extra demotion if position is underdeveloped (>=2 minors on back rank)
        let back_r = if side==Color::White { 0usize } else { 7usize };
        let mut undeveloped = 0;
        for fc in 0..8 {
            if let Some(p) = board.get(back_r, fc)
                && p.get_color()==side
                    && matches!(p.get_type(), PieceType::Knight | PieceType::Bishop) { undeveloped += 1; }
        }
        if undeveloped >= 3 { demote += 6; } else if undeveloped >= 2 { demote += 3; }
        // Extra demotion for big queen sorties (long leaps) in the opening
        let manhattan = (from.0 as i32 - to.0 as i32).abs() + (from.1 as i32 - to.1 as i32).abs();
        if manhattan >= 3 { demote += 4; }
        // Extra demotion for advancing deep (enemy side) without capture
        let deep_adv = match side { Color::White => to.0 >= 3, Color::Black => to.0 <= 4 };
        if deep_adv { demote += 3; }
        bias -= (demote * scale) / 24;
    }
    bias
}
