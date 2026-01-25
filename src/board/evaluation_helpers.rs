use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};

pub(crate) use crate::board::pst::{tapered_eval as taper_general};

// Material scores (centipawns)
const PAWN: i32 = 100;
const KNIGHT: i32 = 320;
const BISHOP: i32 = 330;
const ROOK: i32 = 500;
const QUEEN: i32 = 900;
const KING: i32 = 0; // King material is not counted; PST handles its safety/activity

pub struct PawnFileCounts {
    pub white: [i32; 8],
    pub black: [i32; 8],
}

pub struct FileClearance {
    /// For each file, stores ranges that are clear of pieces
    /// Used for rook evaluation optimization
    pub files: [Vec<(usize, usize)>; 8],
}

impl FileClearance {
    pub fn new(board: &Board) -> Self {
        let mut files = [const { Vec::new() }; 8];

        for col in 0..8 {
            let mut clear_start = 0;
            for row in 0..8 {
                if board.get(row, col).is_some() {
                    if row > clear_start {
                        files[col].push((clear_start, row));
                    }
                    clear_start = row + 1;
                }
            }
            if clear_start < 8 {
                files[col].push((clear_start, 8));
            }
        }

        Self { files }
    }

    #[inline]
    pub fn is_clear_between(&self, r1: usize, r2: usize, file: usize) -> bool {
        let start = r1.min(r2) + 1;
        let end = r1.max(r2);

        if start >= end {
            return true;
        }

        for &(clear_start, clear_end) in &self.files[file] {
            if clear_start <= start && clear_end >= end {
                return true;
            }
        }
        false
    }
}

#[inline]
pub(crate) fn is_piece(board: &Board, r: usize, c: usize, color: Color, pt: PieceType) -> bool {
    matches!(board.get(r, c), Some(p) if p.get_color() == color && p.get_type() == pt)
}

#[inline]
pub(crate) fn is_color(board: &Board, r: usize, c: usize, color: Color) -> bool {
    matches!(board.get(r, c), Some(p) if p.get_color() == color)
}

#[inline]
pub(crate) fn get_piece_type(board: &Board, r: usize, c: usize) -> Option<PieceType> {
    board.get(r, c).map(|p| p.get_type())
}

#[inline]
pub(crate) fn material_value(piece: PieceType) -> i32 {
    match piece {
        PieceType::Pawn => PAWN,
        PieceType::Knight => KNIGHT,
        PieceType::Bishop => BISHOP,
        PieceType::Rook => ROOK,
        PieceType::Queen => QUEEN,
        PieceType::King => KING,
    }
}

#[inline]
pub(crate) fn square_attacked_by_enemy_pawn(board: &Board, r: usize, c: usize, enemy: Color) -> bool {
    match enemy {
        Color::White => {
            if r > 0 {
                if c > 0 && is_piece(board, r - 1, c - 1, Color::White, PieceType::Pawn) { return true; }
                if c < 7 && is_piece(board, r - 1, c + 1, Color::White, PieceType::Pawn) { return true; }
            }
        }
        Color::Black => {
            if r < 7 {
                if c > 0 && is_piece(board, r + 1, c - 1, Color::Black, PieceType::Pawn) { return true; }
                if c < 7 && is_piece(board, r + 1, c + 1, Color::Black, PieceType::Pawn) { return true; }
            }
        }
    }
    false
}

#[inline]
pub(crate) fn find_king(board: &Board, color: Color) -> Option<(usize, usize)> {
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c)
                && p.get_color() == color
                && p.get_type() == PieceType::King
            {
                return Some((r, c));
            }
        }
    }
    None
}

#[inline]
pub(crate) fn opponent(color: Color) -> Color {
    match color {
        Color::White => Color::Black,
        Color::Black => Color::White,
    }
}

#[inline]
pub(crate) fn chebyshev_dist(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

/// Apply score from White's perspective (+) or Black's perspective (-)
#[inline]
pub(crate) fn apply_color_score(score: i32, color: Color) -> i32 {
    match color {
        Color::White => score,
        Color::Black => -score,
    }
}

#[inline]
pub(crate) fn count_knight_targets(board: &Board, r: usize, c: usize, color: Color) -> usize {
    const K: [(i32, i32); 8] = [(2, 1), (1, 2), (-1, 2), (-2, 1), (-2, -1), (-1, -2), (1, -2), (2, -1)];
    let mut n = 0usize;
    for (dr, dc) in K {
        let nr = r as i32 + dr;
        let nc = c as i32 + dc;
        if (0..8).contains(&nr) && (0..8).contains(&nc) {
            match board.get(nr as usize, nc as usize) {
                None => n += 1,
                Some(tp) if tp.get_color() != color => n += 1,
                _ => {}
            }
        }
    }
    n
}

#[inline]
pub(crate) fn count_slider_targets(board: &Board, r: usize, c: usize, color: Color, dirs: &[(i32, i32)]) -> usize {
    let mut n = 0usize;
    for (dr, dc) in dirs.iter() {
        let mut nr = r as i32 + dr;
        let mut nc = c as i32 + dc;
        while (0..8).contains(&nr) && (0..8).contains(&nc) {
            if let Some(tp) = board.get(nr as usize, nc as usize) {
                if tp.get_color() != color {
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
