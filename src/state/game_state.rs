use crate::board::Board;
use crate::board::board::UndoMove;
use crate::history::history::History;
use crate::piece::pieces::{Piece, Color, PieceType};
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

    // Lightweight constructor for validators: build a GameState from a Board snapshot and side to move.
    // Castling rights are inferred from piece placement on initial squares (approximation).
    pub fn from_board_and_side(board: Board, side: Color) -> Self {
        // Infer castling rights string based on king/rook presence at start squares
        let mut rights_str = String::new();
        // White
        if matches!(board.get(0,4), Some(p) if p.get_color()==Color::White && p.get_type()==PieceType::King) {
            if matches!(board.get(0,7), Some(p) if p.get_color()==Color::White && p.get_type()==PieceType::Rook) {
                rights_str.push('K');
            }
            if matches!(board.get(0,0), Some(p) if p.get_color()==Color::White && p.get_type()==PieceType::Rook) {
                rights_str.push('Q');
            }
        }
        // Black
        if matches!(board.get(7,4), Some(p) if p.get_color()==Color::Black && p.get_type()==PieceType::King) {
            if matches!(board.get(7,7), Some(p) if p.get_color()==Color::Black && p.get_type()==PieceType::Rook) {
                rights_str.push('k');
            }
            if matches!(board.get(7,0), Some(p) if p.get_color()==Color::Black && p.get_type()==PieceType::Rook) {
                rights_str.push('q');
            }
        }
        if rights_str.is_empty() { rights_str.push('-'); }
        let rights = CastlingRights::from_fen(&rights_str);
        GameState::new_from_existing_state(board, side, rights, None, 0, 1)
    }
}


// Fast make/unmake specifically for perft and search hot paths.
// Applies a move on the current GameState and returns an undo snapshot for perfect restoration.
#[derive(Clone, Copy)]
pub struct UndoGameState {
    board_undo: UndoMove,
    prev_active_color: Color,
    prev_castling_rights: CastlingRights,
    prev_en_passant_target: Option<(usize, usize)>,
    prev_half_move_clock: u32,
    prev_full_move_number: u32,
    // En passant captured pawn (if any)
    ep_captured_sq: Option<(usize, usize)>,
    ep_captured_piece: Option<Piece>,
}

impl GameState {
    // Single entry-point "make": handles captures (incl. EP), promotions, castling rights, clocks, EP target, and side to move.
    pub fn make_move_fast(&mut self, from: (usize, usize), to: (usize, usize), promo: Option<char>) -> UndoGameState {
        let prev_active_color = self.active_color;
        let prev_castling_rights = self.castling_rights;
        let prev_en_passant_target = self.en_passant_target;
        let prev_half_move_clock = self.half_move_clock;
        let prev_full_move_number = self.full_move_number;

        let moving_piece = self.board.get(from.0, from.1);

        // Determine if this is an en passant capture before moving
        let mut ep_captured_sq: Option<(usize, usize)> = None;
        let mut ep_captured_piece: Option<Piece> = None;
        if let Some(p) = moving_piece {
            if p.get_type() == PieceType::Pawn {
                if let Some(ep) = self.en_passant_target {
                    if ep == to && self.board.get(to.0, to.1).is_none() && from.1 != to.1 {
                        // EP capture: captured pawn sits behind the target square
                        let cap_row = if p.get_color() == Color::White { to.0 - 1 } else { to.0 + 1 };
                        ep_captured_sq = Some((cap_row, to.1));
                        ep_captured_piece = self.board.get(cap_row, to.1);
                        // Remove captured pawn now (before board.move) would be messy; do it after move
                    }
                }
            }
        }

        // Apply the board move (handles king+rook relocation for castling and normal captures on destination)
        let board_undo = self.board.make_move_simple(from, to);

        // If this was an en passant capture, clear the captured pawn square
        if let (Some(sq), Some(_pc)) = (ep_captured_sq, ep_captured_piece) {
            self.board.clear(sq.0, sq.1);
        }

        // Handle promotion: replace pawn at destination with promoted piece if requested
        if let (Some(p), Some(pc)) = (moving_piece, promo) {
            if p.get_type() == PieceType::Pawn {
                let promote_to = match pc {
                    'q' | 'Q' => PieceType::Queen,
                    'r' | 'R' => PieceType::Rook,
                    'b' | 'B' => PieceType::Bishop,
                    'n' | 'N' => PieceType::Knight,
                    _ => PieceType::Queen,
                };
                self.board.set(to.0, to.1, Some(Piece::new(promote_to, p.get_color())));
            }
        }

        // Update castling rights due to king/rook moves or rook capture from original squares
        if let Some(p) = moving_piece {
            match (p.get_type(), p.get_color()) {
                (PieceType::King, Color::White) => {
                    self.castling_rights.revoke_white_castling();
                }
                (PieceType::King, Color::Black) => {
                    self.castling_rights.revoke_black_castling();
                }
                (PieceType::Rook, Color::White) => {
                    if from == (0, 0) { // a1
                        self.castling_rights = CastlingRights { white_kingside: self.castling_rights.white_kingside(), white_queenside: false, black_kingside: self.castling_rights.black_kingside(), black_queenside: self.castling_rights.black_queenside() };
                    } else if from == (0, 7) { // h1
                        self.castling_rights = CastlingRights { white_kingside: false, white_queenside: self.castling_rights.white_queenside(), black_kingside: self.castling_rights.black_kingside(), black_queenside: self.castling_rights.black_queenside() };
                    }
                }
                (PieceType::Rook, Color::Black) => {
                    if from == (7, 0) { // a8
                        self.castling_rights = CastlingRights { white_kingside: self.castling_rights.white_kingside(), white_queenside: self.castling_rights.white_queenside(), black_kingside: self.castling_rights.black_kingside(), black_queenside: false };
                    } else if from == (7, 7) { // h8
                        self.castling_rights = CastlingRights { white_kingside: self.castling_rights.white_kingside(), white_queenside: self.castling_rights.white_queenside(), black_kingside: false, black_queenside: self.castling_rights.black_queenside() };
                    }
                }
                _ => {}
            }
        }
        // If a rook on initial square was captured (including EP is never rook), revoke corresponding rights
        if board_undo.captured.is_some() {
            let cap_sq = to;
            match cap_sq {
                (0, 0) => { // a1 rook captured
                    self.castling_rights = CastlingRights { white_kingside: self.castling_rights.white_kingside(), white_queenside: false, black_kingside: self.castling_rights.black_kingside(), black_queenside: self.castling_rights.black_queenside() };
                }
                (0, 7) => { // h1 rook captured
                    self.castling_rights = CastlingRights { white_kingside: false, white_queenside: self.castling_rights.white_queenside(), black_kingside: self.castling_rights.black_kingside(), black_queenside: self.castling_rights.black_queenside() };
                }
                (7, 0) => { // a8 rook captured
                    self.castling_rights = CastlingRights { white_kingside: self.castling_rights.white_kingside(), white_queenside: self.castling_rights.white_queenside(), black_kingside: self.castling_rights.black_kingside(), black_queenside: false };
                }
                (7, 7) => { // h8 rook captured
                    self.castling_rights = CastlingRights { white_kingside: self.castling_rights.white_kingside(), white_queenside: self.castling_rights.white_queenside(), black_kingside: false, black_queenside: self.castling_rights.black_queenside() };
                }
                _ => {}
            }
        }

        // Update en passant target: only set for double pawn pushes, else clear
        self.en_passant_target = None;
        if let Some(p) = moving_piece {
            if p.get_type() == PieceType::Pawn {
                // Reset half-move clock for pawn move
                self.half_move_clock = 0;
                let start_row = if p.get_color() == Color::White { 1 } else { 6 };
                let dir = if p.get_color() == Color::White { 1isize } else { -1isize };
                if from.0 == start_row && (to.0 as isize) == (from.0 as isize + 2*dir) && from.1 == to.1 {
                    // double push; ep target is the square jumped over
                    let mid_row = (from.0 as isize + dir) as usize;
                    self.en_passant_target = Some((mid_row, from.1));
                }
            } else {
                // Non-pawn move: increment half-move clock; reset on capture handled below
                self.half_move_clock = self.half_move_clock.saturating_add(1);
            }
        }

        // Capture resets half-move clock
        if board_undo.captured.is_some() || ep_captured_piece.is_some() {
            self.half_move_clock = 0;
        }

        // Switch side to move and increment full-move number when Black just moved
        self.active_color = match self.active_color { Color::White => Color::Black, Color::Black => Color::White };
        if self.active_color == Color::White { // just completed a black move
            self.full_move_number += 1;
        }

        UndoGameState {
            board_undo,
            prev_active_color,
            prev_castling_rights,
            prev_en_passant_target,
            prev_half_move_clock,
            prev_full_move_number,
            ep_captured_sq,
            ep_captured_piece,
        }
    }

    // Complementary unmake: fully restore GameState to its previous snapshot
    pub fn unmake_move_fast(&mut self, u: UndoGameState) {
        // Restore board state first (moves pieces back, handles un-castling)
        self.board.unmake_move_simple(u.board_undo);
        // If an en-passant capture occurred, restore the captured pawn
        if let (Some(sq), Some(pc)) = (u.ep_captured_sq, u.ep_captured_piece) {
            self.board.set(sq.0, sq.1, Some(pc));
        }
        // Restore metadata
        self.active_color = u.prev_active_color;
        self.castling_rights = u.prev_castling_rights;
        self.en_passant_target = u.prev_en_passant_target;
        self.half_move_clock = u.prev_half_move_clock;
        self.full_move_number = u.prev_full_move_number;
    }
}


