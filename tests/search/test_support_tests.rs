use chess::board::Board;
use chess::piece::pieces::{Color, Piece, PieceType};
use chess::search::core::advanced_search::SEARCH_ABORTED;
use chess::search::test_support::{has_search_aborted, simulate_move};

#[test]
fn has_search_aborted_detects_abort_marker() {
    let results = vec![((0, 0), (0, 0), None, SEARCH_ABORTED, SEARCH_ABORTED)];
    assert!(has_search_aborted(&results));
}

#[test]
fn simulate_move_applies_piece_updates() {
    let mut board = Board::empty();
    board.set(1, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    let (post, moved) = simulate_move(&board, (1, 0), (3, 0), None);
    assert!(moved.is_some());
    assert!(post.get(3, 0).is_some());
    assert!(post.get(1, 0).is_none());
}
