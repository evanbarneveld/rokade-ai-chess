use crate::piece::pieces::{Color, Piece, PieceType};
use crate::state::game_state::GameState;

pub fn move_bishop(game_state: &mut GameState, piece: Piece, from: (usize, usize), to: (usize, usize), is_capture: bool) -> bool {
    // Perform the actual move on the board
    let moved = game_state.move_piece(from, to);
    if !moved { return false; }

    // Bishop move clears any en-passant target
    game_state.set_en_passant_target(Option::None);

    // Update half-move clock: reset on capture, increment otherwise
    if is_capture {
        game_state.reset_half_move_clock();
    } else {
        game_state.increment_half_move_clock();
    }

    true
}