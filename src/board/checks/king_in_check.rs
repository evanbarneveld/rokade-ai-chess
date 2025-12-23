use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;

/// Given the move (from->to) would be executed, would the king be in check?
/// if so, 'the move is pinned', that means the move cannot be made
pub fn move_piece_is_pinned(board: &mut Board, move_from:(usize, usize), move_to:(usize, usize)) -> bool {
    let from_piece = board.get(move_from.0, move_from.1);
    let to_piece = board.get(move_to.0, move_to.1);

    let piece_color = from_piece.unwrap().get_color();

    board.set(move_from.0, move_from.1, None);
    board.set(move_to.0, move_to.1, from_piece);

    let king_in_check = is_square_attacked_by_opponent(board, board.get_king_location(piece_color), piece_color);

    //restore board
    board.set(move_from.0, move_from.1, from_piece);
    board.set(move_to.0, move_to.1, to_piece);

    king_in_check
}