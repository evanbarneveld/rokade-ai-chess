use crate::state::game_state::GameState;
use crate::piece::pieces::{Piece, PieceType};

#[derive(Debug)]
pub struct PieceMover {}

impl PieceMover {
    pub fn move_piece(game_state: &mut GameState, from: (usize, usize), to: (usize, usize)) -> bool {
        let piece: Piece = match game_state.board().get(from.0, from.1) {
            Some(p) => p,
            None => return false,
        };

        if piece.get_color() != game_state.active_color() {
            return false;
        }

        match piece.get_type() {
            PieceType::Pawn => {
                if Self::is_valid_pawn_move(game_state, from, to) {
                    if from.0 == 1 && to.0 == 3 && from.1 == to.1 {
                        game_state.set_en_passant_target(Option::from((2, from.1)));
                    } else if from.0 == 6 && to.0 == 4 && from.1 == to.1 {
                        game_state.set_en_passant_target(Option::from((5, from.1)));
                    } else {
                        game_state.set_en_passant_target(Option::None);
                    }
                    //handle promotion
                    game_state.reset_half_move_clock();
                    return game_state.move_piece(from, to);
                }
                false
            }
            PieceType::Knight => {
                game_state.increment_half_move_clock(); //only if valid move
                return game_state.move_piece(from, to);
            }
            PieceType::Bishop => {
                game_state.increment_half_move_clock(); //only if valid move
                return game_state.move_piece(from, to);
            }
            PieceType::Rook => {
                game_state.increment_half_move_clock(); //only if valid move
                return game_state.move_piece(from, to);
            }
            PieceType::Queen => {
                game_state.increment_half_move_clock(); //only if valid move
                return game_state.move_piece(from, to);
            }
            PieceType::King => {
                if Self::is_valid_castling_move(game_state, from, to) {
                    game_state.increment_half_move_clock();
                    //adjust_game_state(from, to); //for casting move
                    game_state.move_piece(from, to);
                    //self.game_state.move_piece(from, to); //TODO hop the rook
                    return true
                } else {
                    if Self::is_valid_king_move(game_state, from, to) {
                       game_state.increment_half_move_clock();
                       return game_state.move_piece(from, to);
                    }
                    true
                }
            }
        }
    }

    fn is_valid_pawn_move(game_state: &GameState, from: (usize, usize), to: (usize, usize)) -> bool {
            true
    }

    fn is_valid_king_move(game_state: &GameState, from: (usize, usize), to: (usize, usize)) -> bool {
        //check if the 'from' location is occupied with a piece and the right color
        //determine if the position is in check
        //check if the movement of the piece is correct
        false
    }
    fn is_valid_castling_move(game_state: &GameState, from: (usize, usize), to: (usize, usize)) -> bool {
        //check if the 'from' location is occupied with a piece and the right color
        //determine if the position is in check
        //check if the movement of the piece is correct
        false
    }
}
