use chess::board::Board;
use chess::piece::move_validators::queen_move_validator::is_valid_queen_move;
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn queen_move_validates_diagonal_and_blocks() {
    let mut board = Board::empty();
    board.set(0, 3, Some(Piece::new(PieceType::Queen, Color::White)));
    board.set(1, 4, Some(Piece::new(PieceType::Pawn, Color::White)));

    assert!(is_valid_queen_move(&mut board, (0, 3), (1, 3), false));
    assert!(!is_valid_queen_move(&mut board, (0, 3), (2, 5), false));
}
