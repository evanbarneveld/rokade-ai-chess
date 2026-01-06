use crate::board::Board;
use crate::piece::pieces::{Color, Piece};

use crate::parser::ambiguous_move_solvers::ambiguous_knight_san_move_solver::solve_ambiguous_knight_san_move;
use crate::parser::ambiguous_move_solvers::ambiguous_bishop_san_move_solver::solve_ambiguous_bishop_san_move;
use crate::parser::ambiguous_move_solvers::ambiguous_pawn_san_move_solver::solve_ambiguous_pawn_san_move;
use crate::parser::ambiguous_move_solvers::ambiguous_queen_san_move_solver::solve_ambiguous_queen_san_move;
use crate::parser::ambiguous_move_solvers::ambiguous_king_san_move_solver::solve_ambiguous_king_san_move;
use crate::parser::ambiguous_move_solvers::ambiguous_rook_san_move_solver::solve_ambiguous_rook_san_move;

pub struct CompletedSanMove {
    pub resolved_san_move: String,
    pub is_capture: bool,
    pub is_king_side_castle: bool,
    pub is_queen_side_castle: bool,
    pub promotion_piece: Option<Piece>
}

#[derive(Debug)]
pub struct SanMoveCompleter {}

impl SanMoveCompleter {

    pub fn solve_ambiguous_san_move(&self, piece: char, incomplete_move_part: &str, move_to: &str, is_capture:bool, promotion_piece:Option<Piece>, board: &mut Board, active_color:Color, en_passant_target:Option<(usize,usize)>) -> Result<CompletedSanMove, String> {

        let from_col = Self::get_column_number_from_char(incomplete_move_part.chars().nth(0).unwrap());
        let from_row = Self::get_row_number_from_char(incomplete_move_part.chars().nth(1).unwrap());

        let to_col = Self::get_column_number_from_char(move_to.chars().nth(0).unwrap());
        let to_row = Self::get_row_number_from_char(move_to.chars().nth(1).unwrap());

        let resolved_from_move = match piece {
            'P' => { solve_ambiguous_pawn_san_move(from_col, from_row, to_col, to_row, is_capture, board, active_color, en_passant_target)},
            'N' => { solve_ambiguous_knight_san_move(from_col, from_row, to_col, to_row, is_capture, board, active_color)}
            'B' => { solve_ambiguous_bishop_san_move(from_col, from_row, to_col, to_row, is_capture, board, active_color)}
            'Q' => { solve_ambiguous_queen_san_move(from_col, from_row, to_col, to_row, is_capture, board, active_color)}
            'K' => { solve_ambiguous_king_san_move(from_col, from_row, to_col, to_row, is_capture, board, active_color)}
            'R' => { solve_ambiguous_rook_san_move(from_col, from_row, to_col, to_row, is_capture, board, active_color)}
            _ => return Err(String::from("Invalid piece type"))
        };

        if resolved_from_move.is_none() { return Err(String::from("Invalid move"))};

        let resolved_move = Self::move_to_string(resolved_from_move.unwrap()) + &move_to;

        Ok(CompletedSanMove {
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