use crate::piece::pieces::{Color, Piece, PieceType};
use crate::state::game_state::GameState;

pub fn move_pawn(game_state: &mut GameState, piece: Piece, from: (usize, usize), to: (usize, usize), is_capture: bool, promotion_piece: Option<Piece>) -> bool {
    if is_capture && game_state.en_passant_target().is_some() {
        if game_state.en_passant_target().unwrap() == to {
            if piece.get_color() == Color::White {
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
    let move_ok = game_state.move_pawn(from, to, promotion_piece_to_use);

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