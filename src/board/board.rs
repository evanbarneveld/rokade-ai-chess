use crate::piece::pieces::{Piece, PieceType, Color, piece_value_cp};
use crate::board::checks::move_squares_validity::move_from_and_to_validation_check;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;


const PHASE_KNIGHT: i32 = 1;
const PHASE_BISHOP: i32 = 1;
const PHASE_ROOK: i32 = 2;
const PHASE_QUEEN: i32 = 4;

#[derive(Debug, Clone, Copy)]
pub struct Board {
    //these are the squares on the board
    squares: [[Option<Piece>; 8]; 8],
    //keep track of the king locations
    white_king_location: (usize, usize),
    black_king_location: (usize, usize),
}

#[derive(Clone, Copy)]
pub struct UndoMove {
    from: (usize, usize),
    to: (usize, usize),
    moved: Option<Piece>,
    captured: Option<Piece>,
    // Save king locations to restore accurately on unmake
    prev_white_king: (usize, usize),
    prev_black_king: (usize, usize),
}

impl Board {
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

    pub fn squares(&self) -> &[[Option<Piece>; 8]] {
        &self.squares
    }

    pub fn get_king_location(&self, color:Color) -> (usize, usize) {
        if color == Color::White { self.white_king_location } else { self.black_king_location }
    }

    pub fn set_king_location(&mut self, color:Color, location: (usize, usize)) {
        if color == Color::White { self.white_king_location = location } else { self.black_king_location = location }
    }

    pub fn find_and_set_location_of_kings(&mut self) {
        self.set_king_location(Color::White, self.find_king_location(Color::White).unwrap());
        self.set_king_location(Color::Black, self.find_king_location(Color::Black).unwrap());
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

        self.find_and_set_location_of_kings()
    }

    pub fn get(&self, row: usize, col: usize) -> Option<Piece> {
        self.squares[row][col]
    }

    pub fn set(&mut self, row: usize, col: usize, piece: Option<Piece>) {
        self.squares[row][col] = piece;
    }

    pub fn clear(&mut self, row: usize, col: usize) {
        self.squares[row][col] = None;
    }

    pub fn move_from_and_to_validation_check(&self, from: (usize, usize), to: (usize, usize), active_color:Color, is_capture:bool, is_pawn_move:bool, en_passant_target:Option<(usize, usize)>) -> bool {
        move_from_and_to_validation_check(self, from, to, active_color, is_capture, is_pawn_move, en_passant_target)
    }

    pub fn board_square_has_piece_of_opposite_color(&self, to: (usize, usize), active_color:Color) -> bool {
        let target_piece = self.get(to.0, to.1);
        if target_piece.is_some() && target_piece.unwrap().get_color() != active_color { return true; }
        false
    }

    pub fn board_square_is_empty(&self, location: (usize, usize)) -> bool {
        self.get(location.0, location.1).is_none()
    }
    
    pub fn move_piece(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        if let Some(piece) = self.get(from.0, from.1) {
            self.set(to.0, to.1, Some(piece));
            self.set(from.0, from.1, None);
            true
        } else {
            false
        }
    }
    pub fn move_pawn(&mut self, from: (usize, usize), to: (usize, usize), promotion_piece: Option<Piece>) -> bool {
        let mut piece = self.get(from.0, from.1);

        if piece.is_none() { return false; }

        if promotion_piece.is_some() {
            piece = promotion_piece;
        }

        self.set(to.0, to.1, piece);
        self.set(from.0, from.1, None);
        true
    }

    pub fn find_king_location(&self, color:Color) -> Option<(usize, usize)> {
        //iterate the squares and find the king location
        //iterate ranks
        for row in 0..8 {
            for col in 0 .. 8 {
                if let Some(piece) = self.get(row, col) {
                    if piece.get_type() == PieceType::King && piece.get_color() == color { return Some((row, col)); }
                }
            }
        }
        None
    }

    pub fn is_side_in_check(&mut self, side: Color) -> bool {
        let king_sq = self.get_king_location(side);
        is_square_attacked_by_opponent(self, king_sq, side)
    }

    // --- Lightweight helpers for selective extensions ---
    // Compute a simple material-based game phase similar to evaluator (0..24)
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
        if phase < 0 {
            0
        } else if phase > 24 {
            24
        } else {
            phase
        }
    }


    #[inline]
    pub(crate) fn make_move_simple(&mut self, from: (usize, usize), to: (usize, usize)) -> UndoMove {
        let moved = self.get(from.0, from.1);
        let captured = self.get(to.0, to.1);
        // snapshot king locations before move
        let prev_white_king = self.get_king_location(Color::White);
        let prev_black_king = self.get_king_location(Color::Black);
        // apply move directly
        self.set(to.0, to.1, moved);
        self.set(from.0, from.1, None);
        // update king location cache if a king moved
        if let Some(p) = moved {
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
        }
    }

    #[inline]
    pub(crate) fn unmake_move_simple(&mut self, undo: UndoMove) {
        // restore original squares
        self.set(undo.from.0, undo.from.1, undo.moved);
        self.set(undo.to.0, undo.to.1, undo.captured);
        // restore king location cache from snapshot
        self.set_king_location(Color::White, undo.prev_white_king);
        self.set_king_location(Color::Black, undo.prev_black_king);
    }

    // Determine if a pawn at (row,col) for color is a passed pawn (no enemy pawns ahead on same/adjacent files)
    #[inline]
    pub fn is_passed_pawn_simple(&self, row: usize, col: usize, color: Color) -> bool {
        let dir: i32 = if color == Color::White { 1 } else { -1 };
        let mut r = row as i32 + dir;
        while r >= 0 && r < 8 {
            for dc in [-1i32, 0, 1] {
                let nc = col as i32 + dc;
                if nc < 0 || nc >= 8 {
                    continue;
                }
                if let Some(p) = self.get(r as usize, nc as usize) {
                    if p.get_color() != color && p.get_type() == PieceType::Pawn {
                        return false;
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
        let v = victim
            .map(|p| piece_value_cp(p.get_type()))
            .unwrap_or(0);
        let a = attacker
            .map(|p| piece_value_cp(p.get_type()))
            .unwrap_or(0);
        // Higher is better for ordering. Captures first; quiets get 0 or negative.
        if victim.is_some() { v * 100 - a } else { -1 }
    }

}


