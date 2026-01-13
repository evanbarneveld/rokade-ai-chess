use crate::state::game_state::GameState;

/// Generic move handler for pieces without special move rules (Knight, Bishop, Rook, Queen).
/// Handles the common logic: move piece, clear en-passant, update half-move clock.
pub fn move_standard_piece(game_state: &mut GameState, from: (usize, usize), to: (usize, usize), is_capture: bool) -> bool {
    // Perform the actual move on the board
    let moved = game_state.move_piece(from, to);
    if !moved { return false; }

    // Standard piece move clears any en-passant target
    game_state.set_en_passant_target(Option::None);

    // Update half-move clock: reset on capture, increment otherwise
    if is_capture {
        game_state.reset_half_move_clock();
    } else {
        game_state.increment_half_move_clock();
    }

    true
}
