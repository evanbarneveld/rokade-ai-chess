use crate::board::Board;
use crate::history::history::History;
use crate::piece::pieces::{Piece, Color};
use crate::state::outcome::{recompute_outcome, OutcomeType};
use crate::state::castling::CastlingRights;

#[derive(Debug, Clone, Copy)]
pub struct GameState {
    board: Board,
    active_color: Color,
    castling_rights: CastlingRights,
    en_passant_target: Option<(usize, usize)>,
    half_move_clock: u32,
    full_move_number: u32,
    outcome: Option<OutcomeType>
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            board: Board::new(),
            active_color: Color::White,
            castling_rights: CastlingRights::all(),
            en_passant_target: None,
            half_move_clock: 0, //for 50 move rule (since last move_validators move or capture)
            full_move_number: 1,
            outcome: None
        }
    }

    pub fn new_from_existing_state(board: Board, active_color: Color, castling_rights: CastlingRights, en_passant_target: Option<(usize, usize)>, half_move_clock: u32, full_move_number: u32) -> Self {
        GameState {
            board,
            active_color,
            castling_rights,
            en_passant_target,
            half_move_clock,
            full_move_number,
            outcome: None
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn mutable_board(&mut self) -> &mut Board {
        &mut self.board
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

    pub fn move_from_and_to_validation_check(&self, from: (usize, usize), to: (usize, usize), active_color: Color, is_capture: bool, is_pawn_move: bool, en_passant_target: Option<(usize, usize)>) -> bool {
        self.board.move_from_and_to_validation_check(from, to, active_color, is_capture, is_pawn_move, en_passant_target)
    }

    pub fn board_square_has_piece_of_opposite_color(&self, to: (usize, usize), active_color: Color) -> bool {
        self.board.board_square_has_piece_of_opposite_color(to, active_color)
    }

    pub fn board_square_is_empty(&self, location: (usize, usize)) -> bool {
        self.board.board_square_is_empty(location)
    }

    pub fn clear_square(&mut self, row: usize, col: usize) {
        self.board.clear(row, col);
    }

    pub fn move_piece(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        self.board.move_piece(from, to)
    }

    pub fn move_pawn(&mut self, from: (usize, usize), to: (usize, usize), promotion_piece: Option<Piece>) -> bool {
        self.board.move_pawn(from, to, promotion_piece)
    }

    pub fn set_en_passant_target(&mut self, target: Option<(usize, usize)>) {
        self.en_passant_target = target;
    }

    pub fn get_en_passant_target(&self) -> Option<(usize, usize)> {
        self.en_passant_target
    }

    pub fn active_color(&self) -> Color {
        self.active_color
    }

    pub fn switch_player_turn(&mut self) {
        match self.active_color {
            Color::White => self.active_color = Color::Black,
            Color::Black => self.active_color = Color::White,
        }
        if self.active_color == Color::White {
            self.increment_full_move_number();
        }
    }

    pub fn increment_full_move_number(&mut self) {
        self.full_move_number += 1;
    }

    pub fn increment_half_move_clock(&mut self) {
        self.half_move_clock += 1;
    }

    pub fn update_king_location(&mut self, color: Color, location: (usize, usize)) {
        self.board.set_king_location(color, location);
    }

    pub fn reset_half_move_clock(&mut self) {
        self.half_move_clock = 0;
    }

    pub fn revoke_castling_rights_for_color(&mut self, color: Color) {
        match color {
            Color::White => self.castling_rights.revoke_white_castling(),
            Color::Black => self.castling_rights.revoke_black_castling(),
        }
    }

    pub fn get_half_move_clock(&self) -> u32 { self.half_move_clock }

    pub fn recompute_outcome(&mut self, history: &History) {
        self.outcome = Some(recompute_outcome(self, history));
    }

    pub fn get_outcome(&self) -> Option<OutcomeType> { self.outcome.clone() }
}


