use chess::board::Board;
use chess::piece::pieces::{Color, Piece, PieceType};
use chess::search::test_support::king_safety_root_heuristics;

#[test]
fn king_safety_penalizes_moving_into_attacked_square() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::Rook, Color::Black)));
    board.set(7, 7, Some(Piece::new(PieceType::King, Color::Black)));
    board.find_and_set_location_of_kings();

    let delta = king_safety_root_heuristics(&board, Color::White, (0, 4), (1, 4), false);
    assert!(delta < 0);
}
