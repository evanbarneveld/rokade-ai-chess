use crate::board::Board;
use crate::piece::pieces::{Color, Piece};
use crate::parser::resolvers::pawn_resolver::resolve_pawn_move;

pub struct ResolvedSanMove {
    pub resolved_san_move: String,
    pub is_capture: bool,
    pub is_king_side_castle: bool,
    pub is_queen_side_castle: bool,
    pub promotion_piece: Option<Piece>
}

#[derive(Debug)]
pub struct SanMoveResolver {}

impl SanMoveResolver {

    pub fn resolve_san_move(&self, piece: char, incomplete_move_part: &str, move_to: &str, is_capture:bool, promotion_piece:Option<Piece>, board: &Board, active_color:Color) -> Result<ResolvedSanMove, String> {

        let from_col = Self::get_column_number_from_char(incomplete_move_part.chars().nth(0).unwrap());
        let from_row = Self::get_row_number_from_char(incomplete_move_part.chars().nth(1).unwrap());

        let to_col = Self::get_column_number_from_char(move_to.chars().nth(0).unwrap());
        let to_row = Self::get_row_number_from_char(move_to.chars().nth(1).unwrap());

        let resolved_from_move = match piece {
            'P' => { resolve_pawn_move(from_col, from_row, to_col, to_row, is_capture, board, active_color)},
            /*'N' => {  }
            'B' => { self.revolve_bishop_move(from_col, from_row, to_col, to_row, is_capture, board, active_color)},
            'R' => { self.revolve_rook_move(from_col, from_row, to_col, to_row, is_capture, board, active_color)},
            'Q' => { self.revolve_queen_move(from_col, from_row, to_col, to_row, is_capture, board, active_color)},
            'K' => { self.revolve_king_move(from_col, from_row, to_col, to_row, is_capture, board, active_color)},*/
            _ => return Err(String::from("Invalid piece type"))
        };

        if resolved_from_move.is_none() { return Err(String::from("Invalid move"))};

        let resolved_move = Self::move_to_string(resolved_from_move.unwrap()) + &move_to;

        Ok(ResolvedSanMove{
            resolved_san_move: resolved_move,
            is_capture: is_capture,
            is_king_side_castle: false,
            is_queen_side_castle:false,
            promotion_piece
        })
    }

    fn move_to_string(move_from: (u8,u8)) -> String {
        let file = (move_from.0 as u8 + b'a') as char;
        let rank = move_from.1.to_string();
        format!("{}{}", file, rank)
    }

    fn get_row_number_from_char(rank_char: char) -> i8 {
        let row = if rank_char == '?' {
            -1
        } else {
            (rank_char as u8 - b'1') as i8
        };
        row
    }

    fn get_column_number_from_char(column_char: char) -> i8 {
        let col = if column_char == '?' {
            -1
        } else {
            (column_char as u8 - b'a') as i8
        };
        col
    }
}