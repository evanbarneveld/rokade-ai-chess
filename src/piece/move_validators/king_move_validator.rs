use crate::piece::pieces::{Color, Piece};
use crate::state::game_state::GameState;

pub fn is_valid_king_move(game_state: &GameState, from: (usize, usize), to: (usize, usize)) -> bool {
    // King moves exactly one square in any direction (no castling handled here)
    if from == to { return false; }

    let dr = if from.0 > to.0 { from.0 - to.0 } else { to.0 - from.0 };
    let dc = if from.1 > to.1 { from.1 - to.1 } else { to.1 - from.1 };

    dr <= 1 && dc <= 1
}
