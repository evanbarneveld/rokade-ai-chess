use crate::state::game_state::GameState;
use crate::board::Board;
use crate::parser::parser::MoveParser;
use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{Color, Piece };
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
        match reset_from_fen(&self.starting_fen) {
            Ok(state) => { Ok(self.game_state = state) },
            Err(e) => Err(e)
        }

    }

    pub fn set_starting_fen(&mut self, fen: &str) -> Result<(), String> {
        self.starting_fen = String::from(fen);
        self.game_state = reset_from_fen(&self.starting_fen)?;
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

    pub fn move_piece_str(&mut self, mv: &str) -> bool {
        let active_color = self.game_state.active_color();
        let board = self.game_state.board();
        let parsed_move = self.move_parser.parse(board, active_color, mv);
        match parsed_move {
            Ok(v) => {
                let mut ok: bool = true;

                if v.promotion_piece.is_some() {
                    if !self.promote_piece(v.from, v.to, v.promotion_piece.unwrap()) {
                        ok = false
                    }
                    ok
                } else {
                    if !self.move_piece(v.from, v.to) {
                        ok = false
                    }
                    ok
                }
            },
            Err(e) => { println!("Error parsing move: {}", e); false}
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

    fn promote_piece(&mut self, from:(usize, usize), to: (usize, usize), promotion_piece: char) -> bool {
        let piece = if self.game_state.active_color() == Color::White {
            Piece::from_fen_char(promotion_piece.to_ascii_uppercase())
        } else {
            Piece::from_fen_char(promotion_piece.to_ascii_lowercase())
        };

        if (piece.is_none()) {
            false;
        }

        PieceMover::promote_pawn(&mut self.game_state, from, to, piece.unwrap())
    }

}

