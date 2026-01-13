use crate::board::Board;
use crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move;
use crate::piece::move_validators::knight_move_validator::is_valid_knight_move;
use crate::piece::move_validators::rook_move_validator::is_valid_rook_move;
use crate::piece::move_validators::queen_move_validator::is_valid_queen_move;
use crate::piece::pieces::{Color, PieceType};

pub fn is_square_attacked_by_opponent(board: &mut Board, square: (usize, usize), active_color: Color) -> bool {
    let opponent = match active_color { Color::White => Color::Black, Color::Black => Color::White };

    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c) {
                if p.get_color() != opponent {
                    continue;
                }
                match p.get_type() {
                    PieceType::Pawn => {
                        // Pawns attack one rank forward relative to their own color.
                        // With our indexing (row 0 = rank 1, row 7 = rank 8):
                        // - White pawns move/attack towards increasing row indices (r + 1)
                        // - Black pawns move/attack towards decreasing row indices (r - 1)
                        if opponent == Color::White {
                            // White pawn attacks (r+1, c±1)
                            if r + 1 < 8 && r + 1 == square.0 && (c as i32 - square.1 as i32).abs() == 1 {
                                return true;
                            }
                        } else {
                            // Black pawn attacks (r-1, c±1)
                            if r > 0 && r - 1 == square.0 && (c as i32 - square.1 as i32).abs() == 1 {
                                return true;
                            }
                        }
                    }
                    PieceType::Knight => {
                        if is_valid_knight_move(board, (r, c), square, false) {
                            return true;
                        }
                    }
                    PieceType::Bishop => {
                        if is_valid_bishop_move(board, (r, c), square, false) {
                            return true;
                        }
                    }
                    PieceType::Rook => {
                        if is_valid_rook_move(board, (r, c), square, false) {
                            return true;
                        }
                    }
                    PieceType::Queen => {
                        if is_valid_queen_move(board, (r, c), square, false) {
                            return true;
                        }
                    }
                    PieceType::King => {
                        let dr = r.abs_diff(square.0);
                        let dc = c.abs_diff(square.1);
                        if dr <= 1 && dc <= 1 && !(dr == 0 && dc == 0) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}