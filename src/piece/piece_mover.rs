use crate::state::game_state::GameState;
use crate::piece::pieces::{Color, Piece, PieceType};

#[derive(Debug)]
pub struct PieceMover {}

impl PieceMover {
    pub fn move_piece(game_state: &mut GameState, from: (usize, usize), to: (usize, usize), is_capture: bool, promotion_piece: Option<Piece>) -> bool {
        let piece: Piece = match game_state.board().get(from.0, from.1) {
            Some(p) => p,
            None => return false,
        };

        if piece.get_color() != game_state.active_color() {
            return false;
        }

        if !game_state.is_valid_board_move(from, to, game_state.active_color(), is_capture) { return false;}

        match piece.get_type() {
            PieceType::Pawn => {
                if Self::is_valid_pawn_move(game_state, from, to, is_capture, game_state.active_color(), promotion_piece) {

                    if is_capture && game_state.en_passant_target().is_some() {
                        if game_state.en_passant_target().unwrap() == to {
                            if (piece.get_color() == Color::White) {
                                game_state.clear_square(to.0-1, to.1);
                            } else {
                                game_state.clear_square(to.0+1, to.1);
                            }
                        }
                    }

                    let mut promotion_piece_to_use = promotion_piece;

                    if promotion_piece.is_none() {
                        if game_state.active_color() == Color::White && to.0 == 7 ||
                            game_state.active_color() == Color::Black && to.0 == 0 {
                            promotion_piece_to_use = Some(Piece::new(PieceType::Queen, game_state.active_color()));
                        }
                    }
                    let move_ok = game_state.move_pawn(from, to, is_capture, promotion_piece_to_use);

                    if !move_ok { return false; }

                    if from.0 == 1 && to.0 == 3 && from.1 == to.1 {
                        game_state.set_en_passant_target(Option::from((2, from.1)));
                    } else if from.0 == 6 && to.0 == 4 && from.1 == to.1 {
                        game_state.set_en_passant_target(Option::from((5, from.1)));
                    } else {
                        game_state.set_en_passant_target(Option::None);
                    }

                    game_state.reset_half_move_clock();

                    return true
                }
                false
            }
            PieceType::Knight => {
                game_state.increment_half_move_clock(); //only if valid move
                return game_state.move_piece(from, to, is_capture);
            }
            PieceType::Bishop => {
                game_state.increment_half_move_clock(); //only if valid move
                return game_state.move_piece(from, to, is_capture);
            }
            PieceType::Rook => {
                game_state.increment_half_move_clock(); //only if valid move
                return game_state.move_piece(from, to, is_capture);
            }
            PieceType::Queen => {
                game_state.increment_half_move_clock(); //only if valid move
                return game_state.move_piece(from, to, is_capture);
            }
            PieceType::King => {
                if Self::is_valid_castling_move(game_state, from, to) {
                    game_state.increment_half_move_clock();
                    //adjust_game_state(from, to); //for casting move
                    game_state.move_piece(from, to, is_capture);
                    //self.game_state.move_piece(from, to); //TODO hop the rook
                    return true
                } else {
                    if Self::is_valid_king_move(game_state, from, to) {
                       game_state.increment_half_move_clock();
                       return game_state.move_piece(from, to, is_capture);
                    }
                    true
                }
            }
        }
    }

    fn is_valid_pawn_move(game_state: &GameState, from: (usize, usize), to: (usize, usize), is_capture:bool, active_color:Color, promotion_piece:Option<Piece>) -> bool {

        if is_capture {
            if (from.0 as i32 - to.0 as i32).abs() != 1 { return false; }
        } else {
            if (from.1 != to.1) { return false; }
        }

        if (promotion_piece.is_some()) {
            if (active_color == Color::White) {
                if (to.0 != 7) { return false; }
            } else {
                if (to.0 != 0) { return false; }
            }
        }

        let mut targetOk = if is_capture {
            game_state.board_square_has_piece_of_opposite_color(to, game_state.active_color())
        }  else {
            game_state.board_square_is_empty(to)
        };

        if !targetOk && is_capture {
            //the pawn move can be a valid en passant capture move
            let ep_target = game_state.en_passant_target();
            if ep_target.is_some() {
                targetOk = game_state.board_square_is_empty(ep_target.unwrap());
                return targetOk;
            }
        }
        targetOk
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
