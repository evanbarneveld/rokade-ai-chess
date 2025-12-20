use crate::board::Board;
use crate::piece::pieces::{Piece, Color};
use crate::state::outcome::GameOutcome;
use crate::state::castling::CastlingRights;

#[derive(Debug, Clone)]
pub struct GameState {
    board: Board,
    active_color: Color,
    castling_rights: CastlingRights,
    en_passant_target: Option<(usize, usize)>,
    half_move_clock: u32,
    full_move_number: u32,
    outcome: Option<GameOutcome>
}


impl GameState {
    pub fn new() -> Self {
        GameState {
            board: Board::new(),
            active_color: Color::White,
            castling_rights: CastlingRights::all(),
            en_passant_target: None,
            half_move_clock: 0, //for 50 move rule (since last pawn move or capture)
            full_move_number: 1,
            outcome: None
        }
    }

    pub fn new_from_existing_state(board: Board, active_color:Color, castling_rights: CastlingRights, en_passant_target: Option<(usize, usize)>, half_move_clock: u32, full_move_number: u32, outcome: Option<GameOutcome>) -> Self {
        GameState {
            board,
            active_color,
            castling_rights,
            en_passant_target,
            half_move_clock,
            full_move_number,
            outcome
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn castling_rights(&self) -> CastlingRights {
        self.castling_rights.clone()
    }

    pub fn en_passant_target(&self) -> Option<(usize, usize)> {
        self.en_passant_target
    }

    pub fn half_move_clock(&self) -> u32 {
        self.half_move_clock
    }

    pub fn full_move_number(&self) -> u32 {
        self.full_move_number
    }

    pub fn move_piece(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        self.board.move_piece(from, to)
    }

    pub fn set_en_passant_target(&mut self, target: Option<(usize, usize)>) {
        self.en_passant_target = target;
    }

    pub fn active_color(&self) -> Color {
        self.active_color
    }

    pub fn switch_color(&mut self) {
        match self.active_color {
            Color::White => self.active_color = Color::Black,
            Color::Black => self.active_color = Color::White,
        }
    }

    pub fn increment_full_move_number(&mut self) {
        self.full_move_number += 1;
    }

    pub fn increment_half_move_clock(&mut self) {
        self.half_move_clock += 1;
    }

    pub fn reset_half_move_clock(&mut self) {
        self.half_move_clock = 0;
    }
}


