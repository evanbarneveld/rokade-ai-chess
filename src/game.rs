use crate::game_state::GameState;
use crate::board::Board;

#[derive(Debug)]
pub struct Game {
    game_state: GameState
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

    fn move_piece(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        if self.is_valid_pawn_move(from, to) {
            self.adjust_game_state(from, to); //for pawn move
            self.game_state.move_piece(from, to);
            //handle promotion
        } else if self.is_valid_castling_move(from, to) {
            self.adjust_game_state(from, to); //for casting move
            self.game_state.move_piece(from, to);
            //self.game_state.move_piece(from, to); //TODO hop the rook
            return true
        } else if self.is_valid_move(from, to) { //for regular move
            self.adjust_game_state(from, to);
            self.game_state.move_piece(from, to);
            return true
        }
        false
    }

    fn is_valid_pawn_move(&self, from: (usize, usize), to: (usize, usize)) -> bool {
        false
    }

    fn is_valid_castling_move(&self, from: (usize, usize), to: (usize, usize)) -> bool {
        //check if the 'from' location is occupied with a piece and the right color
        //determine if the position is in check
        //check if the movement of the piece is correct
        false
    }

    fn is_valid_move(&self, from: (usize, usize), to: (usize, usize)) -> bool {
        //check if the 'from' location is occupied with a piece and the right color
        //determine if the position is in check
        //check if the movement of the piece is correct
        true
    }

    fn adjust_game_state(&self, from: (usize, usize), to: (usize, usize)) {
        //adjust the game_state:
        // active_color
        // castling_rights
        // en_passant_target
        // half_move_clock
        // full_move_number
    }
}