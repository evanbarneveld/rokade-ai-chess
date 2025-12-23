use crate::state::game_state::GameState;
use crate::piece::pieces::{ Piece, PieceType};
use crate::piece::move_validators::pawn_move_validator::is_valid_pawn_move;
use crate::piece::move_validators::knight_move_validator::is_valid_knight_move;
use crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move;
use crate::piece::move_validators::queen_move_validator::is_valid_queen_move;
use crate::piece::move_validators::king_move_validator::is_valid_king_move;
use crate::piece::move_validators::rook_move_validator::is_valid_rook_move;

use crate::piece::piece_movers::move_pawn::move_pawn;
use crate::piece::piece_movers::move_knight::move_knight;
use crate::piece::piece_movers::move_bishop::move_bishop;
use crate::piece::piece_movers::move_rook::move_rook;
use crate::piece::piece_movers::move_queen::move_queen;
use crate::piece::piece_movers::move_king::move_king;

#[derive(Debug)]
pub struct PieceMover {}

impl PieceMover {
    pub fn move_piece(game_state: &mut GameState, from: (usize, usize), to: (usize, usize), is_capture: bool, promotion_piece: Option<Piece>) -> bool {

        // be sure there is a piece at the 'from' location, to move
        let piece: Piece = match game_state.board().get(from.0, from.1) {
            Some(p) => p,
            None => return false,
        };

        let active_color = game_state.active_color();

        if !game_state.move_from_and_to_validation_check(from, to, active_color, is_capture, piece.get_type() == PieceType::Pawn, game_state.en_passant_target()) {
            return false;
        }

        match piece.get_type() {
            PieceType::Pawn => {
                let en_passant_target : Option<(usize,usize)> = game_state.en_passant_target();
                if is_valid_pawn_move(game_state.mutable_board(), from, to, is_capture, en_passant_target, active_color, promotion_piece, true) {
                    return move_pawn(game_state, piece, from, to, is_capture, promotion_piece);
                }
                false
            }
            PieceType::Knight => {
                if is_valid_knight_move(game_state.mutable_board(), from, to, true) {
                    return move_knight(game_state, from, to, is_capture);
                }
                false
            }
            PieceType::Bishop => {
                if is_valid_bishop_move(game_state.mutable_board(), from, to, true) {
                    return move_bishop(game_state, from, to, is_capture);
                }
                false
            }
            PieceType::Rook => {
                if is_valid_rook_move(game_state.mutable_board(), from, to, true) {
                    return move_rook(game_state, from, to, is_capture);
                }
                false
            }
            PieceType::Queen => {
                if is_valid_queen_move(game_state.mutable_board(), from, to, true) {
                    return move_queen(game_state, from, to, is_capture);
                }
                false
            }
            PieceType::King => {
                if is_valid_king_move(game_state, from, to) {
                    return move_king(game_state, piece, from, to, is_capture);
                }
                false
            }
        }
    }
}
