use std::sync::{Mutex, OnceLock};
use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};
use crate::search::heuristics::SearchHeuristics;
use crate::search::prune_null_moves::prune_null_moves;
use crate::search::qsearch::qsearch;
use crate::search::advanced_search::{find_all_valid_moves, MAX_EVAL_VALUE, MIN_EVAL_VALUE, SEARCH_ABORTED};
use crate::state::game_state::GameState;
use crate::search::telemetry::bump_node;
use crate::search::time_control::time_is_up;
use crate::search::tt::{decode_move, encode_move, from_tt_score, to_tt_score, Bound, TranspositionTable, MATE_VALUE};
use crate::search::zobrist::compute_zobrist;

const HUNDRED_HALF_MOVES: u32 = 100;

// Helper functions for color-agnostic minimax logic

#[inline]
fn initial_value(maximizing: bool) -> i32 {
    if maximizing { MIN_EVAL_VALUE } else { MAX_EVAL_VALUE }
}

#[inline]
fn is_better(score: i32, best: i32, maximizing: bool) -> bool {
    if maximizing { score > best } else { score < best }
}

#[inline]
fn null_window(alpha: i32, beta: i32, maximizing: bool) -> (i32, i32) {
    if maximizing { (alpha, alpha + 1) } else { (beta - 1, beta) }
}

#[inline]
fn is_good_history(hist: i32, maximizing: bool) -> bool {
    if maximizing { hist > 10_000 } else { hist < -10_000 }
}

#[inline]
fn opponent(color: Color) -> Color {
    match color {
        Color::White => Color::Black,
        Color::Black => Color::White,
    }
}

// Alpha-beta pruning Search. Returns evaluation in centipawns (positive is better for White).
pub fn alphabeta(
    board: &mut Board,
    to_move: Color,
    depth: usize,
    mut alpha: i32,
    mut beta: i32,
    ply: i32,
    tt: &mut TranspositionTable,
    half_move_clock: u32,
    rep_stack: &mut Vec<u64>,
) -> i32 {
    // Count every node we enter
    bump_node();

    // Time cutoff: return SEARCH_ABORTED to signal interruption
    if time_is_up() {
        return SEARCH_ABORTED;
    }

    // Repetition/50-move draw checks at node entry (only if Zobrist hashing is enabled)
    let key_here: u64 = if crate::search::advanced_search::ZOBRIST_HASHING_ENABLED {
        compute_zobrist(&*board, to_move)
    } else { 0 };
    if crate::search::advanced_search::ZOBRIST_HASHING_ENABLED {
        // If this key already exists in the current line, it's a repetition -> draw
        if rep_stack.iter().any(|&k| k == key_here) {
            return 0;
        }
    }
    // 50-move rule
    if half_move_clock >= HUNDRED_HALF_MOVES {
        return 0;
    }

    if depth == 0 {
        // At leaf: switch to quiescence to avoid horizon effects
        return qsearch(board, to_move, alpha, beta, half_move_clock, rep_stack);
    }

    if let Some(value) = prune_null_moves(board, to_move, depth, alpha, beta, ply, tt, half_move_clock, rep_stack) {
        return value;
    }

    // TT probe
    let key = key_here;
    if let Some(entry) = tt.probe(key) {
        if entry.depth as usize >= depth {
            let tt_score = from_tt_score(entry.score, ply);
            match entry.bound {
                Bound::Exact => {
                    return tt_score;
                }
                Bound::Lower => {
                    if tt_score > alpha {
                        alpha = tt_score;
                    }
                }
                Bound::Upper => {
                    if tt_score < beta {
                        beta = tt_score;
                    }
                }
            }
        }
    }

    let gs = GameState::from_board_and_side((*board).clone(), to_move);
    let mut moves: Vec<((usize, usize), (usize, usize), Option<char>)> = find_all_valid_moves(&gs);
    // If TT has the best move, try it first
    if let Some(entry) = tt.probe(key) {
        let bm = decode_move(entry.best_from, entry.best_to);
        if let Some(pos) = moves.iter().position(|(f, t, _)| (*f, *t) == bm) {
            let first = moves.remove(pos);
            moves.insert(0, first);
        }
    }
    // Basic move ordering with heuristics: after TT move, sort by composite key
    if moves.len() > 1 {
        let (head, tail) = moves.split_at_mut(1);
        let board_ref = &*board;
        let hmc = half_move_clock;
        thread_local! {
            static HEUR: OnceLock<Mutex<SearchHeuristics>> = OnceLock::new();
        }
        tail.sort_by_key(|&(from, to, promo)| {
            let mut key = if crate::search::advanced_search::MVV_LVA_ENABLED {
                board_ref.move_score_mvv_lva(from, to)
            } else { 0 };

            if let Some(p) = promo {
                key += match p {
                    'q' => 900,
                    'r' => 500,
                    'b' => 330,
                    'n' => 320,
                    _ => 0,
                };
            }

            let moved_is_pawn = board_ref
                .get(from.0, from.1)
                .map(|p| p.get_type() == PieceType::Pawn)
                .unwrap_or(false);
            let is_capture = board_ref.get(to.0, to.1).is_some();
            if hmc >= 80 {
                if moved_is_pawn || is_capture {
                    key += 100_000;
                }
            }
            if !is_capture && !moved_is_pawn {
                let is_killer = HEUR.with(|h| {
                    let m = h
                        .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                        .lock()
                        .unwrap();
                    m.is_killer(ply as usize, from, to)
                });
                if is_killer {
                    key += 200_000;
                }
                let hist = HEUR.with(|h| {
                    let m = h
                        .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                        .lock()
                        .unwrap();
                    m.history_score(to_move, from, to)
                });
                key += (hist / 32).clamp(-200_000, 200_000);
            }
            -key
        });
        let _ = head;
    }
    if moves.is_empty() {
        let in_check = board.is_side_in_check(to_move);
        return if in_check {
            -MATE_VALUE + depth as i32
        } else {
            0
        }
    }

    let original_alpha = alpha;
    let original_beta = beta;
    let mut best_from_to: Option<((usize, usize), (usize, usize))> = None;
    let pushed_rep = if crate::search::advanced_search::ZOBRIST_HASHING_ENABLED {
        rep_stack.push(key_here);
        true
    } else { false };

    thread_local! {
        static HEUR: OnceLock<Mutex<SearchHeuristics>> = OnceLock::new();
    }

    // Unified move loop - works for both maximizing (White) and minimizing (Black)
    let maximizing = to_move == Color::White;
    let opp = opponent(to_move);
    let mut value = initial_value(maximizing);
    let mut is_first_move = true;
    let mut move_index: i32 = 0;

    for (from, to, promo) in moves.into_iter() {
        let moved_piece = board.get(from.0, from.1);
        let target_piece = board.get(to.0, to.1);
        let u = board.make_move_simple(from, to, promo);

        // Passed-pawn push extension
        let mut child_depth = depth.saturating_sub(1);
        let mut child_hmc = half_move_clock + 1;
        if let Some(p) = moved_piece {
            if p.get_type() == PieceType::Pawn {
                child_hmc = 0;
                let color = p.get_color();
                let r = to.0;
                let c = to.1;
                if board.game_phase_light() <= 8 && board.is_passed_pawn_simple(r, c, color) {
                    let adv: i32 = match color {
                        Color::White => r as i32,
                        Color::Black => (7 - r) as i32,
                    };
                    if adv >= 5 {
                        child_depth = child_depth.saturating_add(1);
                    }
                }
            }
        }
        if target_piece.is_some() {
            child_hmc = 0;
        }

        let is_capture = target_piece.is_some();
        let is_pawn_move = moved_piece
            .map(|p| p.get_type() == PieceType::Pawn)
            .unwrap_or(false);
        let quiet = !is_capture && !is_pawn_move;

        // Principal Variation Search (PVS) + Late Move Reductions (LMR)
        let score;
        if is_first_move {
            // Full window on the first move
            score = alphabeta(
                board,
                opp,
                child_depth,
                alpha,
                beta,
                ply + 1,
                tt,
                child_hmc,
                rep_stack,
            );
            is_first_move = false;
        } else {
            // Late Move Reduction
            let mut reduced_depth = child_depth;
            let gives_check = board.is_side_in_check(opp);
            let allow_reduce = !(gives_check && child_depth <= 5);

            if crate::search::advanced_search::LMR_ENABLED {
                if quiet && child_depth >= 3 && move_index >= 4 && allow_reduce {
                    let hist = HEUR.with(|h| {
                        let m = h
                            .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                            .lock()
                            .unwrap();
                        m.history_score(to_move, from, to)
                    });
                    let hist_good = is_good_history(hist, maximizing);
                    let mut r = 1 + ((move_index as usize) / 6).min(1);
                    if child_depth >= 8 {
                        r += 1;
                    }
                    if hist_good {
                        r = r.saturating_sub(1);
                    }
                    // Extra reduction for quiet queen moves in opening
                    if let Some(mp) = moved_piece {
                        if mp.get_type() == PieceType::Queen {
                            let phase = board.game_phase_light();
                            if phase >= 12 {
                                let back_r = if to_move == Color::White { 0usize } else { 7usize };
                                let mut undeveloped = 0;
                                for fc in 0..8 {
                                    if let Some(p) = board.get(back_r, fc) {
                                        if p.get_color() == to_move {
                                            match p.get_type() {
                                                PieceType::Knight | PieceType::Bishop => { undeveloped += 1; },
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                let king_home_r = back_r;
                                let mut king_file: Option<usize> = None;
                                for fc in 0..8 {
                                    if let Some(kp) = board.get(king_home_r, fc) {
                                        if kp.get_color() == to_move && kp.get_type() == PieceType::King {
                                            king_file = Some(fc);
                                            break;
                                        }
                                    }
                                }
                                let uncastled = match king_file { Some(kf) => !(kf == 2 || kf == 6), None => true };
                                if undeveloped >= 3 { r += 2; }
                                else if undeveloped >= 2 { r += 1; }
                                if uncastled { r += 1; }
                            }
                        }
                    }
                    r = r.min(3);
                    reduced_depth = reduced_depth.saturating_sub(r);
                }
            }

            // Null-window Search for subsequent moves (PVS)
            let (nw_alpha, nw_beta) = null_window(alpha, beta, maximizing);
            let mut sc = alphabeta(
                board,
                opp,
                reduced_depth,
                nw_alpha,
                nw_beta,
                ply + 1,
                tt,
                child_hmc,
                rep_stack,
            );

            // Re-search with full window if score falls within (alpha, beta)
            if sc > alpha && sc < beta {
                sc = alphabeta(
                    board,
                    opp,
                    child_depth,
                    alpha,
                    beta,
                    ply + 1,
                    tt,
                    child_hmc,
                    rep_stack,
                );
            }
            score = sc;
        }
        board.unmake_move_simple(u);

        // Update best value
        if is_better(score, value, maximizing) {
            value = score;
        }

        // Update alpha/beta bound and best move
        if maximizing {
            if value > alpha {
                alpha = value;
                best_from_to = Some((from, to));
                if quiet {
                    HEUR.with(|h| {
                        let mut m = h
                            .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                            .lock()
                            .unwrap();
                        m.add_history(to_move, from, to, (depth as i32) * (depth as i32));
                    });
                }
            } else if quiet && score >= beta {
                HEUR.with(|h| {
                    let mut m = h
                        .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                        .lock()
                        .unwrap();
                    m.add_killer(ply as usize, from, to);
                });
            }
        } else {
            if value < beta {
                beta = value;
                best_from_to = Some((from, to));
                if quiet {
                    HEUR.with(|h| {
                        let mut m = h
                            .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                            .lock()
                            .unwrap();
                        m.add_history(to_move, from, to, (depth as i32) * (depth as i32));
                    });
                }
            } else if quiet && score <= alpha {
                HEUR.with(|h| {
                    let mut m = h
                        .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
                        .lock()
                        .unwrap();
                    m.add_killer(ply as usize, from, to);
                });
            }
        }

        // Cutoff check
        if alpha >= beta {
            break;
        }
        move_index += 1;
    }

    // Pop this node key
    if pushed_rep { let _ = rep_stack.pop(); }

    // Store to TT
    let bound = if value <= original_alpha {
        Bound::Upper
    } else if value >= original_beta {
        Bound::Lower
    } else {
        Bound::Exact
    };
    let (bf, bt) = if let Some((f, t)) = best_from_to {
        let (ff, tt2) = encode_move(f, t);
        (Some(ff), Some(tt2))
    } else {
        (None, None)
    };
    let tt_score = to_tt_score(value, ply);
    tt.store(key, depth as i16, bound, tt_score, bf, bt);
    value
}
