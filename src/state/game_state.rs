use crate::board::Board;
use crate::board::board::UndoMove;
use crate::history::history::History;
use crate::piece::pieces::{Piece, Color, PieceType};
use crate::state::outcome::{recompute_outcome, OutcomeType};
use crate::state::castling::CastlingRights;

// ============================================================
// GAME STATE - Chess Rules + Game Management
// ============================================================

/// GameState represents a complete chess game position including:
/// - Board position (piece placement)
/// - Game rules state (castling rights, en passant, clocks)
/// - Turn management (active color, move numbers)
/// - Outcome tracking
///
/// The Board contains piece placement and basic operations.
/// GameState adds the chess rules layer on top.
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
    // ============================================================
    // CONSTRUCTION
    // ============================================================

    pub fn new() -> Self {
        GameState {
            board: Board::new(),
            active_color: Color::White,
            castling_rights: CastlingRights::all(),
            en_passant_target: None,
            half_move_clock: 0,
            full_move_number: 1,
            outcome: None
        }
    }

    pub fn new_from_existing_state(
        board: Board,
        active_color: Color,
        castling_rights: CastlingRights,
        en_passant_target: Option<(usize, usize)>,
        half_move_clock: u32,
        full_move_number: u32
    ) -> Self {
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

    /// Construct GameState from board and side, inferring castling rights
    pub fn from_board_and_side(board: Board, side: Color) -> Self {
        let mut rights_str = String::new();

        // White castling
        if matches!(board.get(0,4), Some(p) if p.get_color()==Color::White && p.get_type()==PieceType::King) {
            if matches!(board.get(0,7), Some(p) if p.get_color()==Color::White && p.get_type()==PieceType::Rook) {
                rights_str.push('K');
            }
            if matches!(board.get(0,0), Some(p) if p.get_color()==Color::White && p.get_type()==PieceType::Rook) {
                rights_str.push('Q');
            }
        }

        // Black castling
        if matches!(board.get(7,4), Some(p) if p.get_color()==Color::Black && p.get_type()==PieceType::King) {
            if matches!(board.get(7,7), Some(p) if p.get_color()==Color::Black && p.get_type()==PieceType::Rook) {
                rights_str.push('k');
            }
            if matches!(board.get(7,0), Some(p) if p.get_color()==Color::Black && p.get_type()==PieceType::Rook) {
                rights_str.push('q');
            }
        }

        if rights_str.is_empty() {
            rights_str.push('-');
        }

        let rights = CastlingRights::from_fen(&rights_str);
        GameState::new_from_existing_state(board, side, rights, None, 0, 1)
    }

    // ============================================================
    // BOARD ACCESS
    // ============================================================

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn mutable_board(&mut self) -> &mut Board {
        &mut self.board
    }

    // ============================================================
    // GAME STATE ACCESSORS
    // ============================================================

    pub fn active_color(&self) -> Color {
        self.active_color
    }

    pub fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
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

    pub fn get_outcome(&self) -> Option<OutcomeType> {
        self.outcome
    }

    // ============================================================
    // GAME STATE MUTATIONS
    // ============================================================

    pub fn set_en_passant_target(&mut self, target: Option<(usize, usize)>) {
        self.en_passant_target = target;
    }

    pub fn set_half_move_clock(&mut self, val: u32) {
        self.half_move_clock = val;
    }

    pub fn reset_half_move_clock(&mut self) {
        self.half_move_clock = 0;
    }

    pub fn increment_half_move_clock(&mut self) {
        self.half_move_clock += 1;
    }

    pub fn increment_full_move_number(&mut self) {
        self.full_move_number += 1;
    }

    pub fn switch_player_turn(&mut self) {
        self.active_color = match self.active_color {
            Color::White => Color::Black,
            Color::Black => Color::White,
        };

        if self.active_color == Color::White {
            self.increment_full_move_number();
        }
    }

    pub fn update_king_location(&mut self, color: Color, location: (usize, usize)) {
        self.board.set_king_location(color, location);
    }

    pub fn revoke_castling_rights_for_color(&mut self, color: Color) {
        match color {
            Color::White => self.castling_rights.revoke_white_castling(),
            Color::Black => self.castling_rights.revoke_black_castling(),
        }
    }

    pub fn recompute_outcome(&mut self, history: &History) {
        self.outcome = Some(recompute_outcome(self, history));
    }

    #[deprecated(note = "Use game_state.mutable_board() with manual promotion handling instead")]
    pub fn move_pawn(&mut self, from: (usize, usize), to: (usize, usize), promotion_piece: Option<Piece>) -> bool {
        let mut piece = self.board.get(from.0, from.1);
        if piece.is_none() {
            return false;
        }

        if promotion_piece.is_some() {
            piece = promotion_piece;
        }

        self.board.set(to.0, to.1, piece);
        self.board.set(from.0, from.1, None);
        true
    }
}

// ============================================================
// FAST MAKE/UNMAKE FOR SEARCH
// ============================================================

/// Undo information for perfect move reversal in search
#[derive(Clone, Copy)]
pub struct UndoGameState {
    pub(crate) board_undo: UndoMove,
    pub(crate) prev_active_color: Color,
    pub(crate) prev_castling_rights: CastlingRights,
    pub(crate) prev_en_passant_target: Option<(usize, usize)>,
    pub(crate) prev_half_move_clock: u32,
    pub(crate) prev_full_move_number: u32,
    pub(crate) ep_captured_sq: Option<(usize, usize)>,
    pub(crate) ep_captured_piece: Option<Piece>,
}

impl GameState {
    /// Fast move application for search - handles all chess rules
    pub fn make_move_fast(&mut self, from: (usize, usize), to: (usize, usize), promo: Option<char>) -> UndoGameState {
        let prev_active_color = self.active_color;
        let prev_castling_rights = self.castling_rights;
        let prev_en_passant_target = self.en_passant_target;
        let prev_half_move_clock = self.half_move_clock;
        let prev_full_move_number = self.full_move_number;

        let moving_piece = self.board.get(from.0, from.1);

        // Detect en passant capture before moving
        let mut ep_captured_sq: Option<(usize, usize)> = None;
        let mut ep_captured_piece: Option<Piece> = None;

        if let Some(p) = moving_piece {
            if p.get_type() == PieceType::Pawn {
                if let Some(ep) = self.en_passant_target {
                    if ep == to && self.board.get(to.0, to.1).is_none() && from.1 != to.1 {
                        let cap_row = if p.get_color() == Color::White { to.0 - 1 } else { to.0 + 1 };
                        ep_captured_sq = Some((cap_row, to.1));
                        ep_captured_piece = self.board.get(cap_row, to.1);
                    }
                }
            }
        }

        // Apply board move (handles castling and normal captures)
        let board_undo = self.board.make_move_simple(from, to, promo);

        // Clear en passant captured pawn
        if let (Some(sq), Some(_)) = (ep_captured_sq, ep_captured_piece) {
            self.board.clear(sq.0, sq.1);
        }

        // Handle promotion (make_move_simple already does this, but ensure correctness)
        if let (Some(p), Some(pc)) = (moving_piece, promo) {
            if p.get_type() == PieceType::Pawn {
                let promote_to = match pc {
                    'q' | 'Q' => PieceType::Queen,
                    'r' | 'R' => PieceType::Rook,
                    'b' | 'B' => PieceType::Bishop,
                    'n' | 'N' => PieceType::Knight,
                    _ => PieceType::Queen,
                };
                let promoted_piece = Piece::new(promote_to, p.get_color());
                self.board.set(to.0, to.1, Some(promoted_piece));

                if promoted_piece.get_type() == PieceType::King {
                    self.board.set_king_location(promoted_piece.get_color(), to);
                }
            }
        }

        // Update castling rights
        if let Some(p) = moving_piece {
            match (p.get_type(), p.get_color()) {
                (PieceType::King, Color::White) => {
                    self.castling_rights.revoke_white_castling();
                }
                (PieceType::King, Color::Black) => {
                    self.castling_rights.revoke_black_castling();
                }
                (PieceType::Rook, Color::White) => {
                    if from == (0, 0) {
                        self.castling_rights = CastlingRights {
                            white_kingside: self.castling_rights.white_kingside(),
                            white_queenside: false,
                            black_kingside: self.castling_rights.black_kingside(),
                            black_queenside: self.castling_rights.black_queenside()
                        };
                    } else if from == (0, 7) {
                        self.castling_rights = CastlingRights {
                            white_kingside: false,
                            white_queenside: self.castling_rights.white_queenside(),
                            black_kingside: self.castling_rights.black_kingside(),
                            black_queenside: self.castling_rights.black_queenside()
                        };
                    }
                }
                (PieceType::Rook, Color::Black) => {
                    if from == (7, 0) {
                        self.castling_rights = CastlingRights {
                            white_kingside: self.castling_rights.white_kingside(),
                            white_queenside: self.castling_rights.white_queenside(),
                            black_kingside: self.castling_rights.black_kingside(),
                            black_queenside: false
                        };
                    } else if from == (7, 7) {
                        self.castling_rights = CastlingRights {
                            white_kingside: self.castling_rights.white_kingside(),
                            white_queenside: self.castling_rights.white_queenside(),
                            black_kingside: false,
                            black_queenside: self.castling_rights.black_queenside()
                        };
                    }
                }
                _ => {}
            }
        }

        // Revoke castling rights if rook captured
        if board_undo.captured.is_some() {
            match to {
                (0, 0) => {
                    self.castling_rights = CastlingRights {
                        white_kingside: self.castling_rights.white_kingside(),
                        white_queenside: false,
                        black_kingside: self.castling_rights.black_kingside(),
                        black_queenside: self.castling_rights.black_queenside()
                    };
                }
                (0, 7) => {
                    self.castling_rights = CastlingRights {
                        white_kingside: false,
                        white_queenside: self.castling_rights.white_queenside(),
                        black_kingside: self.castling_rights.black_kingside(),
                        black_queenside: self.castling_rights.black_queenside()
                    };
                }
                (7, 0) => {
                    self.castling_rights = CastlingRights {
                        white_kingside: self.castling_rights.white_kingside(),
                        white_queenside: self.castling_rights.white_queenside(),
                        black_kingside: self.castling_rights.black_kingside(),
                        black_queenside: false
                    };
                }
                (7, 7) => {
                    self.castling_rights = CastlingRights {
                        white_kingside: self.castling_rights.white_kingside(),
                        white_queenside: self.castling_rights.white_queenside(),
                        black_kingside: false,
                        black_queenside: self.castling_rights.black_queenside()
                    };
                }
                _ => {}
            }
        }

        // Update en passant target
        self.en_passant_target = None;

        if let Some(p) = moving_piece {
            if p.get_type() == PieceType::Pawn {
                self.half_move_clock = 0;

                let start_row = if p.get_color() == Color::White { 1 } else { 6 };
                let dir = if p.get_color() == Color::White { 1isize } else { -1isize };

                if from.0 == start_row && (to.0 as isize) == (from.0 as isize + 2 * dir) && from.1 == to.1 {
                    let mid_row = (from.0 as isize + dir) as usize;
                    self.en_passant_target = Some((mid_row, from.1));
                }
            } else {
                self.half_move_clock = self.half_move_clock.saturating_add(1);
            }
        }

        // Reset clock on capture
        if board_undo.captured.is_some() || ep_captured_piece.is_some() {
            self.half_move_clock = 0;
        }

        // Switch side and increment move number
        self.active_color = match self.active_color {
            Color::White => Color::Black,
            Color::Black => Color::White
        };

        if self.active_color == Color::White {
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

    /// Fast move reversal for search
    pub fn unmake_move_fast(&mut self, u: UndoGameState) {
        // Restore board
        self.board.unmake_move_simple(u.board_undo);

        // Restore en passant captured pawn
        if let (Some(sq), Some(pc)) = (u.ep_captured_sq, u.ep_captured_piece) {
            self.board.set(sq.0, sq.1, Some(pc));

            if pc.get_type() == PieceType::King {
                self.board.set_king_location(pc.get_color(), sq);
            }
        }

        // Restore game state
        self.active_color = u.prev_active_color;
        self.castling_rights = u.prev_castling_rights;
        self.en_passant_target = u.prev_en_passant_target;
        self.half_move_clock = u.prev_half_move_clock;
        self.full_move_number = u.prev_full_move_number;

        #[cfg(debug_assertions)]
        {
            let wk = self.board.get_king_location(Color::White);
            let bk = self.board.get_king_location(Color::Black);
            debug_assert_eq!(wk, u.board_undo.prev_white_king, "White king location mismatch after unmake");
            debug_assert_eq!(bk, u.board_undo.prev_black_king, "Black king location mismatch after unmake");

            if let Some(k) = self.board.get(wk.0, wk.1) {
                debug_assert!(k.get_type() == PieceType::King && k.get_color() == Color::White, "Expected white king on its square after unmake");
            }
            if let Some(k) = self.board.get(bk.0, bk.1) {
                debug_assert!(k.get_type() == PieceType::King && k.get_color() == Color::Black, "Expected black king on its square after unmake");
            }
        }
    }
}
