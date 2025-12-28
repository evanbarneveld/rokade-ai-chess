use crate::piece::pieces::{Piece, PieceType, Color};
use crate::board::checks::move_squares_validity::move_from_and_to_validation_check;

#[derive(Debug, Clone, Copy)]
pub struct Board {
    //these are the squares on the board
    squares: [[Option<Piece>; 8]; 8],
    //keep track of the king locations
    white_king_location: (usize, usize),
    black_king_location: (usize, usize),
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
}
