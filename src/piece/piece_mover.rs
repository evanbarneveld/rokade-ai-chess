use crate::state::game_state::GameState;
use crate::piece::pieces::{ Piece, PieceType};
use crate::piece::move_validators::pawn_move_validator::is_valid_pawn_move;
use crate::piece::piece_movers::pawn::move_pawn;

#[derive(Debug)]
pub struct PieceMover {}

impl PieceMover {
    pub fn move_piece(game_state: &mut GameState, from: (usize, usize), to: (usize, usize), is_capture: bool, promotion_piece: Option<Piece>) -> bool {

        // be sure there is a piece at the 'from' location, to move
        let piece: Piece = match game_state.board().get(from.0, from.1) {
            Some(p) => p,
            None => return false,
        };

        if !game_state.move_from_and_to_validation_check(from, to, game_state.active_color(), is_capture, piece.get_type() == PieceType::Pawn, game_state.en_passant_target()) {
            return false;
        }

        match piece.get_type() {
            PieceType::Pawn => {
                if is_valid_pawn_move(game_state, from, to, is_capture, game_state.active_color(), promotion_piece) {
                    return move_pawn(game_state, piece, from, to, is_capture, promotion_piece);
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

    fn is_valid_king_move(game_state: &GameState, from: (usize, usize), to: (usize, usize)) -> bool {
        if game_state.board_square_has_piece_of_opposite_color(to, game_state.active_color()) { return false; }

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
