use crate::game_state::GameState;
use crate::board::Board;
use crate::piece_mover::PieceMover;
use crate::pieces::Color;

#[derive(Debug)]
pub struct Game<> {
    game_state: GameState,
}

impl Game {
    pub fn new() -> Self {
        Game {
            game_state: GameState::new()
        }
    }

    pub fn board(&self) -> &Board {
        &self.game_state.board()
    }

    pub fn to_fen(&self) -> String {
        self.game_state.to_fen()
    }

    // Parse a simple coordinate move in the form "e2-e4" and delegate to the existing move_piece
    // Returns false if the input cannot be parsed or the move fails.
    pub fn move_piece_str(&mut self, mv: &str) -> bool {
        // Expected formats: "e2-e4" or "e2 e4" (accept both '-')
        let cleaned = mv.trim();
        let parts: Vec<&str> = if cleaned.contains('-') {
            cleaned.split('-').collect()
        } else if cleaned.contains(' ') {
            cleaned.split_whitespace().collect()
        } else {
            // also allow "e2e4"
            if cleaned.len() == 4 {
                vec![&cleaned[0..2], &cleaned[2..4]]
            } else {
                return false;
            }
        };

        if parts.len() != 2 { return false; }

        fn parse_sq(sq: &str) -> Option<(usize, usize)> {
            let bytes = sq.as_bytes();
            if bytes.len() != 2 { return None; }
            let file = (bytes[0] as char).to_ascii_lowercase();
            let rank = bytes[1] as char;

            if !('a'..='h').contains(&file) { return None; }
            if !('1'..='8').contains(&rank) { return None; }

            let col = (file as u8 - b'a') as usize; // a->0, h->7
            let row = (rank as u8 - b'1') as usize; // 1->0, 8->7
            Some((row, col))
        }

        let from = match parse_sq(parts[0]) { Some(v) => v, None => return false };
        let to   = match parse_sq(parts[1]) { Some(v) => v, None => return false };

        self.move_piece(from, to)
    }

    pub fn init_state_from_fen(&mut self, fen: &str) -> Result<(), String> {
        self.game_state = GameState::from_fen(fen)?;
        Ok(())
    }

    pub fn init_state(&mut self) -> Result<(), String> {
        self.game_state = GameState::new();
        Ok(())
    }

    fn move_piece(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        if PieceMover::move_piece(&mut self.game_state, from, to) {
            self.game_state.switch_color();
            if self.game_state.active_color() == Color::White {
                self.game_state.increment_full_move_number();
            }
            return true;
        }
        false
    }
}

