use chess::board::Board;
use chess::piece::pieces::{Color, Piece, PieceType};
use chess::search::test_support::critical_square_defense_bonus;

#[test]
fn critical_square_defense_bonus_triggers_when_defending_f2() {
    let mut base = Board::empty();
    base.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    base.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    base.set(1, 7, Some(Piece::new(PieceType::Knight, Color::White)));
    base.set(4, 2, Some(Piece::new(PieceType::Bishop, Color::Black)));
    base.set(3, 7, Some(Piece::new(PieceType::Queen, Color::Black)));
    base.find_and_set_location_of_kings();

    let mut post = base;
    post.move_piece_basic((1, 7), (3, 6));

    let bonus = critical_square_defense_bonus(&base, &post, Color::White, (1, 7), (3, 6));
    assert!(bonus > 0);
}
