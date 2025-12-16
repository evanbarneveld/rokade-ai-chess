use crate::game_state::GameState;
use crate::board::Board;
use crate::move_parser::MoveParser;
use crate::piece_mover::PieceMover;
use crate::pieces::Color;

#[derive(Debug)]
pub struct Game<> {
    game_state: GameState,
    move_parser: MoveParser
}

impl Game {
    pub fn new() -> Self {
        Game {
            game_state: GameState::new(),
            move_parser: MoveParser::new()
        }
    }

    pub fn board(&self) -> &Board {
        &self.game_state.board()
    }

    pub fn to_fen(&self) -> String {
        self.game_state.to_fen()
    }

    pub fn active_color(&self) -> Color {
        self.game_state.active_color()
    }

    pub fn move_piece_str(&mut self, mv: &str) -> bool {
        let parsed_move = self.move_parser.parse(mv);
        match parsed_move {
            Some(mv) => {self.move_piece(mv.from, mv.to) }
            None => { return false }
        }
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

