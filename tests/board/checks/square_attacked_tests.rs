use chess::board::Board;
use chess::board::test_support::is_square_attacked_by_opponent;
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn is_square_attacked_by_opponent_detects_rook_attack() {
    let mut board = Board::empty();
    board.set(0, 0, Some(Piece::new(PieceType::Rook, Color::White)));
    board.set(1, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 7, Some(Piece::new(PieceType::King, Color::Black)));
    board.find_and_set_location_of_kings();

    let attacked = is_square_attacked_by_opponent(&mut board, (0, 7), Color::Black);
    assert!(attacked);
}

#[test]
fn is_square_attacked_by_opponent_detects_pawn_attack() {
    let mut board = Board::empty();
    board.set(3, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.find_and_set_location_of_kings();

    let attacked = is_square_attacked_by_opponent(&mut board, (4, 3), Color::Black);
    assert!(attacked);
}
