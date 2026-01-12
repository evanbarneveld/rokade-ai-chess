use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::history::history::History;
use crate::piece::pieces::{capture_value_cp, opposite_color, Color, Piece, PieceType};
use crate::search::alphabeta::alphabeta;
use crate::search::advanced_search::find_all_valid_moves;
use crate::search::see::{
    apply_destination_see_penalties, attacked_by_pawn, see_dest_estimate,
    SEE_PENALTY_MAX_CP, SEE_PENALTY_MIN_CP,
};
use crate::search::tt::{decode_move, TranspositionTable};
use crate::search::zobrist::compute_zobrist;
use crate::state::game_state::GameState;

// ============================================================
// CONSTANTS
// ============================================================

// Root capture scoring
const ROOT_CAPTURE_BONUS_DIV: i32 = 10;

// Endgame / 50-move rule thresholds
const ENDGAME_SIDEADV_THRESHOLD_CP: i32 = 150;
const ENDGAME_HMC_THRESHOLD: u32 = 80;
const ENDGAME_SCALE_MAX: i32 = 21;
const ENDGAME_CAPTURE_SCALE_BONUS_CP: i32 = 15;
const ENDGAME_NONCAP_SCALE_PENALTY_CP: i32 = 8;

// Check and mobility bonuses (by opponent reply count)
const CHECK_TIEBREAK_BASE: i32 = 1;
const CHECK_MOBILITY_BONUS_0: i32 = 5;
const CHECK_MOBILITY_BONUS_1_2: i32 = 2;
const CHECK_MOBILITY_BONUS_3_5: i32 = 1;

// King safety
const KING_CAPTURE_ROOT_PENALTY: i32 = 5;

// Knight evacuation heuristics
const KNIGHT_IGNORE_PAWN_THREAT_PENALTY: i32 = 4;
const KNIGHT_NON_EVAC_DEMOTION: i32 = 6;
const KNIGHT_SAFE_EVAC_REWARD: i32 = 3;
const KNIGHT_SAFE_TO_SPECIFIC_REWARD: i32 = 4;
const KNIGHT_CENTER_EXTRA_D4: i32 = 1;
const KNIGHT_CENTER_STEP: i32 = 0;

// Center squares (d4, d5, e4, e5)
const CENTER_SQUARES: [(usize, usize); 4] = [(3, 3), (3, 4), (4, 3), (4, 4)];

// ============================================================
// GENERIC BOARD UTILITIES
// ============================================================

/// Apply sign based on side (positive for White, negative for Black).
#[inline]
fn apply_for_side(v: i32, side: Color) -> i32 {
    if side == Color::White { v } else { -v }
}

/// Simulate a move on a cloned board, returning the new board and the moved piece.
#[inline]
fn simulate_move(board: &Board, from: (usize, usize), to: (usize, usize)) -> (Board, Option<Piece>) {
    let mut b = board.clone();
    let moved = board.get(from.0, from.1);
    b.set(from.0, from.1, None);
    if let Some(p) = moved {
        b.set(to.0, to.1, Some(p));
    }
    (b, moved)
}

/// Compute a center-proximity score for a square (higher = closer to center).
#[inline]
fn center_score((r, c): (usize, usize)) -> i32 {
    CENTER_SQUARES.iter().map(|&(cr, cc)| {
        let dr = if r > cr { r - cr } else { cr - r };
        let dc = if c > cc { c - cc } else { cc - c };
        (60 - 10 * ((dr + dc) as i32)).max(0)
    }).max().unwrap_or(0)
}

// ============================================================
// KNIGHT-SPECIFIC HEURISTICS
// ============================================================

/// Compute safe squares a knight can move to from a given position.
#[inline]
fn knight_safe_squares(board: &Board, side: Color, from: (usize, usize)) -> Vec<(usize, usize)> {
    const DELTAS: [(isize, isize); 8] = [
        (2, 1), (2, -1), (-2, 1), (-2, -1),
        (1, 2), (1, -2), (-1, 2), (-1, -2)
    ];
    let (fr, fc) = from;
    let mut v = Vec::with_capacity(8);
    for (dr, dc) in DELTAS {
        let (nr, nc) = (fr as isize + dr, fc as isize + dc);
        if !(0..=7).contains(&nr) || !(0..=7).contains(&nc) {
            continue;
        }
        let (nr, nc) = (nr as usize, nc as usize);
        if let Some(occ) = board.get(nr, nc) {
            if occ.get_color() == side {
                continue;
            }
        }
        let (sim, _) = simulate_move(board, from, (nr, nc));
        let mut tmp = sim.clone();
        if !is_square_attacked_by_opponent(&mut tmp, (nr, nc), side)
            || see_dest_estimate(&sim, side, (nr, nc), 0) >= 0
        {
            v.push((nr, nc));
        }
    }
    v
}

/// Priority adjustment for knight evacuations from pawn threats.
#[inline]
fn knight_evacuations_priority(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    gives_check: bool,
) -> i32 {
    if gives_check {
        return 0;
    }
    let opp = opposite_color(side);

    // Find all our knights attacked by pawns
    let mut attacked_knights: Vec<(usize, usize)> = Vec::new();
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = base_board.get(r, c) {
                if p.get_color() == side && p.get_type() == PieceType::Knight {
                    if attacked_by_pawn(base_board, (r, c), opp) {
                        attacked_knights.push((r, c));
                    }
                }
            }
        }
    }
    if attacked_knights.is_empty() {
        return 0;
    }

    let mut delta = 0;

    // Penalize moves that don't evacuate an attacked knight
    if !attacked_knights.contains(&from) {
        delta -= apply_for_side(500, side);
    } else if let Some(p) = base_board.get(from.0, from.1) {
        if p.get_type() == PieceType::Knight {
            let (tr, tc) = to;
            let (sim, _) = simulate_move(base_board, from, to);
            let mut tmp = sim.clone();
            let dest_attacked = is_square_attacked_by_opponent(&mut tmp, (tr, tc), side);
            let see1 = see_dest_estimate(&sim, side, (tr, tc), 0);

            if !dest_attacked || see1 >= 0 {
                // Safe evacuation bonus with center preference
                let mut evac = KNIGHT_SAFE_EVAC_REWARD;
                for &(cr, cc) in &CENTER_SQUARES {
                    let dr = if tr > cr { tr - cr } else { cr - tr };
                    let dc = if tc > cc { tc - cc } else { cc - tc };
                    let dist = (dr + dc) as i32;
                    evac += (20 - KNIGHT_CENTER_STEP * dist).max(0);
                }
                // Extra bonus for d4 square
                if (tr, tc) == (3, 3) {
                    evac += KNIGHT_CENTER_EXTRA_D4;
                }
                delta += apply_for_side(evac, side);
            } else {
                delta -= apply_for_side(150, side);
            }
        }
    }
    delta
}

// ============================================================
// THREAT RESOLUTION & EVACUATION
// ============================================================

/// Handle threat resolution and piece evacuation heuristics.
#[inline]
fn threat_resolution_and_evacuation(
    base_board: &Board,
    post_after: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    gives_check: bool,
) -> i32 {
    if gives_check {
        return 0;
    }

    let mut base_clone = base_board.clone();
    let opp = opposite_color(side);

    // Find all our threatened pieces
    let mut threatened: Vec<(usize, usize, PieceType, bool)> = Vec::new();
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = base_board.get(r, c) {
                if p.get_color() != side {
                    continue;
                }
                if !is_square_attacked_by_opponent(&mut base_clone, (r, c), side) {
                    continue;
                }
                let pawn_attacks = attacked_by_pawn(base_board, (r, c), opp);
                threatened.push((r, c, p.get_type(), pawn_attacks));
            }
        }
    }
    if threatened.is_empty() {
        return 0;
    }

    let mut delta = 0;
    for (tr, tc, pt, by_pawn) in threatened {
        // Precompute knight safe squares if applicable
        let knight_safe: Vec<(usize, usize)> = if pt == PieceType::Knight && by_pawn {
            knight_safe_squares(base_board, side, (tr, tc))
        } else {
            Vec::new()
        };

        // Check if piece is still attacked after our move
        let still_attacked = if (tr, tc) == from {
            let mut tmpmv = post_after.clone();
            is_square_attacked_by_opponent(&mut tmpmv, to, side)
        } else if post_after.get(tr, tc).is_none() {
            false
        } else {
            let mut tmp2 = post_after.clone();
            is_square_attacked_by_opponent(&mut tmp2, (tr, tc), side)
        };

        if (tr, tc) == from {
            // We moved the threatened piece - calculate evacuation bonus
            let mut evac_bonus = 0;
            let see_new = see_dest_estimate(post_after, side, to, 0);
            if !still_attacked || see_new >= 0 {
                evac_bonus += 400;
            }

            // Knight-specific center bonus
            if pt == PieceType::Knight {
                let mut cb = center_score(to);
                if to == (3, 3) {
                    cb += 80;
                }
                if !knight_safe.is_empty() && knight_safe.iter().any(|&sq| sq == to) {
                    cb += 80;
                }
                evac_bonus += cb.max(0);
            }

            // Bonus for evacuating to a known safe square
            if pt == PieceType::Knight && by_pawn && !knight_safe.is_empty() {
                if knight_safe.iter().any(|&sq| sq == to) {
                    evac_bonus += KNIGHT_SAFE_TO_SPECIFIC_REWARD;
                }
            }
            delta += apply_for_side(evac_bonus, side);
        } else {
            // We did NOT move the threatened piece
            if pt == PieceType::Knight && by_pawn && !knight_safe.is_empty() {
                delta -= apply_for_side(KNIGHT_IGNORE_PAWN_THREAT_PENALTY, side);
            }
            if still_attacked {
                let pen = match pt {
                    PieceType::Knight | PieceType::Bishop => 200,
                    PieceType::Rook => 120,
                    PieceType::Queen => 80,
                    PieceType::Pawn => 40,
                    PieceType::King => 400,
                };
                let val = if by_pawn { pen + 400 } else { pen };
                delta -= apply_for_side(val, side);
            }
        }

        // Additional knight demotion if not evacuating
        if pt == PieceType::Knight && by_pawn && !knight_safe.is_empty() {
            if (tr, tc) != from {
                delta -= apply_for_side(KNIGHT_NON_EVAC_DEMOTION, side);
            } else if knight_safe.iter().any(|&sq| sq == to) {
                delta += apply_for_side(KNIGHT_SAFE_TO_SPECIFIC_REWARD, side);
            }
        }
    }
    delta
}

// ============================================================
// ENDGAME & KING SAFETY
// ============================================================

/// Apply endgame / 50-move rule scaling adjustments.
#[inline]
fn endgame_50move_scaling(
    side: Color,
    score_raw: i32,
    base_hmc: u32,
    is_capture: bool,
    moved_is_pawn: bool,
) -> i32 {
    let side_adv = apply_for_side(score_raw, side);
    if side_adv > ENDGAME_SIDEADV_THRESHOLD_CP && base_hmc >= ENDGAME_HMC_THRESHOLD {
        let scale = (base_hmc as i32 - (ENDGAME_HMC_THRESHOLD as i32 - 1)).min(ENDGAME_SCALE_MAX);
        if is_capture || moved_is_pawn {
            ENDGAME_CAPTURE_SCALE_BONUS_CP * scale
        } else {
            -ENDGAME_NONCAP_SCALE_PENALTY_CP * scale
        }
    } else {
        0
    }
}

/// Apply king safety heuristics for king moves.
#[inline]
fn king_safety_root_heuristics(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    is_capture: bool,
) -> i32 {
    let moved_is_king = base_board
        .get(from.0, from.1)
        .map(|p| p.get_type() == PieceType::King)
        .unwrap_or(false);
    if !moved_is_king {
        return 0;
    }

    let (mut postk, _) = simulate_move(base_board, from, to);
    let mut delta = 0;

    if is_square_attacked_by_opponent(&mut postk, to, side) {
        delta -= 50;
    }
    if is_capture {
        delta -= KING_CAPTURE_ROOT_PENALTY;
    }
    delta
}

// ============================================================
// CHECK & QUEEN HEURISTICS
// ============================================================

/// Calculate check mobility bonus based on opponent's available replies.
#[inline]
fn check_mobility_bonus_for_side(post_after: &Board, checked_side: Color) -> i32 {
    let opp_state = GameState::from_board_and_side(post_after.clone(), checked_side);
    let replies = find_all_valid_moves(&opp_state).len() as i32;
    match replies {
        0 => CHECK_MOBILITY_BONUS_0,
        1..=2 => CHECK_MOBILITY_BONUS_1_2,
        3..=5 => CHECK_MOBILITY_BONUS_3_5,
        _ => 0,
    }
}

/// Self-hanging penalty aggregate OR check tie-break with mobility bonus.
#[inline]
fn self_hang_or_check_mobility(
    _base_board: &Board,
    post_after: &Board,
    side: Color,
    _from: (usize, usize),
    _to: (usize, usize),
    gives_check: bool,
    opp: Color,
) -> i32 {
    // Scan our pieces for hanging penalties
    let mut total_penalty: i32 = 0;
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = post_after.get(r, c) {
                if p.get_color() != side {
                    continue;
                }
                let mut post_for_query = post_after.clone();
                if !is_square_attacked_by_opponent(&mut post_for_query, (r, c), side) {
                    continue;
                }
                let see = see_dest_estimate(post_after, side, (r, c), 0);
                if see < 0 {
                    let pen = (-see).clamp(SEE_PENALTY_MIN_CP, SEE_PENALTY_MAX_CP) / 2;
                    total_penalty += pen;
                    if p.get_type() == PieceType::Queen {
                        let q_extra = ((-see) * 12).clamp(40, 120);
                        total_penalty += q_extra;
                    }
                }
            }
        }
    }

    let agg_cap: i32 = 1000;
    let hang_pen = if total_penalty > 0 {
        -total_penalty.min(agg_cap)
    } else {
        0
    };

    // Check bonus
    let mut check_bonus = 0;
    if gives_check {
        check_bonus += CHECK_TIEBREAK_BASE;
        check_bonus += check_mobility_bonus_for_side(post_after, opp);
    }

    apply_for_side(hang_pen + check_bonus, side)
}

/// Bonus for queen attacking kingside squares (f2/h2 or f7/h7).
#[inline]
fn queen_kingside_pressure_bonus(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
) -> i32 {
    match base_board.get(from.0, from.1) {
        Some(p) if p.get_type() == PieceType::Queen => {}
        _ => return 0,
    }

    let (post, _) = simulate_move(base_board, from, to);
    let targets: &[(usize, usize)] = if side == Color::White {
        &[(1, 5), (1, 7)]
    } else {
        &[(6, 5), (6, 7)]
    };

    let mut hit_count = 0;
    for &sq in targets {
        let mut tmp = post.clone();
        let active_for_query = opposite_color(side);
        if is_square_attacked_by_opponent(&mut tmp, sq, active_for_query) {
            hit_count += 1;
        }
    }

    if hit_count > 0 {
        let bonus = match hit_count {
            1 => 1,
            _ => 2,
        };
        apply_for_side(bonus, side)
    } else {
        0
    }
}

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
