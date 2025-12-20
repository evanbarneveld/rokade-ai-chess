use crate::state::game_state::GameState;
use crate::board::Board;
use crate::cli::parser::MoveParser;
use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::Color;
use crate::state::fen::reader::reset_from_fen;
use crate::state::fen::writer::game_state_to_fen_string;

#[derive(Debug)]
pub struct Chess<> {
    game_state: GameState,
    move_parser: MoveParser,
    starting_fen: String,
}

impl Chess {
    pub const DEFAULT_CHESS_STARTING_FEN: &'static str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    pub fn new() -> Self {

        Chess {
            starting_fen: String::from(Chess::DEFAULT_CHESS_STARTING_FEN),
            game_state:
                match reset_from_fen(Chess::DEFAULT_CHESS_STARTING_FEN) {
                    Ok(state) => state,
                    Err(e) => panic!("Error parsing FEN: {}", e)
                },
            move_parser: MoveParser::new(),
        }
    }

    pub fn reset(&mut self) -> Result<(), String> {
        self.game_state = reset_from_fen(&self.starting_fen)?;
        Ok(())
    }
    pub fn set_starting_fen(&mut self, fen: &str) -> Result<(), String> {
        self.starting_fen = String::from(fen);
        self.game_state = reset_from_fen(&self.starting_fen)?;
        Ok(())
    }

    pub fn board(&self) -> &Board {
        &self.game_state.board()
    }

    pub fn to_fen(&self) -> String {
        game_state_to_fen_string(self.game_state.clone())
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

