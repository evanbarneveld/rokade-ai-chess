use crate::board::Board;
use crate::piece::pieces::{Color};


/// Basic check to see if a move is invalid, regardless of the type of the piece.
/// A move is invalid when:
///
/// - the coordinates are out of range
/// - the source square is empty, there is nothing to move
/// - the source square is occupied by a piece of the other player
/// - the target square is occupied by a piece of the other player, but the move is not a capture move
/// - the target square is occupied by a piece of the current player
/// - the target square is occupied but the move is not a capture
/// - the target square is empty (use the en-passant target if needed)
    pub fn move_from_and_to_validation_check(board: &Board, from: (usize, usize), to: (usize, usize), active_color:Color, is_capture:bool, is_pawn_move:bool, en_passant_target:Option<(usize, usize)>) -> bool {

    if from == to { return false; }

    if from.0 > 7 || from.1 > 7 || to.0 > 7 || to.1 > 7 { return false; }

    let source_piece = board.get(from.0, from.1);
    if source_piece.is_none() { return false; }
    if source_piece.unwrap().get_color() != active_color { return false; }

    let target_piece = board.get(to.0, to.1);

    if target_piece.is_some() {
        if !is_capture { return false; }
        if target_piece.unwrap().get_color() == active_color { return false; }
    } else {
        // no piece on 'to' square
        if is_capture {
            // if the move is an en-passant capture, then check the en-passant target square
            if is_pawn_move && en_passant_target.is_some() {
                // is the to square the en-passant target square? that square must be empty for a valid en-passant capture
                let ep_target = en_passant_target.unwrap();
                if to == ep_target { return true; }
            }
            return false;
        }
    }
    true
}