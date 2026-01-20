use crate::piece::pieces::{Color};

pub fn evaluate_bishop(row: usize, col: usize, color: Color, phase: i32) -> i32 {
    let mut val = 0;
    if phase > 0 {
        let home = match color {
            Color::White => row == 0 && (col == 2 || col == 5),
            Color::Black => row == 7 && (col == 2 || col == 5),
        };
        if !home { val += (8 * phase) / 24; }
    }
    val
}