use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};
use crate::board::evaluator::{PawnFileCounts, is_piece};

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
