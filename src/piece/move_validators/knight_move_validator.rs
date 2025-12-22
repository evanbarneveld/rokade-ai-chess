use crate::piece::pieces::{Color, Piece};
use crate::state::game_state::GameState;

pub fn is_valid_knight_move(game_state: &GameState, from: (usize, usize), to: (usize, usize), is_capture:bool, active_color:Color) -> bool {
    // Knight moves in an L-shape: (2,1) or (1,2). Knights can jump over pieces.
    if from == to { return false; }

    let dr = if to.0 > from.0 { to.0 - from.0 } else { from.0 - to.0 };
    let dc = if to.1 > from.1 { to.1 - from.1 } else { from.1 - to.1 };

    (dr == 2 && dc == 1) || (dr == 1 && dc == 2)
}
