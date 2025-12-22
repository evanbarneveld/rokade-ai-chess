use crate::piece::pieces::{Piece, PieceType, Color};

#[derive(Debug, Clone)]
pub struct Board {
    squares: [[Option<Piece>; 8]; 8],
}

impl Board {
    pub fn new() -> Self {
        let mut board = Board {
            squares: [[None; 8]; 8],
        };
        board.setup_initial_position();
        board
    }

    pub fn empty() -> Self {
        Board {
            squares: [[None; 8]; 8],
        }
    }

    pub fn squares(&self) -> &[[Option<Piece>; 8]] {
        &self.squares
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

    /// Basic check to see if a move is invalid, regardless of the type of the piece.
    /// A move is invalid when:
    ///
    /// - the coordinates are out of range
    /// - the source square is empty, there is nothing to move
    /// - the source square is occupied by a piece of the other player
    /// - the target square is occupied by a piece of the other player, but the move is not a capture move
    /// - the target square is occupied by a piece of the current player
    /// - the target square is occupied but the move is not a capture
    /// - the target square is empty (use the en-passant target if needed)
    pub fn move_from_and_to_validation_check(&self, from: (usize, usize), to: (usize, usize), active_color:Color, is_capture:bool, is_pawn_move:bool, en_passant_target:Option<(usize, usize)>) -> bool {

        if from.0 > 7 || from.1 > 7 || to.0 > 7 || to.1 > 7 { return false; }

        let source_piece = self.get(from.0, from.1);
        if source_piece.is_none() { return false; }
        if source_piece.unwrap().get_color() != active_color { return false; }

        let target_piece = self.get(to.0, to.1);

        if target_piece.is_some() {
            if !is_capture { return false; }
            if target_piece.unwrap().get_color() == active_color { return false; }
        } else {
            // no piece on 'to' square
            if is_capture {
                // if the move is an en-passant capture, then check the en-passant target square
                if is_pawn_move && en_passant_target.is_some() {
                    // is the to square the en-passant target square? that square must be empty for a valid en-passant capture
                    let ep_target = en_passant_target.unwrap();
                    if to == ep_target { return true; }
                }
                return false;
            }
        }
        true
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
}
