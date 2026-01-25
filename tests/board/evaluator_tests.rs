use chess::board::Board;
use chess::board::evaluator::evaluate_position;
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn evaluate_position_is_zero_for_insufficient_material() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.find_and_set_location_of_kings();

    let score = evaluate_position(&board, Color::White);
    assert_eq!(score, 0);
}

#[test]
fn evaluate_position_includes_tempo_bonus() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(3, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.set(0, 1, Some(Piece::new(PieceType::Knight, Color::White)));
    board.set(0, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    board.find_and_set_location_of_kings();

    let white_to_move = evaluate_position(&board, Color::White);
    let black_to_move = evaluate_position(&board, Color::Black);
    assert!(white_to_move > black_to_move);
}

#[test]
fn evaluate_position_rewards_safe_exchange_threats() {
    let mut base = Board::empty();
    base.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    base.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    base.set(6, 5, Some(Piece::new(PieceType::Rook, Color::Black)));
    base.set(1, 0, Some(Piece::new(PieceType::Bishop, Color::White)));
    base.find_and_set_location_of_kings();

    let base_score = evaluate_position(&base, Color::White);

    let mut threat = base;
    threat.set(1, 0, None);
    threat.set(3, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    let threat_score = evaluate_position(&threat, Color::White);

    assert!(threat_score > base_score);
}
