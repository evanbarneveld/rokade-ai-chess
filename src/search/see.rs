use crate::board::Board;
use crate::piece::pieces::{opposite_color, piece_value_cp, Color};

// Root SEE-based gating and penalties
pub const SEE_PENALTY_MIN_CP: i32 = 80; // min penalty for negative SEE (non-queen)
pub const SEE_PENALTY_MAX_CP: i32 = 300; // max penalty for negative SEE (non-queen)

// Conservative SEE estimate on destination square.
// Returns net material from the perspective of the side that just moved.
// Positive => safe/profitable, Negative => likely losing material on dest.
#[inline]
pub fn see_dest_estimate(
    board_after: &Board,
    side_just_moved: Color,
    dest: (usize, usize),
    captured_val: i32,
) -> i32 {
    use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
    let moved_piece = match board_after.get(dest.0, dest.1) {
        Some(p) => p,
        None => return captured_val,
    };
    let moved_val = piece_value_cp(moved_piece.get_type());
    // We need a mutable board for the attack probe helper; clone cheaply
    let mut tmp1 = board_after.clone();
    let attacked_by_opp = is_square_attacked_by_opponent(&mut tmp1, dest, side_just_moved);
    // defend check: reuse helper by swapping perspective
    let mut tmp2 = board_after.clone();
    let defended_by_us =
        is_square_attacked_by_opponent(&mut tmp2, dest, opposite_color(side_just_moved));

    if !attacked_by_opp {
        // No immediate opponent attack: return captured gain (if any)
        return captured_val;
    }
    if attacked_by_opp && !defended_by_us {
        // Likely losing the moved piece outright
        return captured_val - moved_val;
    }
    // Both attacked and defended: assume partial liability
    captured_val - (moved_val / 2)
}
