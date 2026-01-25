//! Threat resolution and piece evacuation heuristics.

use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::piece::pieces::{opposite_color, Color, PieceType};
use crate::search::management::see::{attacked_by_pawn, find_smallest_attacker, see_dest_estimate};
use crate::piece::pieces::piece_value_cp;

use super::utils::{
    apply_for_side, center_score,
    KNIGHT_IGNORE_PAWN_THREAT_PENALTY, KNIGHT_NON_EVAC_DEMOTION, KNIGHT_SAFE_TO_SPECIFIC_REWARD,
};
use super::knight_evacuation::knight_safe_squares;

/// Calculate knight-specific evacuation bonus
#[inline]
fn knight_evacuation_bonus(
    to: (usize, usize),
    knight_safe: &[(usize, usize)],
    by_pawn: bool,
) -> i32 {
    let mut bonus = 0;

    // Center bonus
    let mut cb = center_score(to);
    if to == (3, 3) {
        cb += 80;
    }
    if !knight_safe.is_empty() && knight_safe.contains(&to) {
        cb += 80;
    }
    bonus += cb.max(0);

    // Safe square bonus
    if by_pawn && !knight_safe.is_empty() && knight_safe.contains(&to) {
        bonus += KNIGHT_SAFE_TO_SPECIFIC_REWARD;
    }

    bonus
}

/// Detects if opponent has a pawn one move away from promotion.
/// Returns a massive penalty if we don't address this threat.
#[inline]
fn detect_opponent_promotion_threat(
    base_board: &Board,
    post_after: &Board,
    side: Color,
    _from: (usize, usize),
    to: (usize, usize),
) -> i32 {
    let opp = opposite_color(side);
    let promotion_rank = match opp {
        Color::White => 6, // White pawns on rank 7 (index 6) can promote next move
        Color::Black => 1, // Black pawns on rank 2 (index 1) can promote next move
    };

    let promotion_square_rank = match opp {
        Color::White => 7,
        Color::Black => 0,
    };

    let mut threat_penalty = 0;

    // Check all opponent pawns on the promotion rank
    for col in 0..8 {
        if let Some(p) = base_board.get(promotion_rank, col)
            && p.get_color() == opp && p.get_type() == PieceType::Pawn {
                // Found an opponent pawn threatening to promote!

                // First check: did we capture the pawn?
                if to == (promotion_rank, col) {
                    continue; // Threat eliminated
                }

                // Second: is the pawn still there after our move?
                let pawn_still_exists = post_after.get(promotion_rank, col)
                    .map(|p| p.get_color() == opp && p.get_type() == PieceType::Pawn)
                    .unwrap_or(false);

                if !pawn_still_exists {
                    continue; // Pawn was captured somehow
                }

                // Third: can the pawn actually promote on its next move?
                // Check if it can advance straight
                let can_advance_straight = base_board.get(promotion_square_rank, col).is_none();
                let blocked_straight_by_us = to == (promotion_square_rank, col);

                // Check if it can capture diagonally to promote
                let mut can_capture_diagonal = false;
                if col > 0
                    && let Some(piece) = base_board.get(promotion_square_rank, col - 1)
                        && piece.get_color() == side {
                            can_capture_diagonal = true;
                            // Did we move this piece away?
                            if to != (promotion_square_rank, col - 1) {
                                // Piece still there after our move
                            } else {
                                can_capture_diagonal = false; // We removed the capturable piece
                            }
                        }
                if col < 7
                    && let Some(piece) = base_board.get(promotion_square_rank, col + 1)
                        && piece.get_color() == side {
                            let piece_still_there = post_after.get(promotion_square_rank, col + 1)
                                .map(|p| p.get_color() == side)
                                .unwrap_or(false);
                            if piece_still_there {
                                can_capture_diagonal = true;
                            }
                        }

                // If pawn can promote (either by advancing or capturing) and we didn't stop it
                let can_promote = (can_advance_straight && !blocked_straight_by_us) || can_capture_diagonal;

                if can_promote {
                    // Promotion to queen is worth ~900cp, so penalty should be massive
                    // Apply penalty from the perspective of the side to move
                    // If opponent can promote, that's BAD for us, so:
                    // - For White: penalty makes score more negative (worse)
                    // - For Black: penalty makes score more positive (worse for Black since Black wants negative)
                    let penalty_value = match side {
                        Color::White => -1200, // White wants positive scores, so -1200 is bad
                        Color::Black => 1200,  // Black wants negative scores, so +1200 is bad
                    };
                    threat_penalty += penalty_value;
                }
            }
    }

    threat_penalty
}

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

    // First check for opponent promotion threats
    let mut delta = detect_opponent_promotion_threat(base_board, post_after, side, from, to);

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
        return delta;
    }
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

    // Check if this is a "genuine" threat - smallest attacker has equal or lesser value
        // AND the piece is not well-defended (SEE < 0 means capturing would lose material)
        let piece_val = piece_value_cp(pt);
        let is_genuine_threat = if let Some((_, attacker)) = find_smallest_attacker(base_board, (tr, tc), opp) {
            // Attacker value must be <= piece value for a genuine threat
            if piece_value_cp(attacker.get_type()) > piece_val {
                false
            } else {
                // Also check SEE - if SEE >= 0, the piece is well-defended
                let see = see_dest_estimate(base_board, opp, (tr, tc), 0);
                see > 0 // Only genuine if capturing would gain material (SEE > 0)
            }
        } else {
            false
        };

        if (tr, tc) == from {
            // We moved the threatened piece - calculate evacuation bonus
            let see_new = see_dest_estimate(post_after, side, to, 0);

            // Check if this is a "genuine" threat - smallest attacker has equal or lesser value
            let piece_val = piece_value_cp(pt);
            let is_genuine_threat = if let Some((_, attacker)) = find_smallest_attacker(base_board, (tr, tc), opp) {
                piece_value_cp(attacker.get_type()) <= piece_val
            } else {
                false // No attacker found (shouldn't happen since piece is in threatened list)
            };

            // Base evacuation bonus - scale by piece value
            let base_evac = match pt {
                PieceType::Queen => 800,
                PieceType::Rook => 600,
                PieceType::Bishop | PieceType::Knight => 500,
                PieceType::Pawn => 300,
                PieceType::King => 1000,
            };

            // If not a genuine threat (attacker is more valuable), reduce bonus significantly
            // e.g., bishop attacking pawn is not a real threat, so minimal evacuation bonus
            let threat_factor = if is_genuine_threat { 1 } else { 10 }; // 1/10th bonus for non-genuine threats

            // Moving to safety gets full bonus; moving to another attacked square with negative SEE gets a PENALTY
            let evac_bonus = if !still_attacked || see_new >= 0 {
                let mut bonus = base_evac / threat_factor;
                // Add knight-specific bonuses ONLY if we're actually moving to safety
                if pt == PieceType::Knight {
                    bonus += knight_evacuation_bonus(to, &knight_safe, by_pawn);
                }
                bonus
            } else {
                // If we're moving to a square where we're still hanging (negative SEE),
                // this is a bad move - don't give evacuation bonus and instead apply a penalty.
                // The penalty should discourage "fake evacuations" that still lose material.
                -base_evac / 2 // Strong penalty - half the evacuation bonus as negative
            };

            delta += apply_for_side(evac_bonus, side);
        } else {
            // We did NOT move the threatened piece - apply penalties only if:
            // 1. Piece remains threatened after our move
            // 2. The threat is genuine (attacker can profitably capture)
            if !still_attacked {
                continue; // Piece no longer threatened after our move (e.g., we blocked the attack)
            }

            // Skip penalty if the threat is not genuine (piece is well-defended)
            if !is_genuine_threat {
                continue;
            }

            // Knight-specific penalty for ignoring pawn threats when safe squares exist
            if pt == PieceType::Knight && by_pawn && !knight_safe.is_empty() {
                delta -= apply_for_side(KNIGHT_IGNORE_PAWN_THREAT_PENALTY + KNIGHT_NON_EVAC_DEMOTION, side);
            } else {
                // General penalty for leaving pieces hanging
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
    }
    delta
}
