use crate::piece::pieces::{Color, Piece};
use crate::state::game_state::GameState;
use crate::piece::move_validators::rook_move_validator::is_valid_rook_move;
use crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move;

pub fn is_valid_queen_move(game_state: &GameState, from: (usize, usize), to: (usize, usize), is_capture:bool, active_color:Color) -> bool {
    // Queen moves are valid if they are valid rook moves (straight) or bishop moves (diagonal)
    if from == to { return false; }

    is_valid_rook_move(game_state, from, to, is_capture, active_color) ||
    is_valid_bishop_move(game_state, from, to, is_capture, active_color)
}
