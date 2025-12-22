use crate::piece::pieces::{Color, Piece, PieceType};
use crate::state::game_state::GameState;

pub fn move_king(game_state: &mut GameState, piece: Piece, from: (usize, usize), to: (usize, usize), is_capture: bool, promotion_piece: Option<Piece>) -> bool {
    return true
}