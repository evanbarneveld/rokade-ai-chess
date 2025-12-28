use std::fmt;
use crate::board::Board;
use crate::piece::pieces::Color;

/// Display the chess board using ANSI escape sequences and Unicode chess symbols
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

                if let Some(piece) = self.squares()[row][col] {
                    let ch = piece.symbol();
                    let fg = match piece.get_color() {
                        Color::White => FG_WHITE,
                        Color::Black => FG_BLACK,
                    };
                    // Background + foreground + space + symbol + space + reset
                    write!(f, "{}{} {} {}", bg, fg, ch, RESET)?;
                } else {
                    let ch = '\u{2003}';
                    write!(f, "{}{} {} {}", bg, bg, ch, RESET)?;
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
