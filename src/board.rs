use std::fmt;
use crate::pieces::{Piece, PieceType, Color};

#[derive(Debug)]
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

    pub fn move_piece(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        if let Some(piece) = self.get(from.0, from.1) {
            self.set(to.0, to.1, Some(piece));
            self.set(from.0, from.1, None);
            true
        } else {
            false
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "  a b c d e f g h")?;
        for row in (0..8).rev() {
            write!(f, "{} ", row + 1)?;
            for col in 0..8 {
                if let Some(piece) = self.squares[row][col] {
                    write!(f, "{} ", piece.symbol())?;
                } else {
                    write!(f, "· ")?;
                }
            }
            writeln!(f, "{}", row + 1)?;
        }
        writeln!(f, "  a b c d e f g h")
    }
}
