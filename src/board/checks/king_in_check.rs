use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::piece::pieces::{Color, Piece, PieceType};

/// Given the move (from->to) would be executed, would the king be in check?
/// if so, 'the move is pinned' so that means the move cannot be made
pub fn is_king_in_check_after_move(board: &mut Board, move_from:(usize, usize), move_to:(usize, usize), en_passant_target: Option<(usize, usize)>) -> bool {
    let from_piece = board.get(move_from.0, move_from.1);
    let to_piece = board.get(move_to.0, move_to.1);

    let piece_color = from_piece.unwrap().get_color();

    board.set(move_from.0, move_from.1, None);
    board.set(move_to.0, move_to.1, from_piece);

    let move_is_king_move = from_piece.unwrap().get_type() == PieceType::King;

    if move_is_king_move {
       // we just moved the king on the board
       board.set_king_location(piece_color, move_to);
    }

    let mut is_en_passant_capture = false;

    if from_piece.unwrap().get_type() == PieceType::Pawn
        && let Some(ep_target) = en_passant_target
        && move_to.0 == ep_target.0
        && move_to.1 == ep_target.1
    {
        //is this an en-passant capture move?
        is_en_passant_capture = true;
        if from_piece.unwrap().get_color() == Color::White {
            //adjust the board to simulate the en-passant capture
            //board.set(move_to.0, move_to.1, None);
            board.set(move_to.0 - 1, move_to.1, None); //remove the black pawn that was captured e.p.
        } else {
            board.set(move_to.0 + 1, move_to.1, None); //remove the white pawn that was captured e.p.
        }
    }
    let king_in_check = is_square_attacked_by_opponent(board, board.get_king_location(piece_color), piece_color);

    //restore the board: undo the move
    board.set(move_from.0, move_from.1, from_piece);
    board.set(move_to.0, move_to.1, to_piece);

    if move_is_king_move {
        // we just did an un-move of the king
        board.set_king_location(piece_color, move_from);
    }

    if is_en_passant_capture {
        //rest the board: undo the en-passant capture
        if from_piece.unwrap().get_color() == Color::White {
            //adjust the board to simulate the en-passant capture
            //board.set(move_to.0, move_to.1, None);
            board.set(move_to.0 - 1, move_to.1, Some(Piece::new(PieceType::Pawn, Color::Black)));
        } else {
            board.set(move_to.0 + 1, move_to.1, Some(Piece::new(PieceType::Pawn, Color::White)));
        }
    }
    king_in_check
}