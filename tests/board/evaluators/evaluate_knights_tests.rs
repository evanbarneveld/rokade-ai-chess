use chess::board::Board;
use chess::board::test_support::{evaluate_knight, is_knight_outpost};
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn is_knight_outpost_detects_supported_knight() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(4, 3, Some(Piece::new(PieceType::Knight, Color::White)));
    board.set(3, 2, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.find_and_set_location_of_kings();

    assert!(is_knight_outpost(&board, 4, 3, Color::White));
}

#[test]
fn evaluate_knight_rewards_development() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(2, 2, Some(Piece::new(PieceType::Knight, Color::White)));
    board.find_and_set_location_of_kings();

    let score = evaluate_knight(&board, 2, 2, Color::White, 24);
    assert!(score > 0);
}

#[test]
fn evaluate_knight_penalizes_rim() {
    let mut rim = Board::empty();
    rim.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    rim.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    rim.set(3, 0, Some(Piece::new(PieceType::Knight, Color::White)));
    rim.find_and_set_location_of_kings();

    let mut center = Board::empty();
    center.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    center.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    center.set(3, 2, Some(Piece::new(PieceType::Knight, Color::White)));
    center.find_and_set_location_of_kings();

    let rim_score = evaluate_knight(&rim, 3, 0, Color::White, 24);
    let center_score = evaluate_knight(&center, 3, 2, Color::White, 24);

    assert!(rim_score < center_score);
}
