use crate::board::Board;
use crate::board::checks::king_in_check::is_king_in_check_after_move;
use crate::piece::as_square_str;

pub fn is_valid_knight_move(board: &mut Board, from: (usize, usize), to: (usize, usize), do_pin_check:bool) -> bool {
    if from == to { return false; }

    // Knight moves in an L-shape: (2,1) or (1,2). Knights can jump over pieces.

    let dr = if to.0 > from.0 { to.0 - from.0 } else { from.0 - to.0 };
    let dc = if to.1 > from.1 { to.1 - from.1 } else { from.1 - to.1 };
    let ok = (dr == 2 && dc == 1) || (dr == 1 && dc == 2);

    if ok && do_pin_check && is_king_in_check_after_move(board, from, to, None) {
        return false;
    }

    if ok {
        //println!("valid knight move: {}", as_square_str(from, to));
    }

    ok
}
