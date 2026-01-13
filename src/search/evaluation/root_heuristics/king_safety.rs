//! King safety heuristics for root move evaluation.

use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::piece::pieces::{Color, PieceType};

use super::utils::{simulate_move, KING_CAPTURE_ROOT_PENALTY};

/// Apply king safety heuristics for king moves.
#[inline]
pub fn king_safety_root_heuristics(
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
