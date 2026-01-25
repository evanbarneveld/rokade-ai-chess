use chess::board::Board;
use chess::piece::pieces::{Color, Piece, PieceType};
use chess::search::test_support::threat_resolution_and_evacuation;

#[test]
fn threat_resolution_penalizes_unanswered_promotion_threats() {
    let mut base = Board::empty();
    base.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    base.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    base.set(1, 0, Some(Piece::new(PieceType::Pawn, Color::Black)));
    base.set(0, 7, Some(Piece::new(PieceType::Rook, Color::White)));
    base.find_and_set_location_of_kings();

    let mut post = base;
    post.move_piece_basic((0, 7), (1, 7));

    let delta = threat_resolution_and_evacuation(&base, &post, Color::White, (0, 7), (1, 7), false);
    assert!(delta < 0);
}
