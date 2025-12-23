use crate::board::Board;
use crate::board::checks::king_in_check::move_piece_is_pinned;
use crate::piece::move_validators::rook_move_validator::is_valid_rook_move;
use crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move;

pub fn is_valid_queen_move(board: &mut Board, from: (usize, usize), to: (usize, usize), do_pin_check:bool) -> bool {
    if from == to { return false; }

    // Queen moves are valid if they are valid rook moves (straight) or bishop moves (diagonal)

    let result = is_valid_rook_move(board, from, to, do_pin_check) ||
    is_valid_bishop_move(board, from, to, do_pin_check);

    if do_pin_check && move_piece_is_pinned(board, from, to, None) {
        return false;
    }

    result
}
