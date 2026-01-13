//! Common utilities and constants for root heuristics.

use crate::board::Board;
use crate::piece::pieces::{Color, Piece};

// ============================================================
// CONSTANTS
// ============================================================

// Root capture scoring
pub const ROOT_CAPTURE_BONUS_DIV: i32 = 10;

// Endgame / 50-move rule thresholds
pub const ENDGAME_SIDEADV_THRESHOLD_CP: i32 = 150;
pub const ENDGAME_HMC_THRESHOLD: u32 = 80;
pub const ENDGAME_SCALE_MAX: i32 = 21;
pub const ENDGAME_CAPTURE_SCALE_BONUS_CP: i32 = 15;
pub const ENDGAME_NONCAP_SCALE_PENALTY_CP: i32 = 8;

// Check and mobility bonuses (by opponent reply count)
pub const CHECK_TIEBREAK_BASE: i32 = 1;
pub const CHECK_MOBILITY_BONUS_0: i32 = 5;
pub const CHECK_MOBILITY_BONUS_1_2: i32 = 2;
pub const CHECK_MOBILITY_BONUS_3_5: i32 = 1;

// King safety
pub const KING_CAPTURE_ROOT_PENALTY: i32 = 5;

// Knight evacuation heuristics
pub const KNIGHT_IGNORE_PAWN_THREAT_PENALTY: i32 = 4;
pub const KNIGHT_NON_EVAC_DEMOTION: i32 = 6;
pub const KNIGHT_SAFE_EVAC_REWARD: i32 = 3;
pub const KNIGHT_SAFE_TO_SPECIFIC_REWARD: i32 = 4;
pub const KNIGHT_CENTER_EXTRA_D4: i32 = 1;
pub const KNIGHT_CENTER_STEP: i32 = 0;

// Center squares (d4, d5, e4, e5)
pub const CENTER_SQUARES: [(usize, usize); 4] = [(3, 3), (3, 4), (4, 3), (4, 4)];

// ============================================================
// GENERIC BOARD UTILITIES
// ============================================================

/// Apply sign based on side (positive for White, negative for Black).
#[inline]
pub fn apply_for_side(v: i32, side: Color) -> i32 {
    if side == Color::White { v } else { -v }
}

/// Simulate a move on a cloned board, returning the new board and the moved piece.
#[inline]
pub fn simulate_move(board: &Board, from: (usize, usize), to: (usize, usize)) -> (Board, Option<Piece>) {
    let mut b = *board;
    let moved = board.get(from.0, from.1);
    b.set(from.0, from.1, None);
    if let Some(p) = moved {
        b.set(to.0, to.1, Some(p));
    }
    (b, moved)
}

/// Compute a center-proximity score for a square (higher = closer to center).
#[inline]
pub fn center_score((r, c): (usize, usize)) -> i32 {
    CENTER_SQUARES.iter().map(|&(cr, cc)| {
        let dr = r.abs_diff(cr);
        let dc = c.abs_diff(cc);
        (60 - 10 * ((dr + dc) as i32)).max(0)
    }).max().unwrap_or(0)
}
