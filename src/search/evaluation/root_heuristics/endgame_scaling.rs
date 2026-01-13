//! Endgame and 50-move rule scaling heuristics.

use crate::piece::pieces::Color;

use super::utils::{
    apply_for_side,
    ENDGAME_SIDEADV_THRESHOLD_CP, ENDGAME_HMC_THRESHOLD, ENDGAME_SCALE_MAX,
    ENDGAME_CAPTURE_SCALE_BONUS_CP, ENDGAME_NONCAP_SCALE_PENALTY_CP,
};

/// Apply endgame / 50-move rule scaling adjustments.
#[inline]
pub fn endgame_50move_scaling(
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
