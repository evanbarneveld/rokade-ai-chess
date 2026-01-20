use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};
use crate::board::evaluator::{PawnFileCounts, FileClearance, is_piece, find_king, opponent};

pub fn evaluate_rook(
    board: &Board,
    row: usize,
    col: usize,
    color: Color,
    phase: i32,
    eg: i32,
    white_pawns: i32,
    black_pawns: i32,
    file_clearance: &FileClearance,
) -> i32 {
    let mut val = 0;
    if eg > 0 {
        // Rook on 7th
        let on_7th = match color {
            Color::White => row == 6 && black_pawns > 0,
            Color::Black => row == 1 && white_pawns > 0,
        };
        if on_7th { val += (30 * eg) / 24; }

        // Rook behind passed pawn
        if let Some((pp_r, _)) = crate::board::evaluators::evaluate_pawns::find_passed_pawn_on_file(board, col, color) {
            let behind = match color { Color::White => row < pp_r, Color::Black => row > pp_r };
            if behind && file_clearance.is_clear_between(row, pp_r, col) {
                let adv = match color { Color::White => pp_r as i32, Color::Black => (7 - pp_r) as i32 };
                val += ((12 + 2 * adv) * eg) / 24;
            }
        }

        // Cut-off king
        if let Some((ek_r, ek_c)) = find_king(board, opponent(color)) {
            if col == ek_c
                && file_clearance.is_clear_between(row, ek_r, col)
                && (row as i32 - ek_r as i32).abs() >= 2
            {
                val += (10 * eg) / 24;
            }
            if row == ek_r
                && rank_clear_between(board, col, ek_c, row)
                && (col as i32 - ek_c as i32).abs() >= 2
            {
                val += (10 * eg) / 24;
            }
        }
    }
    if phase > 0 {
        let (is_back_rank, start_row) = match color { Color::White => (row==0, 1usize), Color::Black => (row==7, 6usize) };
        if is_back_rank {
            let left_block = col > 0 && is_piece(board, start_row, col-1, color, PieceType::Pawn);
            let right_block = col < 7 && is_piece(board, start_row, col+1, color, PieceType::Pawn);
            if left_block && right_block { val -= (16 * phase) / 24; }
        }
    }
    val
}

#[allow(dead_code)]
pub fn file_clear_between(board: &Board, r1: usize, r2: usize, file: usize) -> bool {
    let start = r1.min(r2);
    let end = r1.max(r2);
    for r in (start + 1)..end {
        if board.get(r, file).is_some() { return false; }
    }
    true
}

pub fn rank_clear_between(board: &Board, c1: usize, c2: usize, rank: usize) -> bool {
    let start = c1.min(c2);
    let end = c1.max(c2);
    for c in (start + 1)..end {
        if board.get(rank, c).is_some() { return false; }
    }
    true
}

pub fn rook_file_activity(board: &Board, color: Color, counts: &PawnFileCounts) -> i32 {
    let mut score = 0;
    for c in 0..8 {
        let mut has_rook = false;
        for r in 0..8 { if is_piece(board, r, c, color, PieceType::Rook) { has_rook = true; break; } }
        if has_rook {
            let (friendly, enemy) = match color { Color::White => (counts.white[c], counts.black[c]), Color::Black => (counts.black[c], counts.white[c]) };
            if friendly == 0 {
                if enemy == 0 { score += 15; } else { score += 10; }
            }
        }
    }
    score
}

pub fn doubled_rooks_bonus(board: &Board, color: Color, _counts: &PawnFileCounts) -> i32 {
    let mut score = 0;
    for c in 0..8 {
        let mut rooks = 0;
        for r in 0..8 { if is_piece(board, r, c, color, PieceType::Rook) { rooks += 1; } }
        if rooks >= 2 { score += 12; }
    }
    for r in 0..8 {
        let mut rooks = 0;
        for c in 0..8 { if is_piece(board, r, c, color, PieceType::Rook) { rooks += 1; } }
        if rooks >= 2 { score += 12; }
    }
    score
}

pub fn rook_on_enemy_king_file_bonus(board: &Board, color: Color) -> i32 {
    use crate::board::evaluator::find_king;
    use crate::board::evaluator::opponent;
    if let Some((_, ek_c)) = find_king(board, opponent(color)) {
        for r in 0..8 { if is_piece(board, r, ek_c, color, PieceType::Rook) { return 10; } }
    }
    0
}
