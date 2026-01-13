use std::sync::{Mutex, OnceLock};
use crate::piece::pieces::{Color, PieceType};
use crate::search::heuristics::SearchHeuristics;
use crate::search::prune_null_moves::prune_null_moves;
use crate::search::qsearch::qsearch;
use crate::search::advanced_search::{find_all_valid_moves, MAX_EVAL_VALUE, MIN_EVAL_VALUE, SEARCH_ABORTED};
use crate::state::game_state::GameState;
use crate::search::telemetry::bump_node;
use crate::search::time_control::time_is_up;
use crate::search::tt::{decode_move, encode_move, from_tt_score, to_tt_score, Bound, TranspositionTable, MATE_VALUE};
use crate::search::zobrist::compute_zobrist_full;

const HUNDRED_HALF_MOVES: u32 = 100;

// ============================================================
// HELPER FUNCTIONS
// ============================================================

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

/// Thread-local heuristics accessor
#[inline]
fn with_heuristics<F, R>(f: F) -> R
where
    F: FnOnce(&mut SearchHeuristics) -> R,
{
    thread_local! {
        static HEUR: OnceLock<Mutex<SearchHeuristics>> = OnceLock::new();
    }
    HEUR.with(|h| {
        let mut m = h
            .get_or_init(|| Mutex::new(SearchHeuristics::new(128)))
            .lock()
            .unwrap();
        f(&mut m)
    })
}

// ============================================================
// MOVE ORDERING
// ============================================================

struct MoveOrderingContext {
    half_move_clock: u32,
}

/// Compute a heuristic score for move ordering
fn compute_move_order_score(
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
    moved_type: Option<PieceType>,
    is_capture: bool,
    mvv_lva: i32,
    to_move: Color,
    ply: i32,
    ctx: &MoveOrderingContext,
) -> i32 {
    let mut key = mvv_lva;

    // Promotion bonus
    if let Some(p) = promo {
        key += match p {
            'q' => 900,
            'r' => 500,
            'b' => 330,
            'n' => 320,
            _ => 0,
        };
    }

    let is_pawn_moved = moved_type == Some(PieceType::Pawn);

    // Near 50-move rule: prioritize pawn moves and captures
    if ctx.half_move_clock >= 80 && (is_pawn_moved || is_capture) {
        key += 100_000;
    }

    // Quiet moves: check killer and history heuristics
    if !is_capture && !is_pawn_moved {
        if with_heuristics(|h| h.is_killer(ply as usize, from, to)) {
            key += 200_000;
        }
        let hist = with_heuristics(|h| h.history_score(to_move, from, to));
        key += (hist / 32).clamp(-200_000, 200_000);
    }

    key
}

/// Order moves for better alpha-beta cutoffs
fn order_moves(
    moves: Vec<((usize, usize), (usize, usize), Option<char>)>,
    game_state: &GameState,
    tt: &TranspositionTable,
    key: u64,
    to_move: Color,
    ply: i32,
) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    if moves.is_empty() {
        return moves;
    }

    let mut ordered = moves;

    // Try TT move first
    if let Some(entry) = tt.probe(key) {
        let bm = decode_move(entry.best_from, entry.best_to);
        if let Some(pos) = ordered.iter().position(|(f, t, _)| (*f, *t) == bm) {
            let first = ordered.remove(pos);
            ordered.insert(0, first);
        }
    }

    if ordered.len() <= 1 {
        return ordered;
    }

    let ctx = MoveOrderingContext {
        half_move_clock: game_state.half_move_clock(),
    };

    // Collect piece info before sorting
    let piece_info: Vec<_> = {
        let b = game_state.board();
        ordered
            .iter()
            .map(|&(from, to, _)| {
                (
                    b.get(from.0, from.1).map(|p| p.get_type()),
                    b.get(to.0, to.1).is_some(),
                    if crate::search::advanced_search::MVV_LVA_ENABLED {
                        b.move_score_mvv_lva(from, to)
                    } else {
                        0
                    },
                )
            })
            .collect()
    };

    // Score all moves
    let mut moves_with_scores: Vec<_> = ordered
        .into_iter()
        .enumerate()
        .map(|(i, (from, to, promo))| {
            let (moved_type, is_capture, mvv_lva) = piece_info[i];
            let score = compute_move_order_score(
                from, to, promo, moved_type, is_capture, mvv_lva,
                to_move, ply, &ctx,
            );
            ((from, to, promo), score)
        })
        .collect();

    // Sort everything after the first move (TT move)
    if moves_with_scores.len() > 1 {
        let (_, tail) = moves_with_scores.split_at_mut(1);
        tail.sort_by_key(|&(_, score)| -score);
    }

    moves_with_scores.into_iter().map(|(m, _)| m).collect()
}

// ============================================================
// LATE MOVE REDUCTION (LMR)
// ============================================================

/// Calculate the reduction depth for LMR
fn calculate_lmr_reduction(
    child_depth: usize,
    move_index: i32,
    is_quiet: bool,
    _gives_check: bool,
    allow_reduce: bool,
    to_move: Color,
    from: (usize, usize),
    to: (usize, usize),
    board: &crate::board::Board,
    phase: i32,
) -> usize {
    if !crate::search::advanced_search::LMR_ENABLED {
        return 0;
    }

    if !is_quiet || child_depth < 3 || move_index < 4 || !allow_reduce {
        return 0;
    }

    let hist = with_heuristics(|h| h.history_score(to_move, from, to));
    let is_maximizing = to_move == Color::White;
    let hist_good = is_good_history(hist, is_maximizing);

    let mut r = 1 + ((move_index as usize) / 6).min(1);

    if child_depth >= 8 {
        r += 1;
    }

    if hist_good {
        r = r.saturating_sub(1);
    }

    // Extra reduction for quiet queen moves in opening with undeveloped pieces
    if let Some(mp) = board.get(to.0, to.1) {
        if mp.get_type() == PieceType::Queen && phase >= 12 {
            let back_r = if to_move == Color::White { 0 } else { 7 };
            let mut undeveloped = 0;

            for fc in 0..8 {
                if let Some(p) = board.get(back_r, fc) {
                    if p.get_color() == to_move && matches!(p.get_type(), PieceType::Knight | PieceType::Bishop) {
                        undeveloped += 1;
                    }
                }
            }

            let king_file = (0..8).find(|&fc| {
                board.get(back_r, fc)
                    .map_or(false, |p| p.get_color() == to_move && p.get_type() == PieceType::King)
            });
            let uncastled = king_file.map_or(true, |kf| kf != 2 && kf != 6);

            if undeveloped >= 3 {
                r += 2;
            } else if undeveloped >= 2 {
                r += 1;
            }

            if uncastled {
                r += 1;
            }
        }
    }

    r.min(3)
}

// ============================================================
// MOVE SEARCH LOGIC
// ============================================================

struct MoveSearchResult {
    best_value: i32,
    best_from_to: Option<((usize, usize), (usize, usize))>,
}

/// Search all moves and update alpha/beta bounds
fn search_moves(
    moves: Vec<((usize, usize), (usize, usize), Option<char>)>,
    game_state: &mut GameState,
    depth: usize,
    mut alpha: i32,
    mut beta: i32,
    ply: i32,
    tt: &mut TranspositionTable,
    rep_stack: &mut Vec<u64>,
    to_move: Color,
) -> MoveSearchResult {
    let maximizing = to_move == Color::White;
    let mut current_value = initial_value(maximizing);
    let mut best_from_to: Option<((usize, usize), (usize, usize))> = None;
    let mut is_first_move = true;
    let mut move_index: i32 = 0;

    for (from, to, promo) in moves {
        let u = game_state.make_move_fast(from, to, promo);

        // Passed-pawn extension
        let mut child_depth = depth.saturating_sub(1);
        let is_capture = u.ep_captured_piece.is_some() || u.board_undo.captured.is_some();

        {
            let b = game_state.board();
            if let Some(p) = b.get(to.0, to.1) {
                if p.get_type() == PieceType::Pawn
                    && b.game_phase_light() <= 8
                    && b.is_passed_pawn_simple(to.0, to.1, p.get_color())
                {
                    let adv = match p.get_color() {
                        Color::White => to.0 as i32,
                        Color::Black => (7 - to.0) as i32,
                    };
                    if adv >= 5 {
                        child_depth = child_depth.saturating_add(1);
                    }
                }
            }
        }

        let gives_check = game_state.mutable_board().is_side_in_check(opponent(to_move));
        let quiet = !is_capture && game_state.board().get(to.0, to.1)
            .map_or(false, |p| p.get_type() != PieceType::Pawn);
        let allow_reduce = !(gives_check && child_depth <= 5);

        // Calculate LMR reduction
        let reduction = if is_first_move {
            0
        } else {
            calculate_lmr_reduction(
                child_depth, move_index, quiet, gives_check, allow_reduce,
                to_move, from, to, game_state.board(),
                game_state.board().game_phase_light(),
            )
        };
        let reduced_depth = child_depth.saturating_sub(reduction);

        // Search with appropriate window
        let current_score = if is_first_move {
            // Full window on first move
            alphabeta(game_state, child_depth, alpha, beta, ply + 1, tt, rep_stack)
        } else {
            // Null-window search
            let (nw_alpha, nw_beta) = null_window(alpha, beta, maximizing);
            let mut sc = alphabeta(game_state, reduced_depth, nw_alpha, nw_beta, ply + 1, tt, rep_stack);

            // Re-search with full window if needed
            if is_better(sc, alpha, maximizing) && (reduced_depth < child_depth || is_better(beta, sc, maximizing)) {
                sc = alphabeta(game_state, child_depth, alpha, beta, ply + 1, tt, rep_stack);
            }
            sc
        };

        game_state.unmake_move_fast(u);
        is_first_move = false;

        // Update best value
        if is_better(current_score, current_value, maximizing) {
            current_value = current_score;
        }

        // Update bounds and track best move
        if maximizing {
            if current_value > alpha {
                alpha = current_value;
                best_from_to = Some((from, to));
                if quiet {
                    with_heuristics(|h| {
                        h.add_history(to_move, from, to, (depth as i32) * (depth as i32));
                    });
                }
            } else if quiet && current_score >= beta {
                with_heuristics(|h| h.add_killer(ply as usize, from, to));
            }
            if current_value >= beta {
                break; // Beta cutoff
            }
        } else {
            if current_value < beta {
                beta = current_value;
                best_from_to = Some((from, to));
                if quiet {
                    with_heuristics(|h| {
                        h.add_history(to_move, from, to, (depth as i32) * (depth as i32));
                    });
                }
            } else if quiet && current_score <= alpha {
                with_heuristics(|h| h.add_killer(ply as usize, from, to));
            }
            if current_value <= alpha {
                break; // Alpha cutoff
            }
        }

        move_index += 1;
    }

    MoveSearchResult {
        best_value: current_value,
        best_from_to,
    }
}

// ============================================================
// MAIN ALPHA-BETA FUNCTION
// ============================================================

/// Alpha-beta pruning search with PVS, LMR, and null-move pruning.
/// Returns evaluation in centipawns (positive is better for White).
pub fn alphabeta(
    game_state: &mut GameState,
    depth: usize,
    mut alpha: i32,
    mut beta: i32,
    ply: i32,
    tt: &mut TranspositionTable,
    rep_stack: &mut Vec<u64>,
) -> i32 {
    let to_move = game_state.active_color();
    bump_node();

    // Time cutoff
    if time_is_up() {
        return SEARCH_ABORTED;
    }

    // Zobrist key for repetition detection and TT
    let key = if crate::search::advanced_search::ZOBRIST_HASHING_ENABLED {
        compute_zobrist_full(
            game_state.board(),
            to_move,
            &game_state.castling_rights(),
            game_state.en_passant_target(),
        )
    } else {
        0
    };

    // Repetition detection
    let mut pushed_rep = false;
    if crate::search::advanced_search::ZOBRIST_HASHING_ENABLED {
        if rep_stack.iter().any(|&k| k == key) {
            return 0; // Draw by repetition
        }
        rep_stack.push(key);
        pushed_rep = true;
    }

    // 50-move rule
    if game_state.half_move_clock() >= HUNDRED_HALF_MOVES {
        if pushed_rep {
            rep_stack.pop();
        }
        return 0;
    }

    // Leaf node: switch to quiescence search
    if depth == 0 {
        let result = qsearch(game_state, alpha, beta, rep_stack);
        if pushed_rep {
            rep_stack.pop();
        }
        return result;
    }

    // Null-move pruning
    if let Some(value) = prune_null_moves(game_state, depth, alpha, beta, ply, tt, rep_stack) {
        if pushed_rep {
            rep_stack.pop();
        }
        return value;
    }

    // Transposition table probe
    if let Some(entry) = tt.probe(key) {
        if entry.depth as usize >= depth {
            let tt_score = from_tt_score(entry.score, ply);
            match entry.bound {
                Bound::Exact => {
                    if pushed_rep {
                        rep_stack.pop();
                    }
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
            // Early cutoff after bound update
            if alpha >= beta {
                if pushed_rep {
                    rep_stack.pop();
                }
                return tt_score;
            }
        }
    }

    // Generate and order moves
    let moves = find_all_valid_moves(game_state);

    // No legal moves: checkmate or stalemate
    if moves.is_empty() {
        let in_check = game_state.mutable_board().is_side_in_check(to_move);
        if pushed_rep {
            rep_stack.pop();
        }
        return if in_check {
            -MATE_VALUE + depth as i32
        } else {
            0
        };
    }

    let ordered_moves = order_moves(moves, game_state, tt, key, to_move, ply);
    let original_alpha = alpha;
    let original_beta = beta;

    // Search all moves
    let result = search_moves(
        ordered_moves,
        game_state,
        depth,
        alpha,
        beta,
        ply,
        tt,
        rep_stack,
        to_move,
    );

    // Pop repetition stack
    if pushed_rep {
        rep_stack.pop();
    }

    // Store to transposition table
    let bound = if result.best_value <= original_alpha {
        Bound::Upper
    } else if result.best_value >= original_beta {
        Bound::Lower
    } else {
        Bound::Exact
    };

    let (bf, bt) = result.best_from_to
        .map(|(f, t)| {
            let (ff, tt2) = encode_move(f, t);
            (Some(ff), Some(tt2))
        })
        .unwrap_or((None, None));

    let tt_score = to_tt_score(result.best_value, ply);
    tt.store(key, depth as i16, bound, tt_score, bf, bt);

    result.best_value
}
