use chess::board::Board;
use chess::piece::move_validators::rook_move_validator::is_valid_rook_move;
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn rook_move_validates_straight_lines() {
    let mut board = Board::empty();
    board.set(0, 0, Some(Piece::new(PieceType::Rook, Color::White)));
    board.set(0, 2, Some(Piece::new(PieceType::Pawn, Color::White)));

    assert!(is_valid_rook_move(&mut board, (0, 0), (0, 1), false));
    assert!(!is_valid_rook_move(&mut board, (0, 0), (0, 3), false));
}
