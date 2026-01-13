use crate::board::Board;
use crate::board::checks::king_in_check::is_king_in_check_after_move;
use crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move;
use crate::piece::move_validators::knight_move_validator::is_valid_knight_move;
use crate::piece::move_validators::pawn_move_validator::is_valid_pawn_move;
use crate::piece::move_validators::queen_move_validator::is_valid_queen_move;
use crate::piece::move_validators::rook_move_validator::is_valid_rook_move;
use crate::piece::pieces::{Color, Piece, PieceType};

pub mod pawn_move_validator;
pub mod knight_move_validator;
pub mod bishop_move_validator;
pub mod queen_move_validator;
pub mod king_move_validator;
pub mod rook_move_validator;

pub fn is_piece_move_valid(
    board: &Board,
    active_color: Color,
    r: usize,
    c: usize,
    piece: Piece,
    tr: usize,
    tc: usize,
    from: (usize, usize),
    to: (usize, usize),
    is_capture: bool,
) -> bool {
    // piece-type specific path/shape validation including pin checks
    let mut tmp = *board;
    
    match piece.get_type() {
        PieceType::Pawn => is_valid_pawn_move(
            &mut tmp,
            from,
            to,
            is_capture,
            None,
            active_color,
            None,
            true,
        ),
        PieceType::Knight => is_valid_knight_move(&mut tmp, from, to, true),
        PieceType::Bishop => is_valid_bishop_move(&mut tmp, from, to, true),
        PieceType::Rook => is_valid_rook_move(&mut tmp, from, to, true),
        PieceType::Queen => is_valid_queen_move(&mut tmp, from, to, true),
        PieceType::King => {
            // king: allow single-square moves that do not move into check (no castling here)
            let dr = r.abs_diff(tr);
            let dc = c.abs_diff(tc);
            if dr <= 1 && dc <= 1 {
                // ensure the king wouldn't be in check after the move
                !is_king_in_check_after_move(&mut tmp, from, to, None)
            } else {
                false
            }
        }
    }
}