use chess::board::Board;
use chess::board::evaluator::FileClearance;
use chess::board::test_support::{
    evaluate_rook,
    pawn_file_counts,
    rook_file_activity,
    rook_on_enemy_king_file_bonus,
    rook_queen_alignment_bonus,
};
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn rook_file_activity_rewards_open_files() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(0, 0, Some(Piece::new(PieceType::Rook, Color::White)));
    board.find_and_set_location_of_kings();

    let counts = pawn_file_counts(&board);
    let score = rook_file_activity(&board, Color::White, &counts);
    assert!(score > 0);
}

#[test]
fn rook_on_enemy_king_file_bonus_detects_pressure() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 0, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(3, 0, Some(Piece::new(PieceType::Rook, Color::White)));
    board.find_and_set_location_of_kings();

    let bonus = rook_on_enemy_king_file_bonus(&board, Color::White);
    assert!(bonus > 0);
}

#[test]
fn rook_queen_alignment_bonus_rewards_open_file_stack() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(0, 0, Some(Piece::new(PieceType::Rook, Color::White)));
    board.set(3, 0, Some(Piece::new(PieceType::Queen, Color::White)));
    board.find_and_set_location_of_kings();

    let counts = pawn_file_counts(&board);
    let bonus = rook_queen_alignment_bonus(&board, Color::White, &counts);
    assert!(bonus > 0);
}

#[test]
fn rook_blockade_bonus_rewards_blocking_passed_pawn() {
    let mut blocked = Board::empty();
    blocked.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    blocked.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    blocked.set(2, 3, Some(Piece::new(PieceType::Rook, Color::White)));
    blocked.set(3, 3, Some(Piece::new(PieceType::Pawn, Color::Black)));
    blocked.find_and_set_location_of_kings();

    let blocked_clearance = FileClearance::new(&blocked);
    let blocked_score = evaluate_rook(&blocked, 2, 3, Color::White, 0, 24, 0, 1, &blocked_clearance);

    let mut unblocked = Board::empty();
    unblocked.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    unblocked.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    unblocked.set(4, 3, Some(Piece::new(PieceType::Rook, Color::White)));
    unblocked.set(3, 3, Some(Piece::new(PieceType::Pawn, Color::Black)));
    unblocked.find_and_set_location_of_kings();

    let unblocked_clearance = FileClearance::new(&unblocked);
    let unblocked_score = evaluate_rook(&unblocked, 4, 3, Color::White, 0, 24, 0, 1, &unblocked_clearance);

    assert!(blocked_score > unblocked_score);
}
