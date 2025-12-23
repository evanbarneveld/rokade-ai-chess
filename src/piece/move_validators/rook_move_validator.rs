use crate::board::Board;
use crate::board::checks::king_in_check::move_piece_is_pinned;

pub fn is_valid_rook_move(board: &mut Board, from: (usize, usize), to: (usize, usize)) -> bool {
    // Rook moves must be strictly horizontal or vertical
    if from == to { return false; }

    let same_row = from.0 == to.0;
    let same_col = from.1 == to.1;

    if !(same_row || same_col) { return false; }

    if same_row {
        // Move along columns
        let start = if from.1 < to.1 { from.1 + 1 } else { to.1 + 1 };
        let end = if from.1 < to.1 { to.1 } else { from.1 };
        for c in start..end {
            if !board.board_square_is_empty((from.0, c)) { return false; }
        }
    } else {
        // Move along rows
        let start = if from.0 < to.0 { from.0 + 1 } else { to.0 + 1 };
        let end = if from.0 < to.0 { to.0 } else { from.0 };
        for r in start..end {
            if !board.board_square_is_empty((r, from.1)) { return false; }
        }
    }

    if move_piece_is_pinned(board, from, to) {
        return false;
    }

    true
}
