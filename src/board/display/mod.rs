use std::fmt;
use crate::board::Board;
use crate::history::history::History;
use crate::piece::pieces::Color;


impl Board {
    pub fn get_board_display_string(&self, history: &History) -> String {
        // ANSI escape sequences for background colors (light/dark) and reset
        const RESET: &str = "\x1b[0m";
        const BG_LIGHT: &str = "\x1b[48;5;244m"; // light gray
        const BG_DARK: &str = "\x1b[48;5;239m";  // dark gray
        const BG_LAST_LIGHT: &str = "\x1b[48;5;248m"; // light gray highlight (closer to BG_LIGHT)
        const BG_LAST_DARK: &str = "\x1b[48;5;237m";  // slightly lighter gray (closer to BG_DARK)
        // Foreground colors for pieces
        const FG_WHITE: &str = "\x1b[97m"; // white pieces (white)
        const FG_BLACK: &str = "\x1b[30m"; // black pieces (black)

        let mut result = String::new();

        let last_move = if history.len() > 0 {
            history.get_move(history.len() - 1)
        } else {
            None
        };

        // File labels (top), spaced to match 3-character wide squares
        result.push_str("  "); // left margin for rank labels
        for file in 0..8 {
            let c = (b'a' + file as u8) as char;
            result.push_str(&format!(" {} ", c));
        }
        result.push('\n');

        for row in (0..8).rev() {
            // Rank label on the left
            result.push_str(&format!("{} ", row + 1));
            for col in 0..8 {
                // A1 (row 0, col 0) is dark; H1 (row 0, col 7) is light
                let is_dark = (row + col) % 2 == 0;

                let mut is_last_move = false;

                if last_move.is_some() {
                    let lmv = last_move.unwrap();
                    if (col == lmv.1.0.1 && row == lmv.1.0.0) {
                        is_last_move = true;
                    }
                    if (col == lmv.1.1.1 && row == lmv.1.1.0) {
                        is_last_move = true;
                    }
                };

                let bg = if is_dark {
                    if is_last_move {
                        BG_LAST_DARK
                    } else {
                        BG_DARK
                    }
                } else {
                    if is_last_move {
                        BG_LAST_LIGHT
                    } else {
                        BG_LIGHT
                    }
                };

                if let Some(piece) = self.squares()[row][col] {
                    let ch = piece.symbol();
                    let fg = match piece.get_color() {
                        Color::White => FG_WHITE,
                        Color::Black => FG_BLACK,
                    };
                    // Background + foreground + space + symbol + space + reset
                    result.push_str(&format!("{}{} {} {}", bg, fg, ch, RESET));
                } else {
                    let ch = '\u{2003}';
                    result.push_str(&format!("{}{} {} {}", bg, bg, ch, RESET));
                }
            }
            // Rank label on the right
            result.push_str(&format!(" {}\n", row + 1));
        }
        // File labels at the bottom, spaced to match 3-character wide squares
        result.push_str("  ");
        for file in 0..8 {
            let c = (b'a' + file as u8) as char;
            result.push_str(&format!(" {} ", c));
        }
        result.push('\n');

        result
    }
}
