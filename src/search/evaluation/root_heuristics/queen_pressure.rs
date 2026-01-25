//! Queen kingside pressure heuristics.

use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::piece::pieces::{opposite_color, Color, PieceType};

use super::utils::{apply_for_side, simulate_move};

/// Bonus for queen attacking kingside squares (f2/h2 or f7/h7).
#[inline]
pub fn queen_kingside_pressure_bonus(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
) -> i32 {
    match base_board.get(from.0, from.1) {
        Some(p) if p.get_type() == PieceType::Queen => {}
        _ => return 0,
    }

    let (post, _) = simulate_move(base_board, from, to, None);
    let targets: &[(usize, usize)] = if side == Color::White {
        &[(6, 5), (6, 7)]
    } else {
        &[(1, 5), (1, 7)]
    };

    let mut hit_count = 0;
    for &sq in targets {
        let mut tmp = post;
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
