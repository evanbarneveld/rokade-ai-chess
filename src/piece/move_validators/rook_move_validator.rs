use crate::board::Board;
use crate::board::checks::king_in_check::is_king_in_check_after_move;

pub fn is_valid_rook_move(board: &mut Board, from: (usize, usize), to: (usize, usize), do_pin_check:bool) -> bool {
    if from == to { return false; }

    // Rook moves must be strictly horizontal or vertical

    let same_row = from.0 == to.0;
    let same_col = from.1 == to.1;

    if !(same_row || same_col) { return false; }

    if same_row {
        // Move along columns
        let start = if from.1 < to.1 { from.1 + 1 } else { to.1 + 1 };
        let end = if from.1 < to.1 { to.1 } else { from.1 };
        for c in start..end {
            if !board.is_empty((from.0, c)) { return false; }
        }
    } else {
        // Move along rows
        let start = if from.0 < to.0 { from.0 + 1 } else { to.0 + 1 };
        let end = if from.0 < to.0 { to.0 } else { from.0 };
        for r in start..end {
            if !board.is_empty((r, from.1)) { return false; }
        }
    }

    if do_pin_check && is_king_in_check_after_move(board, from, to, None) {
        return false;
    }

    true
}
