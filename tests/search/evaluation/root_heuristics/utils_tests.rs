use chess::board::Board;
use chess::piece::pieces::{Color, Piece, PieceType};
use chess::search::test_support::simulate_move;

#[test]
fn simulate_move_handles_en_passant_capture() {
    let mut b = Board::empty();
    b.set(4, 4, Some(Piece::new(PieceType::Pawn, Color::White))); // e5
    b.set(4, 3, Some(Piece::new(PieceType::Pawn, Color::Black))); // d5

    let (after, _) = simulate_move(&b, (4, 4), (5, 3), None); // e5xd6 ep
    assert!(after.get(4, 3).is_none(), "expected ep capture to remove pawn on d5");
    assert!(matches!(after.get(5, 3), Some(p) if p.get_color() == Color::White));
}

#[test]
fn simulate_move_handles_castling_rook() {
    let mut b = Board::empty();
    b.set(0, 4, Some(Piece::new(PieceType::King, Color::White))); // e1
    b.set(0, 7, Some(Piece::new(PieceType::Rook, Color::White))); // h1

    let (after, _) = simulate_move(&b, (0, 4), (0, 6), None); // O-O
    assert!(matches!(after.get(0, 5), Some(p) if p.get_type() == PieceType::Rook));
    assert!(after.get(0, 7).is_none());
    assert_eq!(after.get_king_location(Color::White), (0, 6));
}

#[test]
fn simulate_move_handles_promotion_piece() {
    let mut b = Board::empty();
    b.set(6, 0, Some(Piece::new(PieceType::Pawn, Color::White))); // a7

    let (after, _) = simulate_move(&b, (6, 0), (7, 0), Some('n'));
    assert!(matches!(after.get(7, 0), Some(p) if p.get_type() == PieceType::Knight));
}
