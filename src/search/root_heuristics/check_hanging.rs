//! Check mobility and self-hanging piece heuristics.

use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::piece::pieces::{Color, PieceType};
use crate::search::advanced_search::find_all_valid_moves;
use crate::search::see::{see_dest_estimate, SEE_PENALTY_MAX_CP, SEE_PENALTY_MIN_CP};
use crate::state::game_state::GameState;

use super::utils::{
    apply_for_side,
    CHECK_TIEBREAK_BASE, CHECK_MOBILITY_BONUS_0, CHECK_MOBILITY_BONUS_1_2, CHECK_MOBILITY_BONUS_3_5,
};

/// Calculate check mobility bonus based on opponent's available replies.
#[inline]
pub fn check_mobility_bonus_for_side(post_after: &Board, checked_side: Color) -> i32 {
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
pub fn self_hang_or_check_mobility(
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
