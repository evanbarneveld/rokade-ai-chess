use chess::board::Board;
use chess::piece::pieces::{Color, Piece, PieceType};
use chess::search::test_support::opponent_knight_check_fork_penalty;

#[test]
fn opponent_knight_check_fork_penalty_detects_forks() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(3, 7, Some(Piece::new(PieceType::Knight, Color::Black)));
    board.set(4, 4, Some(Piece::new(PieceType::Queen, Color::White)));
    board.find_and_set_location_of_kings();

    let penalty = opponent_knight_check_fork_penalty(&board, Color::White, (4, 4));
    assert!(penalty < 0);
}
