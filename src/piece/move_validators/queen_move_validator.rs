use crate::board::Board;
use crate::board::checks::king_in_check::move_piece_is_pinned;
use crate::piece::move_validators::rook_move_validator::is_valid_rook_move;
use crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move;

pub fn is_valid_queen_move(board: &mut Board, from: (usize, usize), to: (usize, usize)) -> bool {
    // Queen moves are valid if they are valid rook moves (straight) or bishop moves (diagonal)
    if from == to { return false; }

    let result = is_valid_rook_move(board, from, to) ||
    is_valid_bishop_move(board, from, to);

    if move_piece_is_pinned(board, from, to, None) {
        return false;
    }

    result
}
