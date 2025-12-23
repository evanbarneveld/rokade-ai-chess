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
                        // Pawns attack forward diagonals relative to their color
                        if opponent == Color::White {
                            if r + 1 == square.0 && (c as i32 - square.1 as i32).abs() == 1 {
                                return true;
                            }
                        } else {
                            if r == 0 {
                                continue;
                            }
                            if r - 1 == square.0 && (c as i32 - square.1 as i32).abs() == 1 {
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
                        let dr = if r > square.0 { r - square.0 } else { square.0 - r };
                        let dc = if c > square.1 { c - square.1 } else { square.1 - c };
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