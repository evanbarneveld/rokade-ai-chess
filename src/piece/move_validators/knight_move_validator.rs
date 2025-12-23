use crate::board::Board;
use crate::board::checks::king_in_check::move_piece_is_pinned;

pub fn is_valid_knight_move(board: &mut Board, from: (usize, usize), to: (usize, usize)) -> bool {
    // Knight moves in an L-shape: (2,1) or (1,2). Knights can jump over pieces.
    if from == to { return false; }

    let dr = if to.0 > from.0 { to.0 - from.0 } else { from.0 - to.0 };
    let dc = if to.1 > from.1 { to.1 - from.1 } else { from.1 - to.1 };

    if move_piece_is_pinned(board, from, to) {
        return false;
    }

    (dr == 2 && dc == 1) || (dr == 1 && dc == 2)
}
