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
        // ANSI escape sequences for background colors (light/dark) and reset
        const RESET: &str = "\x1b[0m";
        const BG_LIGHT: &str = "\x1b[48;5;244m"; // light gray
        const BG_DARK: &str = "\x1b[48;5;239m";  // dark gray
        // Foreground colors for pieces
        const FG_WHITE: &str = "\x1b[97m"; // white pieces (white)
        const FG_BLACK: &str = "\x1b[30m"; // black pieces (black)

        // File labels (top), spaced to match 3-character wide squares
        write!(f, "  ")?; // left margin for rank labels
        for file in 0..8 {
            let c = (b'a' + file as u8) as char;
            write!(f, " {} ", c)?;
        }
        writeln!(f)?;
        for row in (0..8).rev() {
            // Rank label on the left
            write!(f, "{} ", row + 1)?;
            for col in 0..8 {
                // A1 (row 0, col 0) is dark; H1 (row 0, col 7) is light
                let is_dark = (row + col) % 2 == 0;
                let bg = if is_dark { BG_DARK } else { BG_LIGHT };

                if let Some(piece) = self.squares[row][col] {
                    let ch = piece.symbol();
                    let fg = match piece.get_color() {
                        Color::White => FG_WHITE,
                        Color::Black => FG_BLACK,
                    };
                    // Background + foreground + space + symbol + space + reset
                    write!(f, "{}{} {} {}", bg, fg, ch, RESET)?;
                } else {
                    // Empty square: just background with spaces
                    write!(f, "{}   {}", bg, RESET)?;
                }
            }
            // Rank label on the right
            writeln!(f, " {}", row + 1)?;
        }
        // File labels at the bottom, spaced to match 3-character wide squares
        write!(f, "  ")?;
        for file in 0..8 {
            let c = (b'a' + file as u8) as char;
            write!(f, " {} ", c)?;
        }
        writeln!(f)
    }
}
