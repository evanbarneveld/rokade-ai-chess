use chess::board::Board;
use chess::piece::move_validators::bishop_move_validator::is_valid_bishop_move;
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn bishop_move_validates_diagonals_and_blocks() {
    let mut board = Board::empty();
    board.set(0, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    board.set(1, 3, Some(Piece::new(PieceType::Pawn, Color::White)));

    assert!(is_valid_bishop_move(&mut board, (0, 2), (1, 1), false));
    assert!(!is_valid_bishop_move(&mut board, (0, 2), (2, 4), false));
}
