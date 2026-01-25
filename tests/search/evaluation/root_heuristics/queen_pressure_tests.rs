use chess::board::Board;
use chess::piece::pieces::{Color, Piece, PieceType};
use chess::search::test_support::queen_kingside_pressure_bonus;

#[test]
fn queen_kingside_pressure_bonus_detects_attacks() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(0, 3, Some(Piece::new(PieceType::Queen, Color::White)));
    board.find_and_set_location_of_kings();

    let bonus = queen_kingside_pressure_bonus(&board, Color::White, (0, 3), (3, 7));
    assert!(bonus > 0);
}
