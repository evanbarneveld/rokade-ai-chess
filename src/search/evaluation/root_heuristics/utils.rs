//! Common utilities and constants for root heuristics.

use crate::board::Board;
use crate::piece::pieces::{Color, Piece, PieceType};

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
// Increased to make giving check more valuable in close positions
pub const CHECK_TIEBREAK_BASE: i32 = 10;
pub const CHECK_MOBILITY_BONUS_0: i32 = 50;
pub const CHECK_MOBILITY_BONUS_1_2: i32 = 20;
pub const CHECK_MOBILITY_BONUS_3_5: i32 = 10;

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
pub fn simulate_move(
    board: &Board,
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
) -> (Board, Option<Piece>) {
    let mut b = *board;
    let moved = board.get(from.0, from.1);
    if let Some(mut p) = moved {
        let is_pawn = p.get_type() == PieceType::Pawn;
        let is_king = p.get_type() == PieceType::King;

        // En passant capture: pawn moved diagonally to an empty square.
        if is_pawn && from.1 != to.1 && b.get(to.0, to.1).is_none() {
            let cap_sq = (from.0, to.1);
            b.set(cap_sq.0, cap_sq.1, None);
        }

        // Castling: move rook to the correct square.
        if is_king && from.1 == 4 && from.0 == to.0 {
            if to.1 == 6 {
                let rf = (from.0, 7);
                let rt = (from.0, 5);
                if let Some(r) = b.get(rf.0, rf.1)
                    && r.get_type() == PieceType::Rook {
                        b.set(rt.0, rt.1, Some(r));
                        b.set(rf.0, rf.1, None);
                    }
            } else if to.1 == 2 {
                let rf = (from.0, 0);
                let rt = (from.0, 3);
                if let Some(r) = b.get(rf.0, rf.1)
                    && r.get_type() == PieceType::Rook {
                        b.set(rt.0, rt.1, Some(r));
                        b.set(rf.0, rf.1, None);
                    }
            }
        }

        // Promotion: upgrade pawn when reaching the last rank.
        if is_pawn {
            let promote = match p.get_color() {
                Color::White => to.0 == 7,
                Color::Black => to.0 == 0,
            };
            if promote {
                let pt = match promo {
                    Some('q') => PieceType::Queen,
                    Some('r') => PieceType::Rook,
                    Some('b') => PieceType::Bishop,
                    Some('n') => PieceType::Knight,
                    _ => PieceType::Queen,
                };
                p = Piece::new(pt, p.get_color());
            }
        }

        b.set(from.0, from.1, None);
        b.set(to.0, to.1, Some(p));
        if is_king {
            b.set_king_location(p.get_color(), to);
        }
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
