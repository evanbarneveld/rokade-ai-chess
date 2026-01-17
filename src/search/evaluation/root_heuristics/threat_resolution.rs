//! Threat resolution and piece evacuation heuristics.

use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::piece::pieces::{opposite_color, Color, PieceType};
use crate::search::management::see::{attacked_by_pawn, see_dest_estimate};

use super::utils::{
    apply_for_side, center_score,
    KNIGHT_IGNORE_PAWN_THREAT_PENALTY, KNIGHT_NON_EVAC_DEMOTION, KNIGHT_SAFE_TO_SPECIFIC_REWARD,
};
use super::knight_evacuation::knight_safe_squares;

/// Handle threat resolution and piece evacuation heuristics.
#[inline]
pub fn threat_resolution_and_evacuation(
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

    let mut base_clone = *base_board;
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
            let mut tmpmv = *post_after;
            is_square_attacked_by_opponent(&mut tmpmv, to, side)
        } else if post_after.get(tr, tc).is_none() {
            false
        } else {
            let mut tmp2 = *post_after;
            is_square_attacked_by_opponent(&mut tmp2, (tr, tc), side)
        };

        if (tr, tc) == from {
            // We moved the threatened piece - calculate evacuation bonus
            let mut evac_bonus = 0;
            let see_new = see_dest_estimate(post_after, side, to, 0);

            // ALWAYS give evacuation bonus for moving an attacked piece
            // Scale bonus by piece value to prioritize saving more valuable pieces
            let base_evac = match pt {
                PieceType::Queen => 800,
                PieceType::Rook => 600,
                PieceType::Bishop | PieceType::Knight => 500,
                PieceType::Pawn => 300,
                PieceType::King => 1000,
            };

            // Moving to safety gets full bonus; moving to another attacked square gets half bonus
            if !still_attacked || see_new >= 0 {
                evac_bonus += base_evac;
            } else {
                // Even moving to an attacked square is better than leaving it hanging
                evac_bonus += base_evac / 2;
            }


            // Knight-specific center bonus
            if pt == PieceType::Knight {
                let mut cb = center_score(to);
                if to == (3, 3) {
                    cb += 80;
                }
                if !knight_safe.is_empty() && knight_safe.contains(&to) {
                    cb += 80;
                }
                evac_bonus += cb.max(0);
            }

            // Bonus for evacuating to a known safe square
            if pt == PieceType::Knight && by_pawn && !knight_safe.is_empty()
                && knight_safe.contains(&to) {
                    evac_bonus += KNIGHT_SAFE_TO_SPECIFIC_REWARD;
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
            } else if knight_safe.contains(&to) {
                delta += apply_for_side(KNIGHT_SAFE_TO_SPECIFIC_REWARD, side);
            }
        }
    }
    delta
}
