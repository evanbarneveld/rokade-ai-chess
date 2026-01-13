use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};
use crate::board::evaluator::{is_piece, opponent, taper_general};

pub fn evaluate_knight(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> i32 {
    let mut val = 0;
    if phase > 0 {
        let dev_bonus = match color {
            Color::White => match (row, col) {
                (2, 2) | (2, 5) => 6,
                (1, 3) | (1, 4) => 4,
                _ => 0,
            },
            Color::Black => match (row, col) {
                (5, 2) | (5, 5) => 6,
                (6, 3) | (6, 4) => 4,
                _ => 0,
            },
        };
        val += (dev_bonus * phase) / 24;
    }
    if is_knight_outpost(board, row, col, color) {
        val += taper_general(phase, 22, 8);
    }
    val
}

pub fn is_knight_outpost(board: &Board, row: usize, col: usize, color: Color) -> bool {
    let (min_r, max_r) = match color { Color::White => (3, 6), Color::Black => (1, 4) };
    if row < min_r || row > max_r { return false; }
    let mut protected = false;
    let br_opt = match color { Color::White => row.checked_sub(1), Color::Black => if row < 7 { Some(row+1) } else { None } };
    if let Some(br) = br_opt {
        for dc in [-1, 1] {
            let nc = col as i32 + dc;
            if nc >= 0 && nc <= 7 && is_piece(board, br, nc as usize, color, PieceType::Pawn) { protected = true; break; }
        }
    }
    if !protected { return false; }
    let enemy = opponent(color);
    if col > 0 {
        match enemy {
            Color::White => { for r in 0..row { if is_piece(board, r, col-1, enemy, PieceType::Pawn) { return false; } } },
            Color::Black => { for r in (row+1)..8 { if is_piece(board, r, col-1, enemy, PieceType::Pawn) { return false; } } }
        }
    }
    if col < 7 {
        match enemy {
            Color::White => { for r in 0..row { if is_piece(board, r, col+1, enemy, PieceType::Pawn) { return false; } } },
            Color::Black => { for r in (row+1)..8 { if is_piece(board, r, col+1, enemy, PieceType::Pawn) { return false; } } }
        }
    }
    true
}

pub fn count_knight_targets(board: &Board, r: usize, c: usize, color: Color) -> usize {
    let mut targets = 0;
    let jumps = [(-2,-1),(-2,1),(-1,-2),(-1,2),(1,-2),(1,2),(2,-1),(2,1)];
    for (dr, dc) in jumps {
        let nr = r as i32 + dr;
        let nc = c as i32 + dc;
        if nr >= 0 && nr <= 7 && nc >= 0 && nc <= 7 {
            if let Some(p) = board.get(nr as usize, nc as usize) {
                if p.get_color() != color { targets += 1; }
            } else {
                targets += 1;
            }
        }
    }
    targets
}
