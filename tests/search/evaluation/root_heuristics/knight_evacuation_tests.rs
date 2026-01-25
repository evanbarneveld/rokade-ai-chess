use chess::board::Board;
use chess::piece::pieces::{Color, Piece, PieceType};
use chess::search::test_support::knight_safe_squares;

#[test]
fn knight_safe_squares_lists_legal_targets() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(0, 1, Some(Piece::new(PieceType::Knight, Color::White)));
    board.find_and_set_location_of_kings();

    let squares = knight_safe_squares(&board, Color::White, (0, 1));
    assert!(squares.contains(&(2, 0)));
    assert!(squares.contains(&(2, 2)));
}
