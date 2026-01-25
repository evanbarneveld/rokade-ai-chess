#![doc(hidden)]

use crate::board::Board;
use crate::board::evaluation_helpers::{FileClearance, PawnFileCounts};
use crate::piece::pieces::{Color, Piece, PieceType};

pub fn build_attack_maps(board: &Board) -> ([[bool; 8]; 8], [[bool; 8]; 8]) {
    crate::board::attack_maps::build_attack_maps(board)
}

pub fn is_square_attacked_by_opponent(
    board: &mut Board,
    square: (usize, usize),
    active_color: Color,
) -> bool {
    crate::board::checks::square_attacked::is_square_attacked_by_opponent(board, square, active_color)
}

pub fn is_king_in_check_after_move(
    board: &mut Board,
    move_from: (usize, usize),
    move_to: (usize, usize),
    en_passant_target: Option<(usize, usize)>,
) -> bool {
    crate::board::checks::king_in_check::is_king_in_check_after_move(
        board,
        move_from,
        move_to,
        en_passant_target,
    )
}

pub fn mirror_row_for_black(row: usize) -> usize {
    crate::board::pst::mirror_row_for_black(row)
}

pub fn tapered_eval(mg: i32, eg: i32, phase: i32) -> i32 {
    crate::board::pst::tapered_eval(mg, eg, phase)
}

pub fn pst_value_tapered(
    piece: PieceType,
    row: usize,
    col: usize,
    color: Color,
    phase: i32,
) -> i32 {
    crate::board::pst::pst_value_tapered(piece, row, col, color, phase)
}

pub fn evaluate_bishop(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> i32 {
    crate::board::evaluators::evaluate_bishops::evaluate_bishop(board, row, col, color, phase)
}

pub fn evaluate_knight(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> i32 {
    crate::board::evaluators::evaluate_knights::evaluate_knight(board, row, col, color, phase)
}

pub fn is_knight_outpost(board: &Board, row: usize, col: usize, color: Color) -> bool {
    crate::board::evaluators::evaluate_knights::is_knight_outpost(board, row, col, color)
}

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
    crate::board::evaluators::evaluate_rooks::evaluate_rook(
        board,
        row,
        col,
        color,
        phase,
        eg,
        white_pawns,
        black_pawns,
        file_clearance,
    )
}

pub fn rook_file_activity(board: &Board, color: Color, counts: &PawnFileCounts) -> i32 {
    crate::board::evaluators::evaluate_rooks::rook_file_activity(board, color, counts)
}

pub fn doubled_rooks_bonus(board: &Board, color: Color, counts: &PawnFileCounts) -> i32 {
    crate::board::evaluators::evaluate_rooks::doubled_rooks_bonus(board, color, counts)
}

pub fn rook_on_enemy_king_file_bonus(board: &Board, color: Color) -> i32 {
    crate::board::evaluators::evaluate_rooks::rook_on_enemy_king_file_bonus(board, color)
}

pub fn rook_queen_alignment_bonus(board: &Board, color: Color, counts: &PawnFileCounts) -> i32 {
    crate::board::evaluators::evaluate_rooks::rook_queen_alignment_bonus(board, color, counts)
}

pub fn evaluate_queen(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> i32 {
    crate::board::evaluators::evaluate_queens::evaluate_queen(board, row, col, color, phase)
}

pub fn queen_on_semi_open_file_bonus(board: &Board, color: Color, counts: &PawnFileCounts) -> i32 {
    crate::board::evaluators::evaluate_queens::queen_on_semi_open_file_bonus(board, color, counts)
}

pub fn early_queen_penalty(board: &Board, color: Color, counts: &PawnFileCounts) -> i32 {
    crate::board::evaluators::evaluate_queens::early_queen_penalty(board, color, counts)
}

pub fn evaluate_king_shelter_patterns(
    board: &Board,
    color: Color,
    phase: i32,
    king_pos: Option<(usize, usize)>,
) -> i32 {
    crate::board::evaluators::evaluate_king::evaluate_king_shelter_patterns(
        board,
        color,
        phase,
        king_pos,
    )
}

pub fn king_safety(
    board: &Board,
    color: Color,
    phase: i32,
    king_pos: Option<(usize, usize)>,
    pawn_counts: &PawnFileCounts,
) -> i32 {
    crate::board::evaluators::evaluate_king::king_safety(board, color, phase, king_pos, pawn_counts)
}

pub fn king_ring_pressure(
    board: &Board,
    color: Color,
    phase: i32,
    king_pos: Option<(usize, usize)>,
    att_w: &[[bool; 8]; 8],
    att_b: &[[bool; 8]; 8],
) -> i32 {
    crate::board::evaluators::evaluate_king::king_ring_pressure(
        board, color, phase, king_pos, att_w, att_b
    )
}

pub fn king_activity_endgame(king_pos: Option<(usize, usize)>) -> i32 {
    crate::board::evaluators::evaluate_king::king_activity_endgame(king_pos)
}

pub fn development_penalty_on_backrank(board: &Board, color: Color, phase: i32) -> i32 {
    crate::board::evaluators::evaluate_king::development_penalty_on_backrank(board, color, phase)
}

pub fn is_king_in_front_of_pawn(king: (usize, usize), pawn_r: usize, pawn_c: usize, pawn_color: Color) -> bool {
    crate::board::evaluators::evaluate_king::is_king_in_front_of_pawn(king, pawn_r, pawn_c, pawn_color)
}

pub fn evaluate_pawn(
    board: &Board,
    row: usize,
    col: usize,
    color: Color,
    phase: i32,
    king_w: Option<(usize, usize)>,
    king_b: Option<(usize, usize)>,
    att_w: &[[bool; 8]; 8],
    att_b: &[[bool; 8]; 8],
    pawn_counts: &PawnFileCounts,
) -> i32 {
    crate::board::evaluators::evaluate_pawns::evaluate_pawn(
        board,
        row,
        col,
        color,
        phase,
        king_w,
        king_b,
        att_w,
        att_b,
        pawn_counts,
    )
}

pub fn pawn_file_counts(board: &Board) -> PawnFileCounts {
    crate::board::evaluators::evaluate_pawns::pawn_file_counts(board)
}

pub fn is_passed_pawn(board: &Board, row: usize, col: usize, color: Color) -> bool {
    crate::board::evaluators::evaluate_pawns::is_passed_pawn(board, row, col, color)
}

pub fn has_clear_promotion_path(board: &Board, row: usize, col: usize, color: Color) -> bool {
    crate::board::evaluators::evaluate_pawns::has_clear_promotion_path(board, row, col, color)
}

pub fn evaluate_pawn_islands(pawn_counts: &PawnFileCounts, color: Color, phase: i32) -> i32 {
    crate::board::evaluators::evaluate_pawns::evaluate_pawn_islands(pawn_counts, color, phase)
}

pub fn evaluate_pawn_chains(board: &Board, color: Color, phase: i32) -> i32 {
    crate::board::evaluators::evaluate_pawns::evaluate_pawn_chains(board, color, phase)
}

pub fn evaluate_pawn_tension(board: &Board, color: Color, phase: i32) -> i32 {
    crate::board::evaluators::evaluate_pawns::evaluate_pawn_tension(board, color, phase)
}

pub fn evaluate_pawn_storm(
    board: &Board,
    color: Color,
    phase: i32,
    enemy_king: Option<(usize, usize)>,
    own_king: Option<(usize, usize)>,
) -> i32 {
    crate::board::evaluators::evaluate_pawns::evaluate_pawn_storm(
        board,
        color,
        phase,
        enemy_king,
        own_king,
    )
}

pub fn pawn_majority_bonus(pawn_counts: &PawnFileCounts, color: Color, phase: i32) -> i32 {
    crate::board::evaluators::evaluate_pawns::pawn_majority_bonus(pawn_counts, color, phase)
}

pub fn find_passed_pawn_on_file(board: &Board, file: usize, color: Color) -> Option<(usize, usize)> {
    crate::board::evaluators::evaluate_pawns::find_passed_pawn_on_file(board, file, color)
}

pub fn is_hole_square_limited(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> bool {
    crate::board::evaluators::evaluate_pawns::is_hole_square_limited(board, row, col, color, phase)
}

pub fn simulate_move(board: &Board, from: (usize, usize), to: (usize, usize), promo: Option<char>) -> (Board, Option<Piece>) {
    crate::search::test_support::simulate_move(board, from, to, promo)
}
