//! Check mobility and self-hanging piece heuristics.

use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::piece::pieces::{Color, PieceType};
use crate::search::core::advanced_search::find_all_valid_moves;
use crate::search::management::see::{see_dest_estimate, SEE_PENALTY_MAX_CP, SEE_PENALTY_MIN_CP};
use crate::state::game_state::GameState;

use super::utils::{
    apply_for_side,
    CHECK_TIEBREAK_BASE, CHECK_MOBILITY_BONUS_0, CHECK_MOBILITY_BONUS_1_2, CHECK_MOBILITY_BONUS_3_5,
};

const QUEEN_HANGING_SEE_MULTIPLIER: i32 = 8;
const QUEEN_SEE_MIN_CLAMP: i32 = 400;
const QUEEN_SEE_MAX_CLAMP: i32 = 6000;
const ROOK_HANGING_SEE_MULTIPLIER: i32 = 3;
const ROOK_SEE_MIN_CLAMP: i32 = 300;
const ROOK_SEE_MAX_CLAMP: i32 = 2000;
const SEE_PENALTY_MULTIPLIER: i32 = 2;
const KNIGHT_SEE_MAX_CLAMP: i32 = 800;
const EXTRA_PENALTY_PER_ADDITIONAL_HANGING_PIECE_BEYOND_FIRST: i32 = 100;
const SEE_BONUS_FOR_ADVANCED_PAWN_GIVING_CHECK: i32 = 2000;


/// Calculate check mobility bonus based on opponent's available replies.
#[inline]
pub fn check_mobility_bonus_for_side(post_after: &Board, checked_side: Color) -> i32 {
    let mut opp_state = GameState::from_board_and_side(*post_after, checked_side);
    let replies = find_all_valid_moves(&mut opp_state).len() as i32;
    match replies {
        0 => CHECK_MOBILITY_BONUS_0,
        1..=2 => CHECK_MOBILITY_BONUS_1_2,
        3..=5 => CHECK_MOBILITY_BONUS_3_5,
        _ => 0,
    }
}

/// Self-hanging penalty aggregate OR check tie-break with mobility bonus.
#[inline]
pub fn self_hang_or_check_mobility(
    _base_board: &Board,
    post_after: &Board,
    side: Color,
    _from: (usize, usize),
    to: (usize, usize),
    gives_check: bool,
    opp: Color,
) -> i32 {
    // Scan our pieces for hanging penalties
    let mut total_penalty: i32 = 0;
    let mut hanging_pieces_found = 0;
    for r in 0..8 {
        for c in 0..8 {
            // Skip the destination square - it's already handled by apply_destination_see_penalties
            // to avoid double-counting the SEE penalty for the moved piece
            if (r, c) == to {
                continue;
            }
            if let Some(p) = post_after.get(r, c) {
                if p.get_color() != side {
                    continue;
                }
                let mut post_for_query = *post_after;
                if !is_square_attacked_by_opponent(&mut post_for_query, (r, c), side) {
                    continue;
                }
                let see = see_dest_estimate(post_after, side, (r, c), 0);
                if see < 0 {
                    hanging_pieces_found += 1;
                    // Scale penalty based on piece type - don't cap too low for valuable pieces
                    let pen = match p.get_type() {
                        PieceType::Queen => ((-see) * QUEEN_HANGING_SEE_MULTIPLIER).clamp(QUEEN_SEE_MIN_CLAMP, QUEEN_SEE_MAX_CLAMP),
                        PieceType::Rook => ((-see) * ROOK_HANGING_SEE_MULTIPLIER).clamp(ROOK_SEE_MIN_CLAMP, ROOK_SEE_MAX_CLAMP),
                        PieceType::Knight | PieceType::Bishop => ((-see) * SEE_PENALTY_MULTIPLIER).clamp(SEE_PENALTY_MIN_CP, KNIGHT_SEE_MAX_CLAMP),
                        _ => (-see).clamp(SEE_PENALTY_MIN_CP, SEE_PENALTY_MAX_CP) / 2,
                    };
                    total_penalty += pen;
                }
            }
        }
    }

    // If multiple pieces are hanging, apply additional penalty for exposing multiple weaknesses
    if hanging_pieces_found > 1 {
        total_penalty += EXTRA_PENALTY_PER_ADDITIONAL_HANGING_PIECE_BEYOND_FIRST * (hanging_pieces_found - 1);
    }

    // Don't cap hanging penalties - they should fully reflect the material at risk
    // EXCEPT when the move gives check - then the opponent must respond to check first
    let hang_pen = if total_penalty > 0 {
        -total_penalty
    } else {
        0
    };

    // Check bonus
    let mut check_bonus = 0;
    if gives_check {
        check_bonus += CHECK_TIEBREAK_BASE;
        check_bonus += check_mobility_bonus_for_side(post_after, opp);

        // Large bonus for advanced pawn checks (near promotion)
        // These are often winning tactics that should be prioritized
        if let Some(piece) = post_after.get(to.0, to.1)
            && piece.get_type() == PieceType::Pawn {
                let pawn_rank = to.0;
                // For White: rank 6 (row 6) or 7 (row 7) near promotion
                // For Black: rank 1 (row 1) or 2 (row 2) near promotion
                let is_advanced = match side {
                    Color::White => pawn_rank >= 6,
                    Color::Black => pawn_rank <= 1,
                };
                if is_advanced {
                    // Very large bonus for advanced pawn check
                    // This should outweigh most other considerations
                    check_bonus += SEE_BONUS_FOR_ADVANCED_PAWN_GIVING_CHECK;
                }
            }
    }

    // If we give check, skip hanging piece penalties entirely.
    // The opponent must respond to check first, so static analysis of hanging pieces
    // is unreliable. The search has already evaluated the full tactical consequences.
    // This allows brilliant sacrifices to be evaluated correctly.
    let adjusted_hang_pen = if gives_check {
        0 // Trust the search for checking moves
    } else {
        hang_pen
    };

    apply_for_side(adjusted_hang_pen + check_bonus, side)
}
