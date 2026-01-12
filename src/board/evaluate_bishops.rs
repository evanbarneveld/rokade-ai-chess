use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};

pub fn count_bishops(board: &Board) -> (i32, i32) {
    let mut w = 0;
    let mut b = 0;
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c) {
                if p.get_type() == PieceType::Bishop {
                    match p.get_color() {
                        Color::White => w += 1,
                        Color::Black => b += 1,
                    }
                }
            }
        }
    }
    (w, b)
}
