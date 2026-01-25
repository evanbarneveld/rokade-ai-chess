use chess::board::Board;
use chess::board::test_support::{mirror_row_for_black, simulate_move};
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn board_test_support_helpers_work() {
    assert_eq!(mirror_row_for_black(2), 5);

    let mut board = Board::empty();
    board.set(1, 1, Some(Piece::new(PieceType::Pawn, Color::White)));
    let (post, moved) = simulate_move(&board, (1, 1), (2, 1), None);
    assert!(moved.is_some());
    assert!(post.get(2, 1).is_some());
}
