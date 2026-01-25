use chess::board::Board;
use chess::board::test_support::build_attack_maps;
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn build_attack_maps_marks_attacks_and_stops_on_blockers() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(3, 3, Some(Piece::new(PieceType::Knight, Color::White)));
    board.set(0, 0, Some(Piece::new(PieceType::Bishop, Color::Black)));
    board.set(1, 1, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.find_and_set_location_of_kings();

    let (white_attacks, black_attacks) = build_attack_maps(&board);

    assert!(white_attacks[5][4], "white knight should attack e6");
    assert!(black_attacks[1][1], "black bishop should attack b2");
    assert!(
        !black_attacks[2][2],
        "blocked bishop should not attack beyond the blocker"
    );
}
