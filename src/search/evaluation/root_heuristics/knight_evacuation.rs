//! Knight-specific evacuation heuristics for pawn threats.

use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::piece::pieces::{opposite_color, Color, PieceType};
use crate::search::management::see::{attacked_by_pawn, see_dest_estimate};

use super::utils::{
    apply_for_side, simulate_move, CENTER_SQUARES,
    KNIGHT_SAFE_EVAC_REWARD, KNIGHT_CENTER_EXTRA_D4, KNIGHT_CENTER_STEP,
};

/// Compute safe squares a knight can move to from a given position.
#[inline]
pub fn knight_safe_squares(board: &Board, side: Color, from: (usize, usize)) -> Vec<(usize, usize)> {
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
        if let Some(occ) = board.get(nr, nc)
            && occ.get_color() == side {
                continue;
            }
        let (sim, _) = simulate_move(board, from, (nr, nc), None);
        let mut tmp = sim;
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
pub fn knight_evacuations_priority(
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
            if let Some(p) = base_board.get(r, c)
                && p.get_color() == side && p.get_type() == PieceType::Knight
                    && attacked_by_pawn(base_board, (r, c), opp) {
                        attacked_knights.push((r, c));
                    }
        }
    }
    if attacked_knights.is_empty() {
        return 0;
    }

    let mut delta = 0;

    // Penalize moves that don't evacuate an attacked knight (only if safe squares exist)
    if !attacked_knights.contains(&from) {
        // Check if any attacked knight has safe squares available
        let has_safe_squares = attacked_knights.iter().any(|&knight_pos| {
            !knight_safe_squares(base_board, side, knight_pos).is_empty()
        });
        if has_safe_squares {
            // Reduced from 500 to 250 to avoid overriding legitimately good moves
            // (e.g., winning material elsewhere)
            delta -= apply_for_side(250, side);
        }
    } else if let Some(p) = base_board.get(from.0, from.1)
        && p.get_type() == PieceType::Knight {
            let (tr, tc) = to;
            let (sim, _) = simulate_move(base_board, from, to, None);
            let mut tmp = sim;
            let dest_attacked = is_square_attacked_by_opponent(&mut tmp, (tr, tc), side);
            let see1 = see_dest_estimate(&sim, side, (tr, tc), 0);

            if !dest_attacked || see1 >= 0 {
                // Safe evacuation bonus with center preference
                let mut evac = KNIGHT_SAFE_EVAC_REWARD;
                for &(cr, cc) in &CENTER_SQUARES {
                    let dr = tr.abs_diff(cr) as i32;
                    let dc = tc.abs_diff(cc) as i32;
                    let dist = dr.saturating_add(dc);
                    let center_bonus = 20_i32.saturating_sub(KNIGHT_CENTER_STEP.saturating_mul(dist));
                    evac = evac.saturating_add(center_bonus.max(0));
                }
                // Extra bonus for d4 square
                if (tr, tc) == (3, 3) {
                    evac = evac.saturating_add(KNIGHT_CENTER_EXTRA_D4);
                }
                delta += apply_for_side(evac, side);
            } else {
                delta -= apply_for_side(150, side);
            }
        }
    delta
}
