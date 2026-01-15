use crate::state::game_state::GameState;
use crate::board::Board;
use crate::parser::parser::MoveParser;
use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{Color, Piece, PieceType};
use crate::history::history::History;
use crate::piece::as_square_str;
use crate::state::fen::reader::reset_from_fen;
use crate::state::fen::writer::game_state_to_fen_string;
use crate::search::SearchMode;
use crate::search::state::zobrist::compute_zobrist_full;

#[derive(Debug)]
pub struct Chess<> {
    game_state: GameState,
    move_parser: MoveParser,
    starting_fen: String,
    history: History,
    search_mode: SearchMode,
    playing_strength: usize,
}
impl Default for Chess {
    fn default() -> Self {
        Self::new()
    }
}

impl Chess {
    pub const DEFAULT_CHESS_STARTING_FEN: &'static str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    pub fn new() -> Self {
        let mut chess = Chess {
            starting_fen: String::from(Chess::DEFAULT_CHESS_STARTING_FEN),
            game_state:
                match reset_from_fen(Chess::DEFAULT_CHESS_STARTING_FEN) {
                    Ok(state) => state,
                    Err(e) => panic!("Error parsing FEN: {}", e)
                },
            move_parser: MoveParser::new(),
            history: History::new(),
            search_mode: SearchMode::Normal,
            playing_strength: 1000,
        };
        let zobrist = compute_zobrist_full(
            chess.game_state.board(),
            chess.game_state.active_color(),
            &chess.game_state.castling_rights(),
            chess.game_state.en_passant_target(),
        );
        chess.history.set_starting_position(Chess::DEFAULT_CHESS_STARTING_FEN.to_string(), zobrist);
        chess
    }

    pub fn get_history(&mut self) -> &History {
        &self.history
    }

    pub fn set_search_mode(&mut self, mode: SearchMode) {
        self.search_mode = mode;
    }

    pub fn get_search_mode(&self) -> SearchMode {
        self.search_mode
    }

    pub fn set_playing_strength(&mut self, strength: usize) {
        // clamp to [1..1000]
        let s = if strength == 0 { 1 } else { strength.min(1000) };
        self.playing_strength = s;
    }

    pub fn get_playing_strength(&self) -> usize {
        self.playing_strength
    }

    pub fn reset(&mut self) -> Result<(), String> {
        self.history.reset();
        let mut state = reset_from_fen(&self.starting_fen)?;
        state.recompute_outcome(&self.history);
        self.game_state = state;
        Ok(())
    }

    pub fn set_starting_fen(&mut self, fen: &str) -> Result<(), String> {
        self.starting_fen = String::from(fen);
        self.game_state = reset_from_fen(&self.starting_fen)?;
        self.history.reset();
        let zobrist = compute_zobrist_full(
            self.game_state.board(),
            self.game_state.active_color(),
            &self.game_state.castling_rights(),
            self.game_state.en_passant_target(),
        );
        self.history.set_starting_position(fen.to_string(), zobrist);
        self.game_state.recompute_outcome(&self.history);
        Ok(())
    }

    pub fn reset_board_to_fen(&mut self, fen: &str) -> Result<(), String> {
        match reset_from_fen(fen) {
            Ok(state) => {
                self.game_state = state;
                self.game_state.recompute_outcome(&self.history);
                Ok(())
            }
            Err(e) => {
                println!("{}", e);
                Err(String::from("Error parsing FEN"))
            }
        }
    }

    pub fn board(&mut self) -> &Board {
        self.game_state.board()
    }

    pub fn to_fen(&self) -> String {
        game_state_to_fen_string(self.game_state)
    }

    pub fn active_color_is_white(&self) -> bool {
        self.game_state.active_color() == Color::White
    }

    pub fn list(&self) {
        println!("{}", self.history.show_history());
    }

    pub fn undo_move(&mut self) -> Option<String> {
        // Pop the last move from history
        let removed_move = self.history.undo_move()?;

        if self.history.len() > 0 {
            let last_index = self.history.len() - 1;
            let last_move = self.history.get_move(last_index);
            let last_fen = last_move.map(|mv| mv.2.clone())?;
            self.reset_board_to_fen(&last_fen).unwrap();
        } else {
            self.reset().unwrap();
        }
        Some(removed_move.0)
    }
    
    pub fn move_piece(&mut self, from:(usize, usize), to:(usize,usize), promotion_char:Option<char> ) -> bool {
        let mut is_capture = false;
        let mutable_board = self.get_game_state().mutable_board();
        let piece_to_move = mutable_board.get(from.0, from.1);
        if piece_to_move.is_none() { return false; }
        if mutable_board.get(to.0, to.1).is_some() { is_capture = true; }

        let mut promotion_piece = None;

        if piece_to_move.unwrap().get_type() == PieceType::Pawn && (to.0 == 7 || to.0 == 0) {
            let active_color = piece_to_move.unwrap().get_color();
            if active_color == Color::White {
                // get piece type from promotion_char
                let adjusted_promotion_char = promotion_char.unwrap().to_ascii_uppercase();
                promotion_piece = Piece::from_fen_char(adjusted_promotion_char);
            } else {
                let adjusted_promotion_char = promotion_char.unwrap().to_ascii_lowercase();
                promotion_piece = Piece::from_fen_char(adjusted_promotion_char);
            }
        }

        if PieceMover::move_piece(&mut self.game_state, from, to, is_capture, promotion_piece) {
            self.game_state.switch_player_turn();
            let from_move_string = as_square_str(from);
            let from_to_string = as_square_str(to);
            let mut mv = format!("{}{}", from_move_string, from_to_string);
            if is_capture {
                mv = format!("{}x{}", mv, as_square_str(to));
            }

            let zobrist = compute_zobrist_full(
                self.game_state.board(),
                self.game_state.active_color(),
                &self.game_state.castling_rights(),
                self.game_state.en_passant_target(),
            );
            self.history.add_move(mv.to_string(), (from, to), game_state_to_fen_string(self.game_state), zobrist);
            true
        } else {
            false
        }
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
                    let zobrist = compute_zobrist_full(
                        self.game_state.board(),
                        self.game_state.active_color(),
                        &self.game_state.castling_rights(),
                        self.game_state.en_passant_target(),
                    );
                    self.history.add_move(mv.to_string(), (v.from, v.to), game_state_to_fen_string(self.game_state), zobrist);
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

