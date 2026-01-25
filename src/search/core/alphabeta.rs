use crate::board::evaluator::evaluate_position;
use crate::piece::pieces::{Color, Piece, PieceType, piece_value_cp};
use crate::search::evaluation::heuristics::SearchHeuristics;
use crate::search::management::prune_null_moves::prune_null_moves;
use crate::search::management::see::see_dest_estimate;
use crate::search::core::qsearch::qsearch;
use crate::search::core::advanced_search::{find_all_valid_moves, MAX_EVAL_VALUE, MIN_EVAL_VALUE, SEARCH_ABORTED};
use crate::state::game_state::GameState;
use crate::search::state::tt::{decode_move, encode_move, from_tt_score, to_tt_score, Bound, TranspositionTable, MATE_VALUE};
use crate::search::state::rep_stack::RepetitionStack;
use crate::search::context::SearchContext;

const HUNDRED_HALF_MOVES: u32 = 100;
const FRONTIER_FUTILITY_MARGIN: i32 = 200; // Margin for futility pruning at depth=1
const MOVE_ORDER_COUNTER_BONUS: i32 = 220_000;
const MOVE_ORDER_CONTINUATION_DIV: i32 = 16;
const MOVE_ORDER_CONTINUATION_CAP: i32 = 150_000;
const MOVE_ORDER_SEE_SCALE: i32 = 4;
const MOVE_ORDER_SEE_CAP: i32 = 2_000;
const RAZORING_MAX_DEPTH: usize = 1;
const RFP_MAX_DEPTH: usize = 2;
const LMP_MAX_DEPTH: usize = 2;
const LMP_QUIET_BASE: i32 = 6;
const LMP_QUIET_PER_DEPTH: i32 = 4;
const CHECK_EXTENSION_MAX_DEPTH: usize = 3;
pub(crate) const CHECK_EXTENSION_BUDGET: u8 = 1;
const SINGULAR_EXTENSION_MIN_DEPTH: usize = 6;
const SINGULAR_SEARCH_REDUCTION: usize = 2;
const SINGULAR_MARGIN: i32 = 80;
const SINGULAR_ALT_LIMIT: usize = 6;
const RAZOR_MARGIN_DEPTH_1: i32 = 220;
const RAZOR_MARGIN_DEPTH_2: i32 = 420;
const RFP_MARGIN_DEPTH_2: i32 = 160;
const RFP_MARGIN_DEPTH_3: i32 = 260;


// ============================================================
// MAIN ALPHA-BETA FUNCTION
// ============================================================

/// Alpha-beta pruning search with PVS, LMR, and null-move pruning.
/// Returns evaluation in centipawns (positive is better for White).
pub fn alphabeta(
    ctx: &SearchContext,
    heuristics: &mut SearchHeuristics,
    game_state: &mut GameState,
    depth: usize,
    mut alpha: i32,
    mut beta: i32,
    ply: i32,
    tt: &TranspositionTable,
    rep_stack: &mut RepetitionStack,
    allow_null_move: bool,
    prev_move: Option<((usize, usize), (usize, usize))>,
    check_extension_budget: u8,
) -> i32 {
    let to_move = game_state.active_color();
    ctx.bump_node();

    #[cfg(feature = "debug-search")]
    let indent = "  ".repeat(ply as usize);

    #[cfg(feature = "debug-search")] {
        if ply <= 4 {
            eprintln!("{}[AB] ply={} depth={} α={} β={} side={:?}",
                indent, ply, depth, alpha, beta, to_move);
        }
    }

    // Time cutoff
    if ctx.time_is_up() {
        return SEARCH_ABORTED;
    }

    // Zobrist key for repetition detection and TT
    let key = if crate::search::core::advanced_search::ZOBRIST_HASHING_ENABLED {
        game_state.zobrist_key()
    } else {
        0
    };

    // Repetition detection
    let mut pushed_rep = false;
    if crate::search::core::advanced_search::ZOBRIST_HASHING_ENABLED {
        if rep_stack.contains(&key) {
            #[cfg(feature = "debug-search")]
            if ply <= 4 {
                eprintln!("{}[AB] repetition detected -> return 0", indent);
            }
            return 0; // Draw by repetition
        }
        rep_stack.push(key);
        pushed_rep = true;
    }

    // 50-move rule
    if game_state.half_move_clock() >= HUNDRED_HALF_MOVES {
        #[cfg(feature = "debug-search")]
        if ply <= 4 {
            eprintln!("{}[AB] 50-move rule -> return 0", indent);
        }
        if pushed_rep {
            rep_stack.pop();
        }
        return 0;
    }

    // Leaf node: switch to quiescence search
    if depth == 0 {
        let result = qsearch(ctx, game_state, alpha, beta, rep_stack);
        #[cfg(feature = "debug-search")]
        if ply <= 4 {
            eprintln!("{}[AB] → qsearch returned {}", indent, result);
        }
        if pushed_rep {
            rep_stack.pop();
        }
        return result;
    }

    // Transposition table probe (before null-move pruning for cutoff + move hint)
    // Extract TT move hint before any mutable borrows
    let tt_entry = tt.probe(key);
    let tt_move_hint: Option<(u8, u8)> = tt_entry.map(|e| (e.best_from, e.best_to));
    let singular_info = tt_entry.and_then(|entry| {
        if entry.bound != Bound::Exact {
            return None;
        }
        let entry_depth = entry.depth as usize;
        if entry_depth + 1 < depth {
            return None;
        }
        let score = from_tt_score(entry.score, ply);
        if score.abs() >= MATE_VALUE - 100 {
            return None;
        }
        Some(SingularInfo {
            mv: decode_move(entry.best_from, entry.best_to),
            score,
            depth: entry_depth,
        })
    });

    if let Some(entry) = tt_entry
        && entry.depth as usize >= depth {
            let tt_score = from_tt_score(entry.score, ply);

            // Don't trust cached 0 scores near the root - they may be stale or imprecise
            // Force re-evaluation to get more accurate scores for move ordering
            let use_cached = !(tt_score == 0 && ply <= 3 && depth >= 3);

            if use_cached {
                match entry.bound {
                    Bound::Exact => {
                        #[cfg(feature = "debug-search")]
                        if ply <= 4 {
                            eprintln!("{}[AB] TT exact hit: score={}", indent, tt_score);
                        }
                        #[cfg(feature = "debug-search")]
                        if ply <= 4 && tt_score == 0 {
                            eprintln!("{}[AB] TT exact hit is 0", indent);
                        }
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
            }
            // Early cutoff after bound update
            if use_cached && alpha >= beta {
                if pushed_rep {
                    rep_stack.pop();
                }
                return tt_score;
            }
        }

    // Null-move pruning (after TT probe so we skip if TT already cut off)
    if let Some(value) = prune_null_moves(
        ctx,
        heuristics,
        game_state,
        depth,
        alpha,
        beta,
        ply,
        tt,
        rep_stack,
        allow_null_move,
        prev_move,
    ) {
        #[cfg(feature = "debug-search")]
        if ply <= 4 {
            eprintln!("{}[AB] null-move pruning returned {}", indent, value);
        }
        if pushed_rep {
            rep_stack.pop();
        }
        return value;
    }

    // Reverse futility pruning and razoring at shallow depths (skip in check)
    let ply_u = if ply < 0 { 0 } else { ply as usize };
    if (depth <= RFP_MAX_DEPTH || depth <= RAZORING_MAX_DEPTH)
        && ply >= 2
        && ply_u.saturating_add(depth) >= 5 {
        let in_check = game_state.mutable_board().is_side_in_check(to_move);
        if !in_check {
            let is_pv = beta.saturating_sub(alpha) > 1;
            if is_pv {
                // Skip shallow pruning in PV nodes to avoid missing tactical lines.
            } else {
                if !has_checking_move(game_state, to_move) {
                    let static_eval = evaluate_position(game_state.board(), to_move);
                    let maximizing = to_move == Color::White;

                if depth >= 2 && depth <= RFP_MAX_DEPTH {
                    let margin = rfp_margin(depth);
                    let rfp_cut = if maximizing {
                        static_eval - margin >= beta
                    } else {
                        static_eval + margin <= alpha
                    };
                    if rfp_cut {
                        if pushed_rep {
                            rep_stack.pop();
                        }
                        return static_eval;
                    }
                }

                    if depth <= RAZORING_MAX_DEPTH {
                    let margin = razor_margin(depth);
                    let razor_cut = if maximizing {
                        static_eval + margin <= alpha
                    } else {
                        static_eval - margin >= beta
                    };
                    if razor_cut {
                        let score = qsearch(ctx, game_state, alpha, beta, rep_stack);
                        if score == SEARCH_ABORTED {
                            if pushed_rep {
                                rep_stack.pop();
                            }
                            return SEARCH_ABORTED;
                        }
                        let should_prune = if maximizing {
                            score <= alpha
                        } else {
                            score >= beta
                        };
                        if should_prune {
                            if pushed_rep {
                                rep_stack.pop();
                            }
                            return score;
                        }
                    }
                    }
                }
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
            // Checkmate: the side to move is mated
            // Use White-perspective scoring: positive = good for White
            if to_move == Color::White {
                -MATE_VALUE + ply  // White is mated (very bad for White)
            } else {
                MATE_VALUE - ply   // Black is mated (very good for White)
            }
        } else {
            0  // Stalemate is a draw
        };
    }

    let ordered_moves = order_moves(moves, game_state, tt_move_hint, to_move, ply, heuristics, prev_move);
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
        prev_move,
        ctx,
        heuristics,
        singular_info,
        check_extension_budget,
    );

    // Pop repetition stack
    if pushed_rep {
        rep_stack.pop();
    }

    if result.best_value == SEARCH_ABORTED {
        return SEARCH_ABORTED;
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

// ============================================================
// MOVE SEARCH LOGIC
// ============================================================

struct MoveSearchResult {
    best_value: i32,
    best_from_to: Option<((usize, usize), (usize, usize))>,
}

#[derive(Copy, Clone)]
struct SingularInfo {
    mv: ((usize, usize), (usize, usize)),
    score: i32,
    depth: usize,
}

/// Search all moves and update alpha/beta bounds
fn search_moves(
    moves: Vec<((usize, usize), (usize, usize), Option<char>)>,
    game_state: &mut GameState,
    depth: usize,
    mut alpha: i32,
    mut beta: i32,
    ply: i32,
    tt: &TranspositionTable,
    rep_stack: &mut RepetitionStack,
    to_move: Color,
    prev_move: Option<((usize, usize), (usize, usize))>,
    ctx: &SearchContext,
    heuristics: &mut SearchHeuristics,
    singular_info: Option<SingularInfo>,
    check_extension_budget: u8,
) -> MoveSearchResult {
    let maximizing = to_move == Color::White;
    let mut current_value = initial_value(maximizing);
    let mut best_from_to: Option<((usize, usize), (usize, usize))> = None;
    let mut is_first_move = true;
    let mut move_index: i32 = 0;
    let history_bonus = (depth as i32) * (depth as i32);
    let in_check = game_state.mutable_board().is_side_in_check(to_move);
    let is_pv = beta.saturating_sub(alpha) > 1;
    let singular_candidate = if is_pv && !in_check && depth >= SINGULAR_EXTENSION_MIN_DEPTH && moves.len() > 1 {
        singular_info
    } else {
        None
    };

    #[cfg(feature = "debug-search")]
    let indent = "  ".repeat(ply as usize);

    #[cfg(feature = "debug-search")] {
        if ply <= 3 {
            eprintln!("{}[AB] searching {} moves at ply={} depth={}", indent, moves.len(), ply, depth);
        }
    }

    // Frontier futility pruning: compute static eval once if at depth=1
    let static_eval = if depth == 1 {
        Some(evaluate_position(game_state.board(), to_move))
    } else {
        None
    };

    for &(from, to, promo) in moves.iter() {
        let mut singular_extend = false;
        if let Some(singular) = singular_candidate
            && singular.mv == (from, to)
            && singular.depth + 1 >= depth
            && move_index == 0
        {
            singular_extend = is_singular_extension(
                ctx,
                game_state,
                depth,
                ply,
                tt,
                rep_stack,
                to_move,
                singular.mv,
                singular.score,
                &moves,
                prev_move,
            );
        }

        let u = game_state.make_move_fast(from, to, promo);

        // Passed-pawn extension
        let mut child_depth = depth.saturating_sub(1);
        let mut extended = false;
        let mut check_extension_used = false;
        let is_capture = u.ep_captured_piece.is_some() || u.board_undo.captured.is_some();

        {
            let b = game_state.board();
            if let Some(p) = b.get(to.0, to.1)
                && p.get_type() == PieceType::Pawn
                && b.game_phase_light() <= 8
                && b.is_passed_pawn_simple(to.0, to.1, p.get_color())
            {
                let adv = match p.get_color() {
                    Color::White => to.0 as i32,
                    Color::Black => (7 - to.0) as i32,
                };
                if adv >= 5 {
                    child_depth = child_depth.saturating_add(1);
                    extended = true;
                }
            }
        }

        let gives_check = game_state.mutable_board().is_side_in_check(opponent(to_move));
        let quiet = !is_capture && promo.is_none();
        let allow_reduce = !gives_check && !singular_extend;
        let captured_piece = u.board_undo.captured.or(u.ep_captured_piece);
        let moved_piece_type = u.board_undo.moved.map(|p| p.get_type());

        if singular_extend {
            child_depth = child_depth.saturating_add(1);
            extended = true;
        }

        if !extended && check_extension_budget > 0 && should_check_extend(depth, in_check, gives_check) {
            child_depth = child_depth.saturating_add(1);
            check_extension_used = true;
        }
        let next_check_extension_budget = if check_extension_used {
            check_extension_budget.saturating_sub(1)
        } else {
            check_extension_budget
        };

        if should_late_move_prune(depth, move_index, quiet, gives_check, in_check, is_pv) {
            game_state.unmake_move_fast(u);
            move_index += 1;
            continue;
        }

        // Frontier futility pruning: skip quiet moves at depth=1 that can't improve position
        if let Some(eval) = static_eval
            && quiet && !gives_check && !is_first_move {
                if maximizing && eval + FRONTIER_FUTILITY_MARGIN <= alpha {
                    game_state.unmake_move_fast(u);
                    move_index += 1;
                    continue;
                } else if !maximizing && eval - FRONTIER_FUTILITY_MARGIN >= beta {
                    game_state.unmake_move_fast(u);
                    move_index += 1;
                    continue;
                }
            }

        // Calculate LMR reduction
        let reduction = if is_first_move {
            0
        } else {
            calculate_lmr_reduction(
                child_depth, move_index, quiet, gives_check, allow_reduce,
                to_move, from, to, moved_piece_type, game_state.board(),
                game_state.board().game_phase_light(),
                captured_piece,
                heuristics,
                ply,
                prev_move,
            )
        };
        let reduced_depth = child_depth.saturating_sub(reduction);

        // Debug: show LMR reduction
        #[cfg(feature = "debug-search")]
        if ply <= 3 && reduction > 0 {
            let from_sq = crate::piece::as_square_str(from);
            let to_sq = crate::piece::as_square_str(to);
            eprintln!("{}  [LMR] {}{}: depth {} → {} (reduction={})",
                indent, from_sq, to_sq, child_depth, reduced_depth, reduction);
        }

        // Search with appropriate window
        let current_score = if is_first_move {
            // Full window on first move
            alphabeta(
                ctx,
                heuristics,
                game_state,
                child_depth,
                alpha,
                beta,
                ply + 1,
                tt,
                rep_stack,
                true,
                Some((from, to)),
                next_check_extension_budget,
            )
        } else {
            // Null-window search (PVS)
            let (nw_alpha, nw_beta) = null_window(alpha, beta, maximizing);

            #[cfg(feature = "debug-search")]
            if ply <= 2 {
                let from_sq = crate::piece::as_square_str(from);
                let to_sq = crate::piece::as_square_str(to);
                eprintln!("{}  [PVS] {}{}: null-window [{}, {}] depth={}",
                    indent, from_sq, to_sq, nw_alpha, nw_beta, reduced_depth);
            }

            let mut sc = alphabeta(
                ctx,
                heuristics,
                game_state,
                reduced_depth,
                nw_alpha,
                nw_beta,
                ply + 1,
                tt,
                rep_stack,
                true,
                Some((from, to)),
                next_check_extension_budget,
            );

            // Re-search with full window if needed
            // For PVS: re-search if score falls within (alpha, beta) after null-window search
            // This indicates the move might improve the bound and needs precise evaluation
            let needs_research = if maximizing {
                // White: re-search if score beats alpha but isn't a beta cutoff
                sc > alpha && sc < beta
            } else {
                // Black: re-search if score beats beta but isn't an alpha cutoff
                sc < beta && sc > alpha
            };
            // Also re-search if we used reduced depth (LMR) and got a promising score
            let lmr_research = reduced_depth < child_depth && is_better(sc, alpha, maximizing);

            if needs_research || lmr_research {
                #[cfg(feature = "debug-search")]
                if ply <= 2 {
                    let from_sq = crate::piece::as_square_str(from);
                    let to_sq = crate::piece::as_square_str(to);
                    eprintln!("{}  [PVS] {}{}: RE-SEARCH! score {} in window ({}, {})",
                        indent, from_sq, to_sq, sc, alpha, beta);
                }
                sc = alphabeta(
                    ctx,
                    heuristics,
                    game_state,
                    child_depth,
                    alpha,
                    beta,
                    ply + 1,
                    tt,
                    rep_stack,
                    true,
                    Some((from, to)),
                    next_check_extension_budget,
                );
            }
            sc
        };

        game_state.unmake_move_fast(u);
        if current_score == SEARCH_ABORTED {
            return MoveSearchResult {
                best_value: SEARCH_ABORTED,
                best_from_to: None,
            };
        }
        is_first_move = false;

        #[cfg(feature = "debug-search")]
        if ply <= 3 {
            let from_sq = crate::piece::as_square_str(from);
            let to_sq = crate::piece::as_square_str(to);
            let promo_str = promo.map(|c| c.to_string()).unwrap_or_default();
            eprintln!("{}  move {}{}{}: score={} α={} β={} best={}",
                indent, from_sq, to_sq, promo_str, current_score, alpha, beta, current_value);
        }

        // Track if this move improved the bound before updating
        let old_alpha = alpha;
        let old_beta = beta;

        // Update best value
        if is_better(current_score, current_value, maximizing) {
            current_value = current_score;
        }

        // Update bounds and track best move
        if maximizing {
            if current_value > alpha {
                #[cfg(feature = "debug-search")]
                if ply <= 3 {
                    let from_sq = crate::piece::as_square_str(from);
                    let to_sq = crate::piece::as_square_str(to);
                    eprintln!("{}  ★ NEW BEST: {}{} score={} (was α={})",
                        indent, from_sq, to_sq, current_value, alpha);
                }
                alpha = current_value;
                best_from_to = Some((from, to));
                if quiet {
                    heuristics.add_history(to_move, from, to, history_bonus);
                    if let Some((_, prev_to)) = prev_move {
                        heuristics.add_continuation_history(to_move, prev_to, from, to, history_bonus);
                    }
                }
            } else if quiet && current_score <= old_alpha {
                // Penalize quiet moves that failed to improve alpha
                let penalty = -history_bonus / 2;
                heuristics.add_history(to_move, from, to, penalty);
                if let Some((_, prev_to)) = prev_move {
                    heuristics.add_continuation_history(to_move, prev_to, from, to, penalty);
                }
            }
            if current_value >= beta {
                #[cfg(feature = "debug-search")]
                if ply <= 3 {
                    let from_sq = crate::piece::as_square_str(from);
                    let to_sq = crate::piece::as_square_str(to);
                    eprintln!("{}  ✂ BETA CUTOFF: {}{} score={} ≥ β={}",
                        indent, from_sq, to_sq, current_value, beta);
                }
                // Record killer move on beta cutoff for quiet moves
                if quiet {
                    heuristics.add_killer(ply as usize, from, to);
                    if let Some((prev_from, prev_to)) = prev_move {
                        heuristics.set_counter_move(to_move, prev_from, prev_to, from, to);
                    }
                }
                break; // Beta cutoff
            }
        } else {
            if current_value < beta {
                #[cfg(feature = "debug-search")]
                if ply <= 3 {
                    let from_sq = crate::piece::as_square_str(from);
                    let to_sq = crate::piece::as_square_str(to);
                    eprintln!("{}  ★ NEW BEST: {}{} score={} (was β={})",
                        indent, from_sq, to_sq, current_value, beta);
                }
                beta = current_value;
                best_from_to = Some((from, to));
                if quiet {
                    heuristics.add_history(to_move, from, to, history_bonus);
                    if let Some((_, prev_to)) = prev_move {
                        heuristics.add_continuation_history(to_move, prev_to, from, to, history_bonus);
                    }
                }
            } else if quiet && current_score >= old_beta {
                // Penalize quiet moves that failed to improve beta
                let penalty = -history_bonus / 2;
                heuristics.add_history(to_move, from, to, penalty);
                if let Some((_, prev_to)) = prev_move {
                    heuristics.add_continuation_history(to_move, prev_to, from, to, penalty);
                }
            }
            if current_value <= alpha {
                #[cfg(feature = "debug-search")]
                if ply <= 3 {
                    let from_sq = crate::piece::as_square_str(from);
                    let to_sq = crate::piece::as_square_str(to);
                    eprintln!("{}  ✂ ALPHA CUTOFF: {}{} score={} ≤ α={}",
                        indent, from_sq, to_sq, current_value, alpha);
                }
                // Record killer move on alpha cutoff for quiet moves
                if quiet {
                    heuristics.add_killer(ply as usize, from, to);
                    if let Some((prev_from, prev_to)) = prev_move {
                        heuristics.set_counter_move(to_move, prev_from, prev_to, from, to);
                    }
                }
                break; // Alpha cutoff
            }
        }

        move_index += 1;
    }

    #[cfg(feature = "debug-search")]
    if ply <= 3 {
        let best_str = best_from_to.map(|(f, t)| {
            format!("{}{}", crate::piece::as_square_str(f), crate::piece::as_square_str(t))
        }).unwrap_or_else(|| "none".to_string());
        eprintln!("{}[AB] search_moves done: best_value={} best_move={}", indent, current_value, best_str);
    }

    MoveSearchResult {
        best_value: current_value,
        best_from_to,
    }
}


// ============================================================
// MOVE ORDERING
// ============================================================

struct MoveOrderingContext {
    half_move_clock: u32,
    prev_move: Option<((usize, usize), (usize, usize))>,
}

#[inline]
fn razor_margin(depth: usize) -> i32 {
    match depth {
        1 => RAZOR_MARGIN_DEPTH_1,
        2 => RAZOR_MARGIN_DEPTH_2,
        _ => RAZOR_MARGIN_DEPTH_2,
    }
}

#[inline]
fn rfp_margin(depth: usize) -> i32 {
    match depth {
        2 => RFP_MARGIN_DEPTH_2,
        3 => RFP_MARGIN_DEPTH_3,
        _ => RFP_MARGIN_DEPTH_3,
    }
}

#[inline]
fn capture_see_order_score(
    board: &crate::board::Board,
    to_move: Color,
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
    is_en_passant: bool,
) -> i32 {
    let moved = match board.get(from.0, from.1) {
        Some(p) => p,
        None => return 0,
    };
    let cap_sq = if is_en_passant { Some((from.0, to.1)) } else { None };
    let captured = if let Some(sq) = cap_sq {
        board.get(sq.0, sq.1)
    } else {
        board.get(to.0, to.1)
    };
    let cap_val = captured.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);

    let mut post = *board;
    post.set(from.0, from.1, None);
    if let Some(sq) = cap_sq {
        post.set(sq.0, sq.1, None);
    }

    let mut moved_piece = moved;
    if let Some(pc) = promo {
        let pt = match pc {
            'q' | 'Q' => PieceType::Queen,
            'r' | 'R' => PieceType::Rook,
            'b' | 'B' => PieceType::Bishop,
            'n' | 'N' => PieceType::Knight,
            _ => moved_piece.get_type(),
        };
        moved_piece = Piece::new(pt, moved_piece.get_color());
    }
    post.set(to.0, to.1, Some(moved_piece));
    if moved_piece.get_type() == PieceType::King {
        post.set_king_location(moved_piece.get_color(), to);
    }

    see_dest_estimate(&post, to_move, to, cap_val)
}

/// Compute a heuristic score for move ordering
fn compute_move_order_score(
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
    moved_type: Option<PieceType>,
    is_capture: bool,
    mvv_lva: i32,
    see_score: i32,
    to_move: Color,
    ply: i32,
    ctx: &MoveOrderingContext,
    heuristics: &SearchHeuristics,
) -> i32 {
    let mut key = mvv_lva;
    if is_capture {
        key += (see_score * MOVE_ORDER_SEE_SCALE).clamp(-MOVE_ORDER_SEE_CAP, MOVE_ORDER_SEE_CAP);
    }

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
        if let Some((prev_from, prev_to)) = ctx.prev_move {
            if heuristics.is_counter_move(to_move, prev_from, prev_to, from, to) {
                key += MOVE_ORDER_COUNTER_BONUS;
            }
            let cont = heuristics.continuation_score(to_move, prev_to, from, to);
            key += (cont / MOVE_ORDER_CONTINUATION_DIV).clamp(-MOVE_ORDER_CONTINUATION_CAP, MOVE_ORDER_CONTINUATION_CAP);
        }
        if heuristics.is_killer(ply as usize, from, to) {
            key += 200_000;
        }
        let hist = heuristics.history_score(to_move, from, to);
        key += (hist / 32).clamp(-200_000, 200_000);
    }

    key
}

/// Order moves for better alpha-beta cutoffs
fn order_moves(
    moves: Vec<((usize, usize), (usize, usize), Option<char>)>,
    game_state: &GameState,
    tt_move_hint: Option<(u8, u8)>,
    to_move: Color,
    ply: i32,
    heuristics: &SearchHeuristics,
    prev_move: Option<((usize, usize), (usize, usize))>,
) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    if moves.is_empty() {
        return moves;
    }

    let mut ordered = moves;

    // Try TT move first (use pre-extracted hint, available even if depth was insufficient)
    if let Some((best_from, best_to)) = tt_move_hint {
        let bm = decode_move(best_from, best_to);
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
        prev_move,
    };

    // Collect piece info before sorting
    let piece_info: Vec<_> = {
        let b = game_state.board();
        let ep_target = game_state.en_passant_target();
        ordered
            .iter()
            .map(|&(from, to, promo)| {
                let moved = b.get(from.0, from.1);
                let moved_type = moved.map(|p| p.get_type());
                let is_ep = ep_target.is_some()
                    && ep_target == Some(to)
                    && moved_type == Some(PieceType::Pawn)
                    && b.get(to.0, to.1).is_none();
                let is_capture = b.get(to.0, to.1).is_some() || is_ep;
                let mvv_lva = if crate::search::core::advanced_search::MVV_LVA_ENABLED {
                    b.move_score_mvv_lva(from, to)
                } else {
                    0
                };
                let see_score = if is_capture {
                    capture_see_order_score(b, to_move, from, to, promo, is_ep)
                } else {
                    0
                };
                (moved_type, is_capture, mvv_lva, see_score)
            })
            .collect()
    };

    // Score all moves
    let mut moves_with_scores: Vec<_> = ordered
        .into_iter()
        .enumerate()
        .map(|(i, (from, to, promo))| {
            let (moved_type, is_capture, mvv_lva, see_score) = piece_info[i];
            let score = compute_move_order_score(
                from, to, promo, moved_type, is_capture, mvv_lva,
                see_score, to_move, ply, &ctx, heuristics,
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

/// Calculate the reduction depth for LMR using logarithmic scaling
pub(crate) fn calculate_lmr_reduction(
    child_depth: usize,
    move_index: i32,
    is_quiet: bool,
    gives_check: bool,
    allow_reduce: bool,
    to_move: Color,
    from: (usize, usize),
    to: (usize, usize),
    moved_pt: Option<PieceType>,
    board: &crate::board::Board,
    phase: i32,
    captured: Option<Piece>,
    heuristics: &SearchHeuristics,
    ply: i32,
    prev_move: Option<((usize, usize), (usize, usize))>,
) -> usize {
    if !crate::search::core::advanced_search::LMR_ENABLED {
        return 0;
    }

    // Avoid LMR in late endgames where tactics are sharper and depth is sparse.
    if phase <= 8 {
        return 0;
    }

    // Checking moves are tactical; avoid reductions.
    if gives_check {
        return 0;
    }

    // Quiet queen moves can be critical for mating nets; don't reduce them.
    if is_quiet && moved_pt == Some(PieceType::Queen) {
        return 0;
    }

    // Check if this is a bad capture (negative SEE) that should be reduced
    let is_bad_capture = if !is_quiet && captured.is_some() {
        let cap_val = captured.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
        let see = see_dest_estimate(board, to_move, to, cap_val);
        see < 0
    } else {
        false
    };

    // Only reduce quiet moves or bad captures after a few moves have been searched.
    // Avoid reducing at shallow depths to preserve tactical lines (e.g., mate-in-3).
    if (!is_quiet && !is_bad_capture) || child_depth < 4 || move_index < 3 || !allow_reduce {
        return 0;
    }

    // Base reduction using logarithmic formula: ln(depth) * ln(move_index) * C
    // This scales naturally with both depth and move ordering position
    let depth_f = (child_depth as f32).ln();
    let move_f = (move_index as f32 + 1.0).ln();
    let base_reduction = (depth_f * move_f * 0.5).floor() as usize;

    let mut r = base_reduction.max(1);

    if child_depth >= 10 && move_index >= 8 {
        r += 1;
    }
    if child_depth >= 14 && move_index >= 12 {
        r += 1;
    }

    let hist = heuristics.history_score(to_move, from, to);
    let mut cont = 0;
    let mut is_counter = false;
    if let Some((prev_from, prev_to)) = prev_move {
        is_counter = heuristics.is_counter_move(to_move, prev_from, prev_to, from, to);
        cont = heuristics.continuation_score(to_move, prev_to, from, to);
    }
    let is_killer = heuristics.is_killer(ply as usize, from, to);

    if is_killer || is_counter || cont > 8_000 {
        r = r.saturating_sub(1);
    }
    if is_good_history(hist) {
        r = r.saturating_sub(1);
    } else if hist < -10_000 {
        r += 1;
    }
    if cont < -8_000 {
        r += 1;
    }

    // Extra reduction for quiet queen moves in opening with undeveloped pieces
    if let Some(mp) = board.get(from.0, from.1)
        && mp.get_type() == PieceType::Queen && phase >= 12 {
        let back_r = if to_move == Color::White { 0 } else { 7 };
        let mut undeveloped = 0;

        for fc in 0..8 {
            if let Some(p) = board.get(back_r, fc)
                && p.get_color() == to_move && matches!(p.get_type(), PieceType::Knight | PieceType::Bishop) {
                undeveloped += 1;
            }
        }

        let king_file = (0..8).find(|&fc| {
            board.get(back_r, fc)
                .is_some_and(|p| p.get_color() == to_move && p.get_type() == PieceType::King)
        });
        let uncastled = king_file.is_none_or(|kf| kf != 2 && kf != 6);

        if undeveloped >= 3 {
            r += 2;
        } else if undeveloped >= 2 {
            r += 1;
        }

        if uncastled {
            r += 1;
        }
    }

    // Cap reduction to avoid searching too shallow
    // Never reduce more than depth - 1 (must search at least 1 ply)
    r.min(child_depth.saturating_sub(1)).min(5)
}

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
fn is_good_history(hist: i32) -> bool {
    // History scores are stored per-side and are always positive for good moves
    hist > 10_000
}

#[inline]
fn opponent(color: Color) -> Color {
    match color {
        Color::White => Color::Black,
        Color::Black => Color::White,
    }
}

#[inline]
pub(crate) fn should_late_move_prune(
    depth: usize,
    move_index: i32,
    is_quiet: bool,
    gives_check: bool,
    in_check: bool,
    is_pv: bool,
) -> bool {
    if depth == 0 || depth > LMP_MAX_DEPTH {
        return false;
    }
    if in_check || is_pv || !is_quiet || gives_check {
        return false;
    }
    let limit = LMP_QUIET_BASE + (depth as i32 * LMP_QUIET_PER_DEPTH);
    move_index >= limit
}

#[inline]
pub(crate) fn should_check_extend(depth: usize, in_check: bool, gives_check: bool) -> bool {
    if depth == 0 || depth > CHECK_EXTENSION_MAX_DEPTH {
        return false;
    }
    in_check || gives_check
}

#[inline]
pub(crate) fn is_singular_extension(
    ctx: &SearchContext,
    game_state: &mut GameState,
    depth: usize,
    ply: i32,
    tt: &TranspositionTable,
    rep_stack: &mut RepetitionStack,
    to_move: Color,
    best_move: ((usize, usize), (usize, usize)),
    best_score: i32,
    moves: &Vec<((usize, usize), (usize, usize), Option<char>)>,
    _prev_move: Option<((usize, usize), (usize, usize))>,
) -> bool {
    if depth < SINGULAR_EXTENSION_MIN_DEPTH || moves.len() <= 1 {
        return false;
    }

    let verify_depth = depth.saturating_sub(1 + SINGULAR_SEARCH_REDUCTION);
    if verify_depth == 0 {
        return false;
    }

    let maximizing = to_move == Color::White;
    let target = if maximizing {
        best_score.saturating_sub(SINGULAR_MARGIN)
    } else {
        best_score.saturating_add(SINGULAR_MARGIN)
    };
    let (a, b) = if maximizing {
        (target.saturating_sub(1), target)
    } else {
        (target, target.saturating_add(1))
    };

    let mut local_heuristics = SearchHeuristics::new(128);
    let mut checked = 0usize;

    for &(from, to, promo) in moves.iter() {
        if (from, to) == best_move {
            continue;
        }
        let u = game_state.make_move_fast(from, to, promo);
        let sc = alphabeta(
            ctx,
            &mut local_heuristics,
            game_state,
            verify_depth,
            a,
            b,
            ply + 1,
            tt,
            rep_stack,
            false,
            Some((from, to)),
            0,
        );
        game_state.unmake_move_fast(u);
        if sc == SEARCH_ABORTED {
            return false;
        }
        if maximizing {
            if sc >= target {
                return false;
            }
        } else if sc <= target {
            return false;
        }
        checked += 1;
        if checked >= SINGULAR_ALT_LIMIT {
            break;
        }
    }

    checked > 0
}

#[inline]
fn has_checking_move(game_state: &mut GameState, side: Color) -> bool {
    let moves = find_all_valid_moves(game_state);
    for &(from, to, promo) in moves.iter() {
        let u = game_state.make_move_fast(from, to, promo);
        let gives_check = game_state.mutable_board().is_side_in_check(opponent(side));
        game_state.unmake_move_fast(u);
        if gives_check {
            return true;
        }
    }
    false
}

