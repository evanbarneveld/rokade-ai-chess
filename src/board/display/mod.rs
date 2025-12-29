use crate::board::Board;
use crate::history::history::History;
use crate::piece::pieces::Color;


impl Board {
    pub fn get_board_display_string(&self, history: Option<&History>) -> String {
        // ANSI escape sequences for background colors (light/dark) and reset
        const RESET: &str = "\x1b[0m";
        const BG_LIGHT: &str = "\x1b[48;5;248m"; // light gray
        const BG_DARK: &str = "\x1b[48;5;242m";  // dark gray
        const BG_LAST_LIGHT: &str = "\x1b[48;5;252m"; // slightly lighter than light gray
        const BG_LAST_DARK: &str = "\x1b[48;5;238m";  // slightly lighter than dark gray
        // Foreground colors for pieces
        const FG_WHITE: &str = "\x1b[97m"; // white pieces (white)
        const FG_BLACK: &str = "\x1b[30m"; // black pieces (black)

        let mut result = String::new();

        let mut last_move = None;

        if history.is_some() {
            let h = history.unwrap();
            if h.len() > 0 {
               last_move = h.get_move(h.len() - 1)
            }
        }

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
                    if col == lmv.1.0.1 && row == lmv.1.0.0 {
                        is_last_move = true;
                    }
                    if col == lmv.1.1.1 && row == lmv.1.1.0 {
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
