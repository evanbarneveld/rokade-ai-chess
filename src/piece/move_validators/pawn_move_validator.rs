use crate::board::Board;
use crate::board::checks::king_in_check::move_leads_to_check;
use crate::piece::as_square_str;
use crate::piece::pieces::{Color, Piece};

pub fn is_valid_pawn_move(board: &mut Board, from: (usize, usize), to: (usize, usize), is_capture:bool, en_passant_target:Option<(usize,usize)>, active_color:Color, promotion_piece:Option<Piece>, do_pin_check:bool) -> bool {

    if from == to { return false; }

    // from (rank, col)
    // to (rank, col)

    // check horizontal movement
    if is_capture {
        // a capture move has a target on the column before or after the column
        if (from.1 as i32 - to.1 as i32).abs() != 1 { return false }
    } else {
        // normal move stays in the same column
        if from.1 != to.1 { return false }
    }

    // check vertical movement
    if active_color == Color::White {
        if from.0 == 1 {
            //move starts from rank#2, 1 step or 2 steps possible, not > 2
            if to.0 < from.0 { return false }
            if to.0 - from.0 > 2 { return false };
            if to.0 - from.0 == 2 {
                //2 steps only possible if the path is clear
                if !board.board_square_is_empty( (from.0 + 1, from.1)) { return false }
            }
         } else {
            if to.0 as i32 - from.0 as i32 != 1 { return false };
         }
    } else {
        //black
        if from.0 == 6 {
            //move starts from rank#6, 1 step or 2 steps possible, not > 2
            if to.0 > from.0 { return false }
            if from.0 - to.0 > 2 { return false }
            if from.0 - to.0 == 2 {
                //2 steps only possible if the path is clear
                if !board.board_square_is_empty( (from.0 - 1, from.1)) { return false }
            }
        } else {
            if from.0 as i32 - to.0 as i32 != 1 { return false }
        }
    }

    if promotion_piece.is_some() {
        if active_color == Color::White {
            if to.0 != 7 { return false }
        } else {
            if to.0 != 0 { return false }
        }
    }

    if do_pin_check &&  move_leads_to_check(board, (from.0, from.1), (to.0, to.1), en_passant_target) {
        return false;
    }

    //println!("valid pawn move: {}", as_square_str(from, to));
    true
}
