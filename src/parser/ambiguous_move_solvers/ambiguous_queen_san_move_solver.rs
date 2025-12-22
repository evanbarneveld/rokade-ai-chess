use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};

/*
 Given a incomplete SAN move and the target position on the board, return the source position, or None if the move is invalid.
 */
pub fn solve_ambiguous_queen_san_move(from_col: i8, from_row: i8, to_col: i8, to_row: i8, is_capture:bool, board: &Board, active_color:Color) -> Option<(u8, u8)> {
    None
}

