use crate::board::Board;
use crate::board::checks::king_in_check::is_king_in_check_after_move;

pub fn is_valid_bishop_move(board: &mut Board, from: (usize, usize), to: (usize, usize), do_pin_check: bool) -> bool {

    if from == to { return false; }

    // Bishop must move diagonally: absolute delta row equals absolute delta col
    let d_row = to.0.abs_diff(from.0);
    let d_col = to.1.abs_diff(from.1);

    if d_row == 0 || d_col == 0 { return false; }
    if d_row != d_col { return false; }

    // Determine step direction for rows and columns (+1 or -1)
    let step_row: i32 = if to.0 > from.0 { 1 } else { -1 };
    let step_col: i32 = if to.1 > from.1 { 1 } else { -1 };

    // Check all intermediate squares are empty (exclude destination)
    let mut r: i32 = from.0 as i32 + step_row;
    let mut c: i32 = from.1 as i32 + step_col;
    let end_r: i32 = to.0 as i32;
    let end_c: i32 = to.1 as i32;

    while r != end_r && c != end_c {
        if !board.is_empty((r as usize, c as usize)) { return false; }
        r += step_row;
        c += step_col;
    }

    if do_pin_check && is_king_in_check_after_move(board, from, to, None) {
        return false;
    }

    true
}
