use crate::piece::pieces::{Piece, PieceType, Color, piece_value_cp};
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;

const PHASE_KNIGHT: i32 = 1;
const PHASE_BISHOP: i32 = 1;
const PHASE_ROOK: i32 = 2;
const PHASE_QUEEN: i32 = 4;

// ============================================================
// BOARD - Pure Data Structure + Basic Operations
// ============================================================

/// Board represents the 8x8 chess board with pieces and king locations.
/// This is a pure data structure focused on piece placement and basic queries.
/// Game rules (castling, en passant, turn management) belong in GameState.
#[derive(Debug, Clone, Copy)]
pub struct Board {
    squares: [[Option<Piece>; 8]; 8],
    white_king_location: (usize, usize),
    black_king_location: (usize, usize),
}

#[derive(Clone, Copy)]
pub struct UndoMove {
    pub(crate) from: (usize, usize),
    pub(crate) to: (usize, usize),
    pub(crate) moved: Option<Piece>,
    pub(crate) captured: Option<Piece>,
    pub(crate) prev_white_king: (usize, usize),
    pub(crate) prev_black_king: (usize, usize),
    pub(crate) castle_rook_from: Option<(usize, usize)>,
    pub(crate) castle_rook_to: Option<(usize, usize)>,
}

impl Board {
    // ============================================================
    // CONSTRUCTION
    // ============================================================

    pub fn new() -> Self {
        let mut board = Board {
            squares: [[None; 8]; 8],
            white_king_location: (0, 0),
            black_king_location: (0, 0)
        };
        board.setup_initial_position();
        board
    }

    pub fn empty() -> Self {
        Board {
            squares: [[None; 8]; 8],
            white_king_location: (0, 0),
            black_king_location: (0, 0)
        }
    }

    fn setup_initial_position(&mut self) {
        // Setup pawns
        for col in 0..8 {
            self.squares[1][col] = Some(Piece::new(PieceType::Pawn, Color::White));
            self.squares[6][col] = Some(Piece::new(PieceType::Pawn, Color::Black));
        }

        // Setup other pieces
        let back_rank = [
            PieceType::Rook, PieceType::Knight, PieceType::Bishop, PieceType::Queen,
            PieceType::King, PieceType::Bishop, PieceType::Knight, PieceType::Rook,
        ];

        for (col, &piece_type) in back_rank.iter().enumerate() {
            self.squares[0][col] = Some(Piece::new(piece_type, Color::White));
            self.squares[7][col] = Some(Piece::new(piece_type, Color::Black));
        }

        self.find_and_set_location_of_kings();
    }

    // ============================================================
    // BASIC ACCESSORS
    // ============================================================

    pub fn squares(&self) -> &[[Option<Piece>; 8]] {
        &self.squares
    }

    #[inline]
    pub fn get(&self, row: usize, col: usize) -> Option<Piece> {
        self.squares[row][col]
    }

    #[inline]
    pub fn set(&mut self, row: usize, col: usize, piece: Option<Piece>) {
        self.squares[row][col] = piece;
    }

    #[inline]
    pub fn clear(&mut self, row: usize, col: usize) {
        self.squares[row][col] = None;
    }

    // ============================================================
    // PIECE ITERATION
    // ============================================================

    /// Iterate over all pieces on the board with their positions
    pub fn iter_pieces(&self) -> impl Iterator<Item = ((usize, usize), Piece)> + '_ {
        self.squares.iter().enumerate().flat_map(|(r, row)| {
            row.iter().enumerate().filter_map(move |(c, piece)| {
                piece.map(|p| ((r, c), p))
            })
        })
    }

    /// Iterate over pieces of a specific color
    pub fn iter_pieces_of_color(&self, color: Color) -> impl Iterator<Item = ((usize, usize), Piece)> + '_ {
        self.iter_pieces().filter(move |(_, p)| p.get_color() == color)
    }

    /// Iterate over pieces of a specific type
    pub fn iter_pieces_of_type(&self, piece_type: PieceType) -> impl Iterator<Item = ((usize, usize), Piece)> + '_ {
        self.iter_pieces().filter(move |(_, p)| p.get_type() == piece_type)
    }

    // ============================================================
    // KING LOCATION MANAGEMENT
    // ============================================================

    #[inline]
    pub fn get_king_location(&self, color: Color) -> (usize, usize) {
        match color {
            Color::White => self.white_king_location,
            Color::Black => self.black_king_location,
        }
    }

    #[inline]
    pub fn set_king_location(&mut self, color: Color, location: (usize, usize)) {
        match color {
            Color::White => self.white_king_location = location,
            Color::Black => self.black_king_location = location,
        }
    }

    pub fn find_king_location(&self, color: Color) -> Option<(usize, usize)> {
        for row in 0..8 {
            for col in 0..8 {
                if let Some(piece) = self.get(row, col) {
                    if piece.get_type() == PieceType::King && piece.get_color() == color {
                        return Some((row, col));
                    }
                }
            }
        }
        None
    }

    pub fn find_and_set_location_of_kings(&mut self) {
        if let Some(wk) = self.find_king_location(Color::White) {
            self.set_king_location(Color::White, wk);
        }
        if let Some(bk) = self.find_king_location(Color::Black) {
            self.set_king_location(Color::Black, bk);
        }
    }

    // ============================================================
    // BOARD QUERIES
    // ============================================================

    /// Check if a square has a piece of the opposite color
    #[inline]
    pub fn has_opposite_color_piece(&self, sq: (usize, usize), color: Color) -> bool {
        matches!(self.get(sq.0, sq.1), Some(p) if p.get_color() != color)
    }

    /// Check if a square is empty
    #[inline]
    pub fn is_empty(&self, sq: (usize, usize)) -> bool {
        self.get(sq.0, sq.1).is_none()
    }

    /// Check if a side is in check
    pub fn is_side_in_check(&mut self, side: Color) -> bool {
        let king_sq = self.get_king_location(side);
        is_square_attacked_by_opponent(self, king_sq, side)
    }

    // ============================================================
    // MOVE OPERATIONS
    // ============================================================

    /// Move a piece from one square to another (basic operation, no game rules)
    pub fn move_piece_basic(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        if let Some(piece) = self.get(from.0, from.1) {
            self.set(to.0, to.1, Some(piece));
            self.set(from.0, from.1, None);

            // Update king location if moving a king
            if piece.get_type() == PieceType::King {
                self.set_king_location(piece.get_color(), to);
            }
            true
        } else {
            false
        }
    }

    /// Make a move with full castling and promotion support (for search/perft)
    #[inline]
    pub(crate) fn make_move_simple(&mut self, from: (usize, usize), to: (usize, usize), promo: Option<char>) -> UndoMove {
        let moved = self.get(from.0, from.1);
        let captured = self.get(to.0, to.1);
        let prev_white_king = self.get_king_location(Color::White);
        let prev_black_king = self.get_king_location(Color::Black);

        let mut castle_rook_from: Option<(usize, usize)> = None;
        let mut castle_rook_to: Option<(usize, usize)> = None;

        // Handle castling
        if let Some(p) = moved {
            if p.get_type() == PieceType::King {
                let dr = if from.0 > to.0 { from.0 - to.0 } else { to.0 - from.0 };
                let dc = if from.1 > to.1 { from.1 - to.1 } else { to.1 - from.1 };

                if dr == 0 && dc == 2 && from.1 == 4 {
                    // Kingside castling
                    if to.1 == 6 {
                        let rf = (from.0, 7);
                        let rt = (from.0, 5);
                        if let Some(rook) = self.get(rf.0, rf.1) {
                            if rook.get_type() == PieceType::Rook {
                                self.set(rt.0, rt.1, Some(rook));
                                self.set(rf.0, rf.1, None);
                                castle_rook_from = Some(rf);
                                castle_rook_to = Some(rt);
                            }
                        }
                    }
                    // Queenside castling
                    else if to.1 == 2 {
                        let rf = (from.0, 0);
                        let rt = (from.0, 3);
                        if let Some(rook) = self.get(rf.0, rf.1) {
                            if rook.get_type() == PieceType::Rook {
                                self.set(rt.0, rt.1, Some(rook));
                                self.set(rf.0, rf.1, None);
                                castle_rook_from = Some(rf);
                                castle_rook_to = Some(rt);
                            }
                        }
                    }
                }
            }
        }

        // Handle promotion and move piece
        if let Some(mut p) = moved {
            if p.get_type() == PieceType::Pawn {
                if let Some(pc) = promo {
                    let pt = match pc {
                        'q' => PieceType::Queen,
                        'r' => PieceType::Rook,
                        'b' => PieceType::Bishop,
                        'n' => PieceType::Knight,
                        _ => p.get_type(),
                    };
                    p = Piece::new(pt, p.get_color());
                } else if (p.get_color() == Color::White && to.0 == 7) || (p.get_color() == Color::Black && to.0 == 0) {
                    // Auto-promote to queen
                    p = Piece::new(PieceType::Queen, p.get_color());
                }
            }

            self.set(to.0, to.1, Some(p));
            self.set(from.0, from.1, None);

            if p.get_type() == PieceType::King {
                self.set_king_location(p.get_color(), to);
            }
        }

        UndoMove {
            from,
            to,
            moved,
            captured,
            prev_white_king,
            prev_black_king,
            castle_rook_from,
            castle_rook_to,
        }
    }

    #[inline]
    pub(crate) fn unmake_move_simple(&mut self, undo: UndoMove) {
        // Restore original squares
        self.set(undo.from.0, undo.from.1, undo.moved);
        self.set(undo.to.0, undo.to.1, undo.captured);

        // Restore castling rook if needed
        if let (Some(rf), Some(rt)) = (undo.castle_rook_from, undo.castle_rook_to) {
            if let Some(rook) = self.get(rt.0, rt.1) {
                if rook.get_type() == PieceType::Rook {
                    self.set(rf.0, rf.1, Some(rook));
                    self.set(rt.0, rt.1, None);
                }
            }
        }

        // Restore king locations
        self.set_king_location(Color::White, undo.prev_white_king);
        self.set_king_location(Color::Black, undo.prev_black_king);
    }

    // ============================================================
    // EVALUATION HELPERS
    // ============================================================

    #[inline]
    pub fn game_phase_light(&self) -> i32 {
        let mut phase: i32 = 0;
        for r in 0..8 {
            for c in 0..8 {
                if let Some(p) = self.get(r, c) {
                    phase += match p.get_type() {
                        PieceType::Knight => PHASE_KNIGHT,
                        PieceType::Bishop => PHASE_BISHOP,
                        PieceType::Rook => PHASE_ROOK,
                        PieceType::Queen => PHASE_QUEEN,
                        _ => 0,
                    };
                }
            }
        }
        phase.clamp(0, 24)
    }

    #[inline]
    pub fn is_passed_pawn_simple(&self, row: usize, col: usize, color: Color) -> bool {
        let dir: i32 = if color == Color::White { 1 } else { -1 };
        let mut r = row as i32 + dir;

        while r >= 0 && r < 8 {
            for dc in [-1, 0, 1] {
                let nc = col as i32 + dc;
                if nc >= 0 && nc < 8 {
                    if let Some(p) = self.get(r as usize, nc as usize) {
                        if p.get_color() != color && p.get_type() == PieceType::Pawn {
                            return false;
                        }
                    }
                }
            }
            r += dir;
        }
        true
    }

    #[inline]
    pub fn move_score_mvv_lva(&self, from: (usize, usize), to: (usize, usize)) -> i32 {
        let victim = self.get(to.0, to.1);
        let attacker = self.get(from.0, from.1);
        let v = victim.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
        let a = attacker.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);

        if victim.is_some() {
            v * 100 - a
        } else {
            -1
        }
    }

    #[inline]
    pub fn is_square_pawn_attacked_by(&self, attacker: Color, sq: (usize, usize)) -> bool {
        let (r, c) = sq;

        if attacker == Color::White {
            if r >= 1 {
                if c >= 1 {
                    if let Some(p) = self.get(r - 1, c - 1) {
                        if p.get_color() == attacker && p.get_type() == PieceType::Pawn {
                            return true;
                        }
                    }
                }
                if c + 1 < 8 {
                    if let Some(p) = self.get(r - 1, c + 1) {
                        if p.get_color() == attacker && p.get_type() == PieceType::Pawn {
                            return true;
                        }
                    }
                }
            }
        } else {
            if r + 1 < 8 {
                if c >= 1 {
                    if let Some(p) = self.get(r + 1, c - 1) {
                        if p.get_color() == attacker && p.get_type() == PieceType::Pawn {
                            return true;
                        }
                    }
                }
                if c + 1 < 8 {
                    if let Some(p) = self.get(r + 1, c + 1) {
                        if p.get_color() == attacker && p.get_type() == PieceType::Pawn {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

}
