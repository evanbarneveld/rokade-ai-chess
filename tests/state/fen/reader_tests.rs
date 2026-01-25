use chess::state::fen::reader::reset_from_fen;
use chess::piece::pieces::Color;

#[test]
fn reset_from_fen_parses_basic_position() {
    let gs = reset_from_fen("8/8/8/8/8/8/6k1/6K1 w - - 0 1")
        .expect("Invalid FEN");
    assert_eq!(gs.active_color(), Color::White);
    assert_eq!(gs.full_move_number(), 1);
}

#[test]
fn reset_from_fen_rejects_invalid_fields() {
    let err = reset_from_fen("invalid fen").err();
    assert!(err.is_some());
}
