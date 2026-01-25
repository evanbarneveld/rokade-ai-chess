use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};
use crate::board::evaluation_helpers::{count_knight_mobility, is_piece, opponent, taper_general};

// Knight mobility constants
const KNIGHT_MOBILITY_BASELINE: i32 = 4; // Average knight has ~4 safe squares
const KNIGHT_MOBILITY_MG: i32 = 4;       // Centipawns per safe square above baseline
const KNIGHT_MOBILITY_EG: i32 = 3;       // Slightly less important in endgame
const KNIGHT_UNSAFE_PENALTY: i32 = 2;    // Penalty per square only reachable unsafely

pub fn evaluate_knight(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> i32 {
    let mut val = 0;

    // Mobility evaluation
    let (total, safe) = count_knight_mobility(board, row, col, color);
    let mobility_delta = safe - KNIGHT_MOBILITY_BASELINE;
    val += taper_general(mobility_delta * KNIGHT_MOBILITY_MG, mobility_delta * KNIGHT_MOBILITY_EG, phase);

    // Penalty for squares that are only reachable unsafely (attacked by enemy pawns)
    let unsafe_squares = total - safe;
    if unsafe_squares > 0 {
        val -= (unsafe_squares * KNIGHT_UNSAFE_PENALTY * phase) / 24;
    }

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
    if col == 0 || col == 7 {
        val -= taper_general(14, 6, phase);
    }
    if is_knight_outpost(board, row, col, color) {
        val += taper_general(22, 8, phase);
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
            if (0..=7).contains(&nc) && is_piece(board, br, nc as usize, color, PieceType::Pawn) { protected = true; break; }
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

