use crate::piece::pieces::{Color, PieceType};

// Piece-Square Tables (from White's perspective, row 0 = White back rank)
// Values in centipawns; lightweight, generic PSTs
// Flipped vertically so that advancing pawns are rewarded toward promotion
pub(crate) const PST_PAWN: [[i32; 8]; 8] = [
    [  0,   0,   0,   0,   0,   0,   0,   0],  // row 0
    [  5,  10,  10, -20, -20,  10,  10,   5],  // row 1 (start rank)
    [  5,  -5, -10,   0,   0, -10,  -5,   5],
    [  0,   0,   0,  20,  20,   0,   0,   0],
    [  5,   5,  10,  25,  25,  10,   5,   5],
    [ 10,  10,  20,  30,  30,  20,  10,  10],
    [ 50,  50,  50,  50,  50,  50,  50,  50],  // advanced pawns credited
    [  0,  0,  0,  0,  0,  0,  0,  0],         // row 7 (promotion rank)
];

pub(crate) const PST_KNIGHT: [[i32; 8]; 8] = [
    [-50, -40, -30, -30, -30, -30, -40, -50],
    [-40, -20,   0,   0,   0,   0, -20, -40],
    [-30,   0,  10,  15,  15,  10,   0, -30],
    [-30,   5,  15,  20,  20,  15,   5, -30],
    [-30,   0,  15,  20,  20,  15,   0, -30],
    [-30,   5,  10,  15,  15,  10,   5, -30],
    [-40, -20,   0,   5,   5,   0, -20, -40],
    [-50, -40, -30, -30, -30, -30, -40, -50],
];

// Endgame PSTs for minor/major pieces to improve endgame play
pub(crate) const PST_KNIGHT_ENDGAME: [[i32; 8]; 8] = [
    [-40, -30, -20, -20, -20, -20, -30, -40],
    [-30, -10,   0,   0,   0,   0, -10, -30],
    [-20,   0,  10,  15,  15,  10,   0, -20],
    [-20,   5,  15,  20,  20,  15,   5, -20],
    [-20,   0,  15,  20,  20,  15,   0, -20],
    [-20,   5,  10,  15,  15,  10,   5, -20],
    [-30, -10,   0,   5,   5,   0, -10, -30],
    [-40, -30, -20, -20, -20, -20, -30, -40],
];

pub(crate) const PST_BISHOP: [[i32; 8]; 8] = [
    [-20, -10, -10, -10, -10, -10, -10, -20],
    [-10,   5,   0,   0,   0,   0,   5, -10],
    [-10,  10,  10,  10,  10,  10,  10, -10],
    [-10,   0,  10,  10,  10,  10,   0, -10],
    [-10,   5,   5,  10,  10,   5,   5, -10],
    [-10,   0,   5,  10,  10,   5,   0, -10],
    [-10,   0,   0,   0,   0,   0,   0, -10],
    [-20, -10, -10, -10, -10, -10, -10, -20],
];

pub(crate) const PST_BISHOP_ENDGAME: [[i32; 8]; 8] = [
    [-15,  -8,  -8,  -8,  -8,  -8,  -8, -15],
    [ -8,   2,   4,   4,   4,   4,   2,  -8],
    [ -8,   6,   8,  10,  10,   8,   6,  -8],
    [ -8,   6,  12,  14,  14,  12,   6,  -8],
    [ -8,   6,  12,  14,  14,  12,   6,  -8],
    [ -8,   6,   8,  10,  10,   8,   6,  -8],
    [ -8,   2,   4,   4,   4,   4,   2,  -8],
    [-15,  -8,  -8,  -8,  -8,  -8,  -8, -15],
];

pub(crate) const PST_ROOK: [[i32; 8]; 8] = [
    [  0,   0,   5,  10,  10,   5,   0,   0],
    [ -5,   0,   0,   0,   0,   0,   0,  -5],
    [ -5,   0,   0,   0,   0,   0,   0,  -5],
    [ -5,   0,   0,   0,   0,   0,   0,  -5],
    [ -5,   0,   0,   0,   0,   0,   0,  -5],
    [ -5,   0,   0,   0,   0,   0,   0,  -5],
    [  5,  10,  10,  10,  10,  10,  10,   5],
    [  0,   0,   0,   0,   0,   0,   0,   0],
];

pub(crate) const PST_ROOK_ENDGAME: [[i32; 8]; 8] = [
    [  0,   0,   5,  10,  10,   5,   0,   0],
    [  0,   0,   6,  10,  10,   6,   0,   0],
    [  0,   2,   8,  12,  12,   8,   2,   0],
    [  0,   4,  10,  14,  14,  10,   4,   0],
    [  0,   4,  10,  14,  14,  10,   4,   0],
    [  0,   2,   8,  12,  12,   8,   2,   0],
    [  0,   0,   6,  10,  10,   6,   0,   0],
    [  0,   0,   4,   8,   8,   4,   0,   0],
];

pub(crate) const PST_QUEEN: [[i32; 8]; 8] = [
    [-20, -10, -10,  -5,  -5, -10, -10, -20],
    [-10,   0,   0,   0,   0,   0,   0, -10],
    [-10,   0,   5,   5,   5,   5,   0, -10],
    [ -5,   0,   5,   5,   5,   5,   0,  -5],
    [  0,   0,   5,   5,   5,   5,   0,  -5],
    [-10,   5,   5,   5,   5,   5,   0, -10],
    [-10,   0,   5,   0,   0,   0,   0, -10],
    [-20, -10, -10,  -5,  -5, -10, -10, -20],
];

pub(crate) const PST_QUEEN_ENDGAME: [[i32; 8]; 8] = [
    [-10,  -6,  -6,  -4,  -4,  -6,  -6, -10],
    [ -6,  -4,  -2,  -2,  -2,  -2,  -4,  -6],
    [ -6,  -2,   0,   2,   2,   0,  -2,  -6],
    [ -4,  -2,   2,   4,   4,   2,  -2,  -4],
    [ -4,  -2,   2,   6,   6,   2,  -2,  -4],
    [ -6,  -2,   0,   2,   2,   0,  -2,  -6],
    [ -6,  -4,  -2,  -2,  -2,  -2,  -4,  -6],
    [-10,  -6,  -6,  -4,  -4,  -6,  -6, -10],
];

pub(crate) const PST_KING_MIDGAME: [[i32; 8]; 8] = [
    [-30, -40, -40, -50, -50, -40, -40, -30],
    [-30, -40, -40, -50, -50, -40, -40, -30],
    [-30, -40, -40, -50, -50, -40, -40, -30],
    [-30, -40, -40, -50, -50, -40, -40, -30],
    [-20, -30, -30, -40, -40, -30, -30, -20],
    [-10, -20, -20, -20, -20, -20, -20, -10],
    [ 20,  20,   0,   0,   0,   0,  20,  20],
    [ 20,  30,  10,   0,   0,  10,  30,  20],
];

// Endgame king PST to encourage centralization and activity in simplified positions
pub(crate) const PST_KING_ENDGAME: [[i32; 8]; 8] = [
    [-10, -10, -10, -10, -10, -10, -10, -10],
    [ -5,   0,   0,   0,   0,   0,   0,  -5],
    [ -5,   0,  10,  15,  15,  10,   0,  -5],
    [ -5,   0,  15,  20,  20,  15,   0,  -5],
    [ -5,   0,  15,  20,  20,  15,   0,  -5],
    [ -5,   0,  10,  15,  15,  10,   0,  -5],
    [ -5,  -5,   0,  10,  10,   0,  -5,  -5],
    [-10, -10, -10, -10, -10, -10, -10, -10],
];

#[inline]
pub(crate) fn mirror_row_for_black(row: usize) -> usize {
    7 - row
}

#[inline]
pub(crate) fn pst_value_tapered(
    piece: PieceType,
    row: usize,
    col: usize,
    color: Color,
    phase: i32,
) -> i32 {
    // Map black squares by mirroring rows so PSTs are from White's perspective
    let (r, c) = match color {
        Color::White => (row, col),
        Color::Black => (mirror_row_for_black(row), col),
    };

    // Midgame values
    let mg = match piece {
        PieceType::Pawn => PST_PAWN[r][c],
        PieceType::Knight => PST_KNIGHT[r][c],
        PieceType::Bishop => PST_BISHOP[r][c],
        PieceType::Rook => PST_ROOK[r][c],
        PieceType::Queen => PST_QUEEN[r][c],
        PieceType::King => PST_KING_MIDGAME[r][c],
    };

    // Endgame values
    let eg = match piece {
        PieceType::Pawn => PST_PAWN[r][c], // keep pawn PST identical across phases here
        PieceType::Knight => PST_KNIGHT_ENDGAME[r][c],
        PieceType::Bishop => PST_BISHOP_ENDGAME[r][c],
        PieceType::Rook => PST_ROOK_ENDGAME[r][c],
        PieceType::Queen => PST_QUEEN_ENDGAME[r][c],
        PieceType::King => PST_KING_ENDGAME[r][c],
    };

    // Linear interpolation between midgame and endgame based on phase [0..24]
    tapered_eval(mg, eg, phase)
}

#[inline]
pub(crate) fn tapered_eval(mg: i32, eg: i32, phase: i32) -> i32 {
    (mg * phase + eg * (24 - phase)) / 24
}
