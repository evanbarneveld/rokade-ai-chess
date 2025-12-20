use crate::board::Board;
use crate::piece::pieces::Color;

pub struct ResolvedSanMove {
    pub resolved_san_move: String,
    pub is_capture: bool,
    pub is_king_side_castle: bool,
    pub is_queen_side_castle: bool,
    pub promotion_piece: Option<char>
}

#[derive(Debug)]
pub struct SanMoveResolver {}

impl SanMoveResolver {

    pub fn resolve_san_move(&self, piece: char, incomplete_move_part: &str, moveTo: &str, is_capture:bool, board: &Board, active_color:Color) -> Result<ResolvedSanMove, String> {
        let column_char = incomplete_move_part.chars().nth(0).unwrap();
        let rank_char = incomplete_move_part.chars().nth(1).unwrap();

        let col = if column_char == '?' {
            -1
        } else {
            (column_char as u8 - b'a') as i8
        };

        let row = if rank_char == '?' {
            -1
        } else {
            (rank_char as u8 - b'1') as i8
        };
        
        let resolvedMove = match piece {
            'P' => { self.revolve_pawn_move(col, row, moveTo, is_capture, board, active_color)},
            'N' => { self.revolve_knight_move(col, row, moveTo, is_capture, board, active_color)},
            'B' => { self.revolve_bishop_move(col, row, moveTo, is_capture, board, active_color)},
            'R' => { self.revolve_rook_move(col, row, moveTo, is_capture, board, active_color)},
            'Q' => { self.revolve_queen_move(col, row, moveTo, is_capture, board, active_color)},
            'K' => { self.revolve_king_move(col, row, moveTo, is_capture, board, active_color)},
            _ => return Err(String::from("Invalid piece type"))
        };

        Ok(ResolvedSanMove{
            resolved_san_move: resolvedMove,
            is_capture: false,
            is_king_side_castle: false,
            is_queen_side_castle:false,
            promotion_piece: None
        })
    }

    pub fn revolve_pawn_move(&self, col: i8, row: i8, moveTo: &str, is_capture:bool, board: &Board, active_color:Color) -> String {
        String::from("e2e4")
    }

    pub fn revolve_knight_move(&self, col: i8, row: i8, moveTo: &str, is_capture:bool, board: &Board, active_color:Color) -> String {
        String::from("e2e4")
    }

    pub fn revolve_bishop_move(&self, col: i8, row: i8, moveTo: &str, is_capture:bool, board: &Board, active_color:Color) -> String {
        String::from("e2e4")
    }

    pub fn revolve_rook_move(&self, col: i8, row: i8, moveTo: &str, is_capture:bool, board: &Board, active_color:Color) -> String {
        String::from("e2e4")
    }

    pub fn revolve_king_move(&self, col: i8, row: i8, moveTo: &str, is_capture:bool, board: &Board, active_color:Color) -> String {
        String::from("e2e4")
    }

    pub fn revolve_queen_move(&self, col: i8, row: i8, moveTo: &str, is_capture:bool, board: &Board, active_color:Color) -> String {
        String::from("e2e4")
    }
}