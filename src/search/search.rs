use crate::board::Board;
use crate::history::history::History;
use crate::piece::move_validators::is_piece_move_valid;
use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{Color, Piece, PieceType};
use crate::search::locking::get_tt_mutex;
use crate::search::playing_strength::{select_move_based_using_strength, PLAYING_STRENGTH_MAX};
use crate::search::root_moves::{
    adjusted_root_eval_for_move, build_pv_for_root, evaluate_after_root_move, get_root_moves,
    hard_root_filter,
};
use crate::search::threading::init_rayon_pool_if_needed;
use crate::search::tt::{decode_move, TranspositionTable};
use crate::search::uci_feedback::emit_info;
use crate::search::zobrist::compute_zobrist;
use crate::state::fen::writer::game_state_to_fen_string;
use crate::state::game_state::GameState;
use rayon::prelude::*;
use crate::book::book::book_pick;

pub const MIN_EVAL_VALUE: i32 = i32::MIN + 100_000;
pub const MAX_EVAL_VALUE: i32 = i32::MAX - 100_000;

pub const DEFAULT_SEARCH_DEPTH: usize = 15;

// Iterative deepening aspiration window (in centipawns)
const ASP_WINDOW_INIT_CP: i32 = 50; // initial aspiration half-window
const ASP_WINDOW_MAX_CP: i32 = 800; // maximum expanded half-window

// Root parallelization thresholds
const ROOT_PARALLEL_MIN_DEPTH: usize = 6; // enable root parallel only from this depth
const ROOT_PARALLEL_MIN_MOVES: usize = 4; // and when at least this many root moves exist

// Root repetition-avoidance bias when a move would immediately create 3-fold
const REP_AVOIDANCE_BIAS_CP: i32 = 50_000;
pub const MAX_PLAYING_STRENGTH: usize = 1000;
pub const DEFAULT_MOVE_TIME_FOR_STRENGTH_MODE_PLAY: usize = 3000usize;

/// Find the best move for the given game state, the search_depth, and the playing_strength
/// returns the evaluated score (in centipawns) for the selected move
/// and the effective search depth that was actually used internally.
pub fn find_best_move(
    game_state: &GameState,
    history: &History,
    search_depth: usize,
    playing_strength: usize,
) -> Option<((usize, usize), (usize, usize), i32, usize)> {
    init_rayon_pool_if_needed();

    // Persistent Transposition Table across searches: initialize once and reuse.
    // We keep it behind a Mutex to allow mutable access in this serial root search.

    let tt_mutex = get_tt_mutex();

    // collect all legal moves for the side to move
    let board = game_state.board();
    let active_color = game_state.active_color();
    let moves = find_all_valid_moves(board, active_color);

    if moves.is_empty() {
        return None;
    }

    // Opening book: if we have a book move in early game, play it immediately.
    // Limit to first ~8 full moves to avoid forcing book deep into middlegame.
    if game_state.full_move_number() <= 8 {
        if let Some((bf, bt)) = book_pick(game_state) {
            return Some((bf, bt, 0, 0));
        }
    }

    // if depth is 0, treat it as 1 ply (evaluate after making one move)
    let search_depth = if search_depth == 0 { 1 } else { search_depth };

    // Map playing_strength [1..1000] to an effective depth to intentionally weaken play at low strengths.
    // Rough mapping: at ~300 strength, cap to ~3 ply; at 1000 keep requested depth.
    let ps = if playing_strength == 0 {
        1
    } else {
        playing_strength.min(PLAYING_STRENGTH_MAX)
    } as i32;

    let effective_depth = search_depth;

    // Root-level hard 3-fold avoidance: filter out any root move that would create
    // a third occurrence of the same position (per truncated FEN used in History).
    // If filtering removes all moves (e.g., only repetition saves a loss), fall back to all moves.
    let root_moves: Vec<((usize, usize), (usize, usize))> = {
        let mut v = Vec::with_capacity(moves.len());

        get_root_moves(game_state, history, board, active_color, &moves, &mut v);

        // Hard root filter: drop unsafe queen moves (SEE<0) and unsafe minor-piece non-check sacs
        // (SEE<=SEE_MINOR_SAC_THRESHOLD_CP and not giving check)
        // If filtering removes all, keep original set.
        if !v.is_empty() {
            let mut filtered: Vec<((usize, usize), (usize, usize))> = Vec::with_capacity(v.len());
            hard_root_filter(board, active_color, &mut v, &mut filtered);
            if filtered.is_empty() { v } else { filtered }
        } else {
            v
        }
    };

    // Iterative Deepening + Aspiration windows at root (serial evaluation for stability)
    // Reuse persistent TT
    let mut tt = tt_mutex.lock().unwrap();
    let base_hmc = game_state.half_move_clock();
    let mut last_score: i32 = 0;
    let mut chosen: Option<((usize, usize), (usize, usize), i32, usize)> = None;
    let mut window: i32 = ASP_WINDOW_INIT_CP; // cp

    //eprintln!("[root] starting ID; eff_depth={} root_moves={} window={}",
    //          effective_depth, root_moves.len(), window);

    for depth_now in 1..=effective_depth {

        //eprintln!("[root] depth_now={} (pre-asp) last_score={} window={}", depth_now, last_score, window);

        tt.next_age();
        let ((bf, bt), best_adj, best_raw) = probe_with_aspiration(
            &board,
            active_color,
            &root_moves,
            depth_now,
            last_score,
            &mut window,
            &mut tt,
            base_hmc,
            ps,
            game_state,
            history,
        );

        //eprintln!("[root] depth_now={} (post-asp) best_adj={} best_raw={} mv={:?}->{:?}",
        //          depth_now, best_adj, best_raw, (bf, bt).0, (bf, bt).1);

        last_score = best_raw;
        // Emit PV/info for this iteration, including TT hashfull permille
        let pv = build_pv_for_root(board, active_color, bf, bt, &tt, depth_now);
        let hf = tt.hashfull_permille();
        emit_info(bf, bt, best_adj, depth_now, pv, hf);
        chosen = Some((bf, bt, best_adj, depth_now));
    }

    // Final selection based on playing_strength from the last iteration
    if let Some((bf, bt, sc, used_depth)) = chosen {
        if playing_strength >= MAX_PLAYING_STRENGTH {
            Some((bf, bt, sc, used_depth))
        } else {
            // Re-evaluate top K moves for stochastic selection at final depth
            let mut scored: Vec<((usize, usize), (usize, usize), i32)> = Vec::new();
            for &(from, to) in &root_moves {
                let (sr, is_capture, moved_is_pawn) = evaluate_after_root_move(
                    &board,
                    active_color,
                    from,
                    to,
                    effective_depth,
                    MIN_EVAL_VALUE + 1,
                    MAX_EVAL_VALUE - 1,
                    &mut tt,
                    base_hmc,
                );
                let adj = adjusted_root_eval_for_move(
                    &board,
                    active_color,
                    from,
                    to,
                    base_hmc,
                    sr,
                    is_capture,
                    moved_is_pawn,
                    ps,
                );
                scored.push((from, to, adj));
            }

            sort_moves_on_score_asc(&mut scored);
            if active_color == Color::White {
                scored.reverse();
            }

            if let Some((from, to)) = select_move_based_using_strength(&scored, playing_strength) {
                let sc = scored
                    .iter()
                    .find(|e| e.0 == from && e.1 == to)
                    .map(|e| e.2)
                    .unwrap_or(sc);
                Some((from, to, sc, used_depth))
            } else {
                Some((bf, bt, sc, used_depth))
            }
        }
    } else {
        None
    }
}

pub(crate) fn find_all_valid_moves(
    board: &Board,
    active_color: Color,
) -> Vec<((usize, usize), (usize, usize))> {
    let mut result: Vec<((usize, usize), (usize, usize))> = Vec::new();

    // iterate all squares and collect legal moves for the active color
    for r in 0..8 {
        for c in 0..8 {
            let piece = match board.get(r, c) {
                Some(p) => p,
                None => continue,
            };
            if piece.get_color() != active_color {
                continue;
            }

            for tr in 0..8 {
                for tc in 0..8 {
                    let from = (r, c);
                    let to = (tr, tc);
                    if from == to {
                        continue;
                    }

                    let target_piece_is_some = board.get(tr, tc).is_some();

                    // basic board-level validation (ownership, capture flags, bounds)
                    let is_capture = target_piece_is_some;
                    let is_pawn_move = piece.get_type() == PieceType::Pawn;
                    if !board.move_from_and_to_validation_check(
                        from,
                        to,
                        active_color,
                        is_capture,
                        is_pawn_move,
                        None,
                    ) {
                        continue;
                    }

                    // Special-case: allow castling generation using GameState-aware validator
                    if piece.get_type() == PieceType::King {
                        // Try regular single-square king moves via existing validator
                        let dr = if r > tr { r - tr } else { tr - r };
                        let dc = if c > tc { c - tc } else { tc - c };
                        if dr <= 1 && dc <= 1 {
                            if is_piece_move_valid(
                                board,
                                active_color,
                                r,
                                c,
                                piece,
                                tr,
                                tc,
                                from,
                                to,
                                is_capture,
                            ) {
                                result.push((from, to));
                            }
                        } else if dr == 0 && dc == 2 {
                            // Potential castle squares: e1g1/e1c1 or e8g8/e8c8
                            // Reuse GameState + king_move_validator by simulating via PieceMover.
                            // Build a temporary GameState-like context from the current board and a neutral state.
                            use crate::state::game_state::GameState;
                            // Create a shallow copy of the current GameState is not available here; so we can
                            // construct a new GameState with this board snapshot and default state, then set side.
                            // GameState is Copy in some contexts; when not, we create from board.
                            let mut gs = GameState::from_board_and_side(*board, active_color);
                            let can_move = PieceMover::move_piece(&mut gs, from, to, is_capture, None);
                            if can_move {
                                // We do not commit the move here; generator only needs legality.
                                result.push((from, to));
                            }
                        }
                    } else {
                        if is_piece_move_valid(
                            board,
                            active_color,
                            r,
                            c,
                            piece,
                            tr,
                            tc,
                            from,
                            to,
                            is_capture,
                        ) {
                            result.push((from, to));
                        }
                    }
                }
            }
        }
    }
    result
}

// Sorts the move table by score in ascending order, in-place.
fn sort_moves_on_score_asc(
    move_table: &mut Vec<((usize, usize), (usize, usize), i32)>,
) {
    move_table.sort_by_key(|m| m.2);
}

#[inline]
fn reorder_with_tt_hint(
    ordered: &mut Vec<((usize, usize), (usize, usize))>,
    tt: &TranspositionTable,
    board: &Board,
    side: Color,
) {
    if let Some(entry) = tt.probe(compute_zobrist(board, side)) {
        let bm = decode_move(entry.best_from, entry.best_to);
        if let Some(pos) = ordered.iter().position(|m| *m == bm) {
            let first = ordered.remove(pos);
            ordered.insert(0, first);
        }
    }
}

#[inline]
fn aspiration_bounds_for_depth(depth_now: usize, last_score: i32, window: i32) -> (i32, i32) {
    if depth_now <= 1 {
        (MIN_EVAL_VALUE + 1, MAX_EVAL_VALUE - 1)
    } else {
        (
            (last_score - window).max(MIN_EVAL_VALUE + 1),
            (last_score + window).min(MAX_EVAL_VALUE - 1),
        )
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn evaluate_root_for_bounds(
    board: &Board,
    active_color: Color,
    root_moves: &Vec<((usize, usize), (usize, usize))>,
    depth_now: usize,
    a: i32,
    b: i32,
    tt: &mut TranspositionTable,
    base_hmc: u32,
    ps: i32,
    game_state: &GameState,
    history: &History,
) -> (((usize, usize), (usize, usize)), i32, i32) {
    let mut best_from_to: Option<((usize, usize), (usize, usize))> = None;
    let mut best_score_raw = if active_color == Color::White {
        MIN_EVAL_VALUE
    } else {
        MAX_EVAL_VALUE
    };
    let mut best_adjusted = best_score_raw;

    // Order: if TT has a move at root, try to place it first
    let mut ordered: Vec<((usize, usize), (usize, usize))> = root_moves.iter().copied().collect();
    reorder_with_tt_hint(&mut ordered, tt, board, active_color);

    //eprintln!("[root-bounds] depth={} ordered={} a={} b={} parallel?={}",
    //          depth_now, ordered.len(), a, b,
    //         (depth_now >= ROOT_PARALLEL_MIN_DEPTH && ordered.len() >= ROOT_PARALLEL_MIN_MOVES));

    let enable_parallel =
        depth_now >= ROOT_PARALLEL_MIN_DEPTH && ordered.len() >= ROOT_PARALLEL_MIN_MOVES;
    if enable_parallel {
        //eprintln!("Parallel root search enabled");
        // 1) Search the first (best-ordered) move serially to establish PV and bounds
        let &(pv_from, pv_to) = ordered.first().unwrap();
        {
            let (score_raw, is_capture, moved_is_pawn) = evaluate_after_root_move(
                board,
                active_color,
                pv_from,
                pv_to,
                depth_now,
                a,
                b,
                tt,
                base_hmc,
            );

            let adjusted = adjusted_root_eval_for_move(
                board,
                active_color,
                pv_from,
                pv_to,
                base_hmc,
                score_raw,
                is_capture,
                moved_is_pawn,
                ps,
            );

            best_from_to = Some((pv_from, pv_to));
            best_adjusted = adjusted;
            best_score_raw = score_raw;
        }

        // 2) Search the remaining moves in parallel with per-task local TT to avoid contention
        // reuse shared board reference in parallel (read-only access)
        let base_hmc_loc = base_hmc;
        let a_loc = a;
        let b_loc = b;
        let side = active_color;
        let results = ordered[1..]
            .par_iter()
            .map(|&(from, to)| {
                // local TT per task
                let mut local_tt = TranspositionTable::new_with_default_size();
                let (score_raw, is_capture, moved_is_pawn) = evaluate_after_root_move(
                    board,
                    side,
                    from,
                    to,
                    depth_now,
                    a_loc,
                    b_loc,
                    &mut local_tt,
                    base_hmc_loc,
                );

                // Root adjustments (skip repetition-history check to keep parallel code simple)
                let adjusted = adjusted_root_eval_for_move(
                    board,
                    side,
                    from,
                    to,
                    base_hmc_loc,
                    score_raw,
                    is_capture,
                    moved_is_pawn,
                    ps,
                );
                (from, to, adjusted, score_raw)
            })
            .reduce(
                || {
                    // Identity: invalid move placeholder not used; return extreme sentinel
                    (
                        (0usize, 0usize),
                        (0usize, 0usize),
                        if side == Color::White {
                            MIN_EVAL_VALUE
                        } else {
                            MAX_EVAL_VALUE
                        },
                        if side == Color::White {
                            MIN_EVAL_VALUE
                        } else {
                            MAX_EVAL_VALUE
                        },
                    )
                },
                |acc, x| {
                    let better = if side == Color::White {
                        x.2 > acc.2
                    } else {
                        x.2 < acc.2
                    };
                    if better { x } else { acc }
                },
            );

        // Update best with parallel results if better
        let (pf, pt, padj, praw) = results;
        // Ignore identity placeholder
        if !(pf == (0, 0) && pt == (0, 0)) {
            let better = if active_color == Color::White {
                padj > best_adjusted
            } else {
                padj < best_adjusted
            };
            if better {
                best_from_to = Some((pf, pt));
                best_adjusted = padj;
                best_score_raw = praw;
            }
        }
    } else {
        // Search sequentially over root moves
        //eprintln!(
        //    "[root-serial] depth={} scanning {} moves with a={} b={}",
        //    depth_now,
        //    ordered.len(),
        //    a,
        //    b
        //);
        for &(from, to) in &ordered {

            //eprintln!("[root-serial] try mv={:?}->{:?}", from, to);

            let (score_raw, is_capture, moved_is_pawn) = evaluate_after_root_move(
                board,
                active_color,
                from,
                to,
                depth_now,
                a,
                b,
                tt,
                base_hmc,
            );

            //eprintln!(
            //    "[root-serial] mv={:?}->{:?} raw={} (alpha={}, beta={})",
            //    (from, to).0,
            //    (from, to).1,
            //    score_raw,
            //    a,
            //    b
            //);

            // Adjust score for root-only heuristics
            let mut adjusted = adjusted_root_eval_for_move(
                board,
                active_color,
                from,
                to,
                base_hmc,
                score_raw,
                is_capture,
                moved_is_pawn,
                ps,
            );
            // repetition-avoidance at root
            adjusted = apply_repetition_avoidance_bias(
                adjusted,
                game_state,
                history,
                board,
                active_color,
                from,
                to,
            );

            // Track best
            let better = if active_color == Color::White {
                adjusted > best_adjusted
            } else {
                adjusted < best_adjusted
            };

            //eprintln!(
            //  "[root-serial] adj={} best_adj_so_far={} best_raw_so_far={}",
            //    adjusted,
            //    best_adjusted,
            //    best_score_raw
            //);

            if better || best_from_to.is_none() {
                best_from_to = Some((from, to));
                best_adjusted = adjusted;
                best_score_raw = score_raw;
            }
            // Aspiration cutoffs help ordering mid-loop too
            if active_color == Color::White && score_raw >= b {
                //eprintln!("[root-serial] cutoff WHITE raw={} >= beta={}, break", score_raw, b);
                break;
            }
            if active_color == Color::Black && score_raw <= a {
                //eprintln!("[root-serial] cutoff BLACK raw={} <= alpha={}, break", score_raw, a);
                break;
            }
        }
    }

    //eprintln!(
    //    "[root-serial] RETURN depth={} mv={:?} best_raw={} best_adj={}",
    //    depth_now,
    //    best_from_to.unwrap(),
    //    best_score_raw,
    //    best_adjusted
    //);

    (best_from_to.unwrap(), best_adjusted, best_score_raw)
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn probe_with_aspiration(
    board: &Board,
    active_color: Color,
    root_moves: &Vec<((usize, usize), (usize, usize))>,
    depth_now: usize,
    last_score: i32,
    window: &mut i32,
    tt: &mut TranspositionTable,
    base_hmc: u32,
    ps: i32,
    game_state: &GameState,
    history: &History,
) -> (((usize, usize), (usize, usize)), i32, i32) {
    let (mut a, mut b) = aspiration_bounds_for_depth(depth_now, last_score, *window);

    //eprintln!("[asp] depth={} init a={} b={} last={}", depth_now, a, b, last_score);

    let mut tried = 0;
    loop {
        tried += 1;

        //eprintln!("[asp] depth={} try={} a={} b={}", depth_now, tried, a, b);

        let (mv, best_adjusted, best_score_raw) = evaluate_root_for_bounds(
            board,
            active_color,
            root_moves,
            depth_now,
            a,
            b,
            tt,
            base_hmc,
            ps,
            game_state,
            history,
        );

        //eprintln!("[asp] result depth={} try={} raw={} adj={} mv={:?}->{:?}",
        //          depth_now, tried, best_score_raw, best_adjusted, mv.0, mv.1);

        // Check aspiration result
        if best_score_raw <= a {
            // fail-low: widen down

            //eprintln!("[asp] FAIL-LOW depth={} try={} raw={} <= a={}; expand window {}->{}",
            //          depth_now, tried, best_score_raw, a, *window, (*window * 2).min(ASP_WINDOW_MAX_CP));

            *window = (*window * 2).min(ASP_WINDOW_MAX_CP);
            let bounds = aspiration_bounds_for_depth(depth_now, last_score, *window);
            a = bounds.0;
            if tried < 3 {
                continue;
            }
        } else if best_score_raw >= b {
            // fail-high: widen up

            //eprintln!("[asp] FAIL-HIGH depth={} try={} raw={} >= b={}; expand window {}->{}",
            //          depth_now, tried, best_score_raw, b, *window, (*window * 2).min(ASP_WINDOW_MAX_CP));

            *window = (*window * 2).min(ASP_WINDOW_MAX_CP);
            let bounds = aspiration_bounds_for_depth(depth_now, last_score, *window);
            b = bounds.1;
            if tried < 3 {
                continue;
            }
        }
        return (mv, best_adjusted, best_score_raw);
    }
}

#[inline]
fn apply_repetition_avoidance_bias(
    adjusted: i32,
    game_state: &GameState,
    history: &History,
    board: &Board,
    active_color: Color,
    from: (usize, usize),
    to: (usize, usize),
) -> i32 {
    let mut adjusted = adjusted;
    let is_capture = board.get(to.0, to.1).is_some();
    let mut gs = *game_state; // Copy
    let mut promote: Option<Piece> = None;
    if let Some(p) = gs.board().get(from.0, from.1) {
        if p.get_type() == PieceType::Pawn {
            if (active_color == Color::White && to.0 == 7)
                || (active_color == Color::Black && to.0 == 0)
            {
                promote = Some(Piece::new(PieceType::Queen, active_color));
            }
        }
    }
    if PieceMover::move_piece(&mut gs, from, to, is_capture, promote) {
        gs.switch_player_turn();
        let fen = game_state_to_fen_string(gs);
        let truncated = fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ");
        let count = history.fen_repetition_count(&truncated);
        let sa = if active_color == Color::White {
            adjusted
        } else {
            -adjusted
        };
        if count >= 2 && sa > 0 {
            adjusted -= if active_color == Color::White {
                REP_AVOIDANCE_BIAS_CP
            } else {
                -REP_AVOIDANCE_BIAS_CP
            };
        }
    }
    adjusted
}
