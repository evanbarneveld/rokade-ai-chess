//! Root move evaluation and PV construction.
//!
//! This module provides the public API for root-level move evaluation,
//! delegating to specialized heuristic modules for score adjustments.

use crate::board::Board;
use crate::history::history::History;
use crate::piece::pieces::{capture_value_cp, opposite_color, Color, PieceType};
use crate::search::alphabeta::alphabeta;
use crate::search::advanced_search::find_all_valid_moves;
use crate::search::see::apply_destination_see_penalties;
use crate::search::tt::{decode_move, TranspositionTable};
use crate::search::zobrist::compute_zobrist;
use crate::state::game_state::GameState;

// Import heuristics from sub-modules
use crate::search::root_heuristics::{
    simulate_move,
    knight_evacuations_priority,
    threat_resolution_and_evacuation,
    endgame_50move_scaling,
    king_safety_root_heuristics,
    self_hang_or_check_mobility,
    queen_kingside_pressure_bonus,
};
use crate::search::root_heuristics::utils::ROOT_CAPTURE_BONUS_DIV;

// ============================================================
// PUBLIC API: ROOT MOVE BONUSES & SCORING
// ============================================================

/// Build the principal variation line from transposition table.
#[inline]
pub fn build_pv_for_root(
    board: &Board,
    root_side: Color,
    from: (usize, usize),
    to: (usize, usize),
    root_promo: Option<char>,
    tt: &TranspositionTable,
    max_len: usize,
) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    let mut pv: Vec<((usize, usize), (usize, usize), Option<char>)> = Vec::with_capacity(max_len.max(1));
    pv.push((from, to, root_promo));

    let mut tmp = board.clone();
    let _undo = tmp.make_move_simple(from, to, root_promo);
    let mut side = opposite_color(root_side);

    for _ in 1..max_len {
        let key = compute_zobrist(&tmp, side);
        let Some(entry) = tt.probe(key) else {
            break;
        };
        let (bf, bt) = (entry.best_from, entry.best_to);
        let ((nfr, nfc), (ntr, ntc)) = decode_move(bf, bt);

        let gs = GameState::from_board_and_side(tmp.clone(), side);
        let legals = find_all_valid_moves(&gs);
        let found_move = legals.iter().find(|(f, t, _)| (*f, *t) == ((nfr, nfc), (ntr, ntc)));

        if let Some(&(f, t, p)) = found_move {
            pv.push((f, t, p));
            let _u = tmp.make_move_simple(f, t, p);
            side = opposite_color(side);
        } else {
            break;
        }
    }
    pv
}

/// Collect root moves from a move list.
pub fn get_root_moves(
    _game_state: &GameState,
    _history: &History,
    _board: &Board,
    _active_color: Color,
    moves: &Vec<((usize, usize), (usize, usize), Option<char>)>,
    v: &mut Vec<((usize, usize), (usize, usize), Option<char>)>,
) {
    for &(from, to, promo) in moves {
        v.push((from, to, promo));
    }
}

/// Small development/centralization bonus for knights and bishops.
pub fn root_move_bonus(board: &Board, from: (usize, usize), to: (usize, usize), side: Color) -> i32 {
    let piece = match board.get(from.0, from.1) {
        Some(p) => p,
        None => return 0,
    };
    let pt = piece.get_type();
    let (tr, tc) = to;
    let mut bonus: i32 = 0;

    // Knights to c3/f3 (White) or c6/f6 (Black)
    if pt == PieceType::Knight {
        match side {
            Color::White => {
                if (tr, tc) == (2, 2) || (tr, tc) == (2, 5) {
                    bonus += 5;
                }
            }
            Color::Black => {
                if (tr, tc) == (5, 2) || (tr, tc) == (5, 5) {
                    bonus += 5;
                }
            }
        }
    }

    // Bishops to c4/f4 (White) or c5/f5 (Black)
    if pt == PieceType::Bishop {
        match side {
            Color::White => {
                if (tr, tc) == (3, 2) || (tr, tc) == (3, 5) {
                    bonus += 3;
                }
            }
            Color::Black => {
                if (tr, tc) == (4, 2) || (tr, tc) == (4, 5) {
                    bonus += 3;
                }
            }
        }
    }

    match side {
        Color::White => bonus,
        Color::Black => -bonus,
    }
}

/// Main orchestrator: adjust raw search score with root-level heuristics.
///
/// Applies the following adjustments in order:
/// 1. Development/centralization bonus
/// 2. SEE destination penalties
/// 3. Threat resolution and evacuation
/// 4. Knight evacuation priority
/// 5. Capture bonus
/// 6. Endgame / 50-move scaling
/// 7. King safety
/// 8. Self-hanging penalty / check mobility bonus
/// 9. Queen kingside pressure
#[inline]
pub fn adjust_root_score(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    base_hmc: u32,
    is_capture: bool,
    moved_is_pawn: bool,
    score_raw: i32,
) -> i32 {
    let mut adjusted = score_raw + root_move_bonus(base_board, from, to, side);

    // Prepare post-position for heuristics
    let (mut post_after, moved_probe) = simulate_move(base_board, from, to);
    let opp = opposite_color(side);
    let gives_check = post_after.is_side_in_check(opp);
    let moved_is_queen = moved_probe
        .map(|p| p.get_type() == PieceType::Queen)
        .unwrap_or(false);

    // 1. SEE penalties
    adjusted += apply_destination_see_penalties(
        base_board, &post_after, side, from, to,
        is_capture, moved_is_pawn, gives_check, moved_is_queen,
    );

    // 2. Threat resolution and evacuation
    adjusted += threat_resolution_and_evacuation(
        base_board, &post_after, side, from, to, gives_check,
    );

    // 3. Knight evacuation priority
    adjusted += knight_evacuations_priority(base_board, side, from, to, gives_check);

    // 4. Capture bonus
    if let Some(captured) = base_board.get(to.0, to.1) {
        adjusted += capture_value_cp(captured.get_type()) / ROOT_CAPTURE_BONUS_DIV;
    }

    // 5. Endgame / 50-move scaling
    adjusted += endgame_50move_scaling(side, score_raw, base_hmc, is_capture, moved_is_pawn);

    // 6. King safety
    adjusted += king_safety_root_heuristics(base_board, side, from, to, is_capture);

    // 7. Self-hang or check mobility
    adjusted += self_hang_or_check_mobility(
        base_board, &post_after, side, from, to, gives_check, opp,
    );

    // 8. Queen kingside pressure
    adjusted += queen_kingside_pressure_bonus(base_board, side, from, to);

    adjusted
}

// ============================================================
// PUBLIC API: EVALUATION WRAPPERS
// ============================================================

/// Evaluate a position after making a root move.
#[inline]
pub fn evaluate_after_root_move(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
    depth_now: usize,
    a: i32,
    b: i32,
    tt: &mut TranspositionTable,
    base_hmc: u32,
    history: &History,
) -> (i32, bool, bool) {
    let mut tmp = base_board.clone();
    let u = tmp.make_move_simple(from, to, promo);
    let moved_is_pawn = base_board
        .get(from.0, from.1)
        .map(|p| p.get_type() == PieceType::Pawn)
        .unwrap_or(false);
    let is_capture = base_board.get(to.0, to.1).is_some();
    let child_hmc: u32 = if is_capture || moved_is_pawn {
        0
    } else {
        base_hmc.saturating_add(1)
    };

    let gives_check = tmp.is_side_in_check(opposite_color(side));
    let score_raw = if depth_now <= 1 {
        // Shallow depth: use qsearch to avoid horizon blunders
        crate::search::qsearch::qsearch(
            &mut tmp,
            opposite_color(side),
            crate::search::advanced_search::MIN_EVAL_VALUE + 1,
            crate::search::advanced_search::MAX_EVAL_VALUE - 1,
            child_hmc,
            &mut history.get_rep_stack(),
        )
    } else {
        let mut rep_stack = history.get_rep_stack();
        let ext = if gives_check { 2 } else { 0 };
        alphabeta(
            &mut tmp,
            opposite_color(side),
            depth_now - 1 + ext,
            a,
            b,
            1,
            tt,
            child_hmc,
            &mut rep_stack,
        )
    };
    tmp.unmake_move_simple(u);
    (score_raw, is_capture, moved_is_pawn)
}

/// Get adjusted evaluation for a root move, with safety clamping.
#[inline]
pub fn adjusted_root_eval_for_move(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    base_hmc: u32,
    score_raw: i32,
    is_capture: bool,
    moved_is_pawn: bool,
) -> i32 {
    let mut adj = adjust_root_score(
        base_board, side, from, to, base_hmc, is_capture, moved_is_pawn, score_raw,
    );

    // Safety: don't let heuristics turn a losing move into a winning one
    if score_raw < 0 {
        if side == Color::White {
            adj = adj.min(score_raw);
        } else {
            adj = adj.max(score_raw);
        }
    }

    // Don't drag draw scores down
    if score_raw == 0 {
        adj = adj.max(-10);
    }
    adj
}
