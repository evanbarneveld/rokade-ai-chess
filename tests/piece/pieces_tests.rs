use chess::piece::pieces::{capture_value_cp, opposite_color, piece_value_cp, Color, Piece, PieceType};

#[test]
fn piece_value_helpers_match_expected_values() {
    assert_eq!(piece_value_cp(PieceType::Queen), 900);
    assert_eq!(capture_value_cp(PieceType::King), 0);
}

#[test]
fn fen_char_conversion_round_trips() {
    let queen = Piece::from_fen_char('Q').expect("queen");
    assert_eq!(queen.to_fen_char(), 'Q');
    assert_eq!(queen.get_color(), Color::White);
}

#[test]
fn opposite_color_flips() {
    assert_eq!(opposite_color(Color::White), Color::Black);
}
