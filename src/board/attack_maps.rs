//! Attack map building for evaluation.
//!
//! Provides pseudo-legal attack maps showing which squares
//! are attacked by White and Black pieces.

use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};

/// Mobility counts collected during attack map building.
#[derive(Default, Clone, Copy)]
pub struct MobilityCounts {
    pub white_knight: i32,
    pub white_bishop: i32,
    pub white_rook: i32,
    pub white_queen: i32,
    pub black_knight: i32,
    pub black_bishop: i32,
    pub black_rook: i32,
    pub black_queen: i32,
}

/// Build pseudo-legal attack maps for both sides.
/// Returns (white_attacks, black_attacks) as 8x8 boolean arrays.
pub fn build_attack_maps(board: &Board) -> ([[bool; 8]; 8], [[bool; 8]; 8]) {
    let (w, b, _) = build_attack_maps_with_mobility(board);
    (w, b)
}

/// Build attack maps and collect mobility counts in a single pass.
/// Returns (white_attacks, black_attacks, mobility_counts).
pub fn build_attack_maps_with_mobility(board: &Board) -> ([[bool; 8]; 8], [[bool; 8]; 8], MobilityCounts) {
    let mut w = [[false; 8]; 8];
    let mut b = [[false; 8]; 8];
    let mut mob = MobilityCounts::default();

    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c) {
                let color = p.get_color();
                match p.get_type() {
                    PieceType::Knight => {
                        let count = add_knight_attacks(r, c, color, &mut w, &mut b);
                        match color {
                            Color::White => mob.white_knight += count,
                            Color::Black => mob.black_knight += count,
                        }
                    }
                    PieceType::Bishop => {
                        let count = add_slider_attacks(board, r, c, color, &BISHOP_DIRS, &mut w, &mut b);
                        match color {
                            Color::White => mob.white_bishop += count,
                            Color::Black => mob.black_bishop += count,
                        }
                    }
                    PieceType::Rook => {
                        let count = add_slider_attacks(board, r, c, color, &ROOK_DIRS, &mut w, &mut b);
                        match color {
                            Color::White => mob.white_rook += count,
                            Color::Black => mob.black_rook += count,
                        }
                    }
                    PieceType::Queen => {
                        let count = add_slider_attacks(board, r, c, color, &QUEEN_DIRS, &mut w, &mut b);
                        match color {
                            Color::White => mob.white_queen += count,
                            Color::Black => mob.black_queen += count,
                        }
                    }
                    PieceType::King => { add_king_attacks(r, c, color, &mut w, &mut b); }
                    PieceType::Pawn => { add_pawn_attacks(r, c, color, &mut w, &mut b); }
                }
            }
        }
    }
    (w, b, mob)
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
fn add_knight_attacks(r: usize, c: usize, color: Color, w: &mut [[bool; 8]; 8], b: &mut [[bool; 8]; 8]) -> i32 {
    let mut count = 0;
    for (dr, dc) in KNIGHT_MOVES {
        let nr = r as i32 + dr;
        let nc = c as i32 + dc;
        if (0..8).contains(&nr) && (0..8).contains(&nc) {
            count += 1;
            match color {
                Color::White => w[nr as usize][nc as usize] = true,
                Color::Black => b[nr as usize][nc as usize] = true,
            }
        }
    }
    count
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
) -> i32 {
    let mut count = 0;
    for (dr, dc) in dirs.iter() {
        let mut nr = r as i32 + dr;
        let mut nc = c as i32 + dc;
        while (0..8).contains(&nr) && (0..8).contains(&nc) {
            count += 1;
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
    count
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
            if (0..8).contains(&nr) && (0..8).contains(&nc) {
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
