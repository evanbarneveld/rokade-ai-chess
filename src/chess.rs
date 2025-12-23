use crate::state::game_state::GameState;
use crate::board::Board;
use crate::parser::parser::MoveParser;
use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{Color };
use crate::history::history::History;
use crate::state::fen::reader::reset_from_fen;
use crate::state::fen::writer::game_state_to_fen_string;

#[derive(Debug)]
pub struct Chess<> {
    game_state: GameState,
    move_parser: MoveParser,
    starting_fen: String,
    history: History
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
            history: History::new()
        }
    }

    pub fn get_history(&self) -> &History {
        &self.history
    }
    
    pub fn reset(&mut self) -> Result<(), String> {
        self.history.reset();
        match reset_from_fen(&self.starting_fen) {
            Ok(mut state) => {
                state.recompute_outcome();
                Ok(self.game_state = state)
            },
            Err(e) => Err(e)
        }
    }

    pub fn set_starting_fen(&mut self, fen: &str) -> Result<(), String> {
        self.starting_fen = String::from(fen);
        self.game_state = reset_from_fen(&self.starting_fen)?;
        self.game_state.recompute_outcome();
        Ok(())
    }

    pub fn reset_board_to_fen(&mut self, fen: &str) -> Result<(), String> {
        self.game_state = reset_from_fen(fen)?;
        self.game_state.recompute_outcome();
        Ok(())
    }

    pub fn board(&mut self) -> &Board {
        &self.game_state.board()
    }

    pub fn to_fen(&self) -> String {
        game_state_to_fen_string(self.game_state.clone())
    }

    pub fn active_color_is_white(&self) -> bool {
        self.game_state.active_color() == Color::White
    }

    pub fn list(&self) {
        println!("{}", self.history.show_history());
    }

    pub fn undo_move(&mut self) -> Option<String> {
        // Pop the last move from history
        let undone = self.history.undo_move();
        if undone.is_none() {
            return None;
        }

        // Reset the game to the starting position
        let removed_move = undone.unwrap();

        if self.history.len() > 0 {
            let last_index = self.history.len() - 1;
            let last_move = self.history.get_move(last_index);
            let last_fen = last_move.map(|mv| mv.1.clone());
            self.reset_board_to_fen(last_fen.unwrap().as_str()).unwrap();
        } else {
            self.reset().unwrap();
        }
        Some(removed_move.0)
    }

    pub fn move_piece_san(&mut self, mv: &str) -> bool {
        let active_color = self.game_state.active_color();
        let en_passant_target = self.game_state.en_passant_target();
        let mut_game_state = &mut self.game_state;
        let mutable_board = mut_game_state.mutable_board();
        let parsed_move = self.move_parser.parse(mutable_board, active_color, mv, en_passant_target);
        match parsed_move {
            Ok(v) => {
                if PieceMover::move_piece(&mut self.game_state, v.from, v.to, v.is_capture, v.promotion_piece) {
                    self.game_state.switch_player_turn();
                    self.history.add_move(mv.to_string(), game_state_to_fen_string(self.game_state.clone()));
                    true
                } else {
                    false
                }
            },
            Err(e) => { println!("Error parsing move: {}", e); false}
        }
    }

    pub fn get_game_state(&mut self) -> &mut GameState {
        &mut self.game_state
    }
}

