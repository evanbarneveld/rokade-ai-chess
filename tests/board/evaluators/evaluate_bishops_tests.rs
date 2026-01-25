use chess::board::Board;
use chess::board::test_support::evaluate_bishop;
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn evaluate_bishop_rewards_developed_bishops() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(3, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    board.find_and_set_location_of_kings();

    let score = evaluate_bishop(&board, 3, 2, Color::White, 24);
    assert!(score > 0);
}

#[test]
fn evaluate_bishop_does_not_reward_home_bishops() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(0, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    board.find_and_set_location_of_kings();

    let score = evaluate_bishop(&board, 0, 2, Color::White, 24);
    assert_eq!(score, 0);
}

#[test]
fn evaluate_bishop_penalizes_bad_bishop_pawn_color() {
    let mut bad = Board::empty();
    bad.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    bad.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    bad.set(2, 4, Some(Piece::new(PieceType::Bishop, Color::White)));
    bad.set(2, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    bad.set(2, 2, Some(Piece::new(PieceType::Pawn, Color::White)));
    bad.set(2, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    bad.set(4, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    bad.find_and_set_location_of_kings();

    let mut good = Board::empty();
    good.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    good.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    good.set(2, 4, Some(Piece::new(PieceType::Bishop, Color::White)));
    good.set(1, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    good.set(1, 2, Some(Piece::new(PieceType::Pawn, Color::White)));
    good.set(1, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    good.set(1, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    good.find_and_set_location_of_kings();

    let bad_score = evaluate_bishop(&bad, 2, 4, Color::White, 24);
    let good_score = evaluate_bishop(&good, 2, 4, Color::White, 24);

    assert!(bad_score < good_score);
}

#[test]
fn evaluate_bishop_rewards_mobility() {
    let mut open = Board::empty();
    open.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    open.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    open.set(3, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    open.set(3, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    open.set(1, 2, Some(Piece::new(PieceType::Pawn, Color::White)));
    open.set(3, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    open.set(1, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    open.find_and_set_location_of_kings();

    let mut blocked = Board::empty();
    blocked.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    blocked.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    blocked.set(3, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    blocked.set(3, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    blocked.set(1, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    blocked.set(2, 1, Some(Piece::new(PieceType::Pawn, Color::White)));
    blocked.set(2, 3, Some(Piece::new(PieceType::Pawn, Color::White)));
    blocked.find_and_set_location_of_kings();

    let open_score = evaluate_bishop(&open, 3, 2, Color::White, 24);
    let blocked_score = evaluate_bishop(&blocked, 3, 2, Color::White, 24);

    assert!(open_score > blocked_score);
}
