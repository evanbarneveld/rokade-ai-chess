use crate::piece::pieces::{Piece};
use crate::state::game_state::GameState;

pub fn move_king(game_state: &mut GameState, piece: Piece, from: (usize, usize), to: (usize, usize), is_capture: bool) -> bool {
    // Perform the actual king move on the board
    let moved = game_state.mutable_board().move_piece_basic(from, to);
    if !moved { return false; }

    // Any king move revokes both castling rights for that color
    game_state.revoke_castling_rights_for_color(piece.get_color());

    // Handle castling: if king moves two files horizontally on same rank
    let same_rank = from.0 == to.0;
    let file_diff = from.1.abs_diff(to.1);
    if same_rank && file_diff == 2 {
        let row = from.0;
        // Kingside castling (to column 6)
        if to.1 == 6 {
            // Rook moves from h-file (7) to f-file (5)
            let _ = game_state.mutable_board().move_piece_basic((row, 7), (row, 5));
        }
        // Queenside castling (to column 2)
        else if to.1 == 2 {
            // Rook moves from a-file (0) to d-file (3)
            let _ = game_state.mutable_board().move_piece_basic((row, 0), (row, 3));
        }
    }

    // King move clears any en-passant target
    game_state.set_en_passant_target(Option::None);

    // Update half-move clock: reset on capture, increment otherwise
    if is_capture {
        game_state.reset_half_move_clock();
    } else {
        game_state.increment_half_move_clock();
    }

    game_state.update_king_location(piece.get_color(), to);

    true
}