use crate::board::Board;
use crate::board::evaluation_helpers::taper_general;
use crate::piece::pieces::{Color, PieceType};

const BISHOP_DEV_BONUS: i32 = 6;
const BISHOP_MOBILITY_BASE: i32 = 4;
const BISHOP_MOBILITY_MG: i32 = 2;
const BISHOP_MOBILITY_EG: i32 = 3;

pub fn evaluate_bishop(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> i32 {
    let mut val = 0;
    if phase > 0 {
        let home = match color {
            Color::White => row == 0 && (col == 2 || col == 5),
            Color::Black => row == 7 && (col == 2 || col == 5),
        };
        let targets = count_bishop_targets(board, row, col, color);
        if !home && targets >= 2 {
            val += (BISHOP_DEV_BONUS * phase) / 24;
        }
        if !home {
            let mobility = (targets as i32 - BISHOP_MOBILITY_BASE).max(0);
            if mobility > 0 {
                val += taper_general(mobility * BISHOP_MOBILITY_MG, mobility * BISHOP_MOBILITY_EG, phase);
            }
        }
        let same_color_pawns = count_same_color_pawns(board, color, (row + col) % 2 == 0);
        if same_color_pawns >= 4 {
            let mut penalty = (same_color_pawns - 3) * 4;
            if targets <= 4 {
                penalty += 4;
            }
            val -= (penalty * phase) / 24;
        }
    }
    val
}

#[inline]
fn count_bishop_targets(board: &Board, r: usize, c: usize, color: Color) -> usize {
    let mut n = 0usize;
    for (dr, dc) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
        let mut nr = r as i32 + dr;
        let mut nc = c as i32 + dc;
        while (0..8).contains(&nr) && (0..8).contains(&nc) {
            if let Some(tp) = board.get(nr as usize, nc as usize) {
                if tp.get_color() != color && tp.get_type() != PieceType::King {
                    n += 1;
                }
                break;
            } else {
                n += 1;
            }
            nr += dr;
            nc += dc;
        }
    }
    n
}

fn count_same_color_pawns(board: &Board, color: Color, dark_square: bool) -> i32 {
    let mut count = 0;
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c)
                && p.get_color() == color
                && p.get_type() == PieceType::Pawn
                && ((r + c) % 2 == 0) == dark_square
            {
                count += 1;
            }
        }
    }
    count
}
