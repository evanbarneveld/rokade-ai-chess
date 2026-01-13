//! Attack map building for evaluation.
//!
//! Provides pseudo-legal attack maps showing which squares
//! are attacked by White and Black pieces.

use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};

/// Build pseudo-legal attack maps for both sides.
/// Returns (white_attacks, black_attacks) as 8x8 boolean arrays.
pub fn build_attack_maps(board: &Board) -> ([[bool; 8]; 8], [[bool; 8]; 8]) {
    let mut w = [[false; 8]; 8];
    let mut b = [[false; 8]; 8];

    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c) {
                let color = p.get_color();
                match p.get_type() {
                    PieceType::Knight => add_knight_attacks(r, c, color, &mut w, &mut b),
                    PieceType::Bishop => add_slider_attacks(board, r, c, color, &BISHOP_DIRS, &mut w, &mut b),
                    PieceType::Rook => add_slider_attacks(board, r, c, color, &ROOK_DIRS, &mut w, &mut b),
                    PieceType::Queen => add_slider_attacks(board, r, c, color, &QUEEN_DIRS, &mut w, &mut b),
                    PieceType::King => add_king_attacks(r, c, color, &mut w, &mut b),
                    PieceType::Pawn => add_pawn_attacks(r, c, color, &mut w, &mut b),
                }
            }
        }
    }
    (w, b)
}

// Direction arrays for sliders
const BISHOP_DIRS: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
const ROOK_DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const QUEEN_DIRS: [(i32, i32); 8] = [
    (1, 1), (1, -1), (-1, 1), (-1, -1),
    (1, 0), (-1, 0), (0, 1), (0, -1),
];

// Knight move offsets
const KNIGHT_MOVES: [(i32, i32); 8] = [
    (2, 1), (1, 2), (-1, 2), (-2, 1),
    (-2, -1), (-1, -2), (1, -2), (2, -1),
];

#[inline]
fn add_knight_attacks(r: usize, c: usize, color: Color, w: &mut [[bool; 8]; 8], b: &mut [[bool; 8]; 8]) {
    for (dr, dc) in KNIGHT_MOVES {
        let nr = r as i32 + dr;
        let nc = c as i32 + dc;
        if nr >= 0 && nr < 8 && nc >= 0 && nc < 8 {
            match color {
                Color::White => w[nr as usize][nc as usize] = true,
                Color::Black => b[nr as usize][nc as usize] = true,
            }
        }
    }
}

#[inline]
fn add_slider_attacks(
    board: &Board,
    r: usize,
    c: usize,
    color: Color,
    dirs: &[(i32, i32)],
    w: &mut [[bool; 8]; 8],
    b: &mut [[bool; 8]; 8],
) {
    for (dr, dc) in dirs.iter() {
        let mut nr = r as i32 + dr;
        let mut nc = c as i32 + dc;
        while nr >= 0 && nr < 8 && nc >= 0 && nc < 8 {
            match color {
                Color::White => w[nr as usize][nc as usize] = true,
                Color::Black => b[nr as usize][nc as usize] = true,
            }
            if board.get(nr as usize, nc as usize).is_some() {
                break;
            }
            nr += dr;
            nc += dc;
        }
    }
}

#[inline]
fn add_king_attacks(r: usize, c: usize, color: Color, w: &mut [[bool; 8]; 8], b: &mut [[bool; 8]; 8]) {
    for dr in -1..=1 {
        for dc in -1..=1 {
            if dr == 0 && dc == 0 {
                continue;
            }
            let nr = r as i32 + dr;
            let nc = c as i32 + dc;
            if nr >= 0 && nr < 8 && nc >= 0 && nc < 8 {
                match color {
                    Color::White => w[nr as usize][nc as usize] = true,
                    Color::Black => b[nr as usize][nc as usize] = true,
                }
            }
        }
    }
}

#[inline]
fn add_pawn_attacks(r: usize, c: usize, color: Color, w: &mut [[bool; 8]; 8], b: &mut [[bool; 8]; 8]) {
    match color {
        Color::White => {
            if r < 7 {
                if c > 0 {
                    w[r + 1][c - 1] = true;
                }
                if c < 7 {
                    w[r + 1][c + 1] = true;
                }
            }
        }
        Color::Black => {
            if r > 0 {
                if c > 0 {
                    b[r - 1][c - 1] = true;
                }
                if c < 7 {
                    b[r - 1][c + 1] = true;
                }
            }
        }
    }
}
