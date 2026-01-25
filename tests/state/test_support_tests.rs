use chess::state::test_support::{game_state_to_fen_string, CastlingRights};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn state_test_support_exposes_castling_rights() {
    let rights = CastlingRights::all();
    assert!(rights.white_kingside());
}

#[test]
fn state_test_support_exposes_fen_writer() {
    let gs = reset_from_fen("8/8/8/8/8/8/6k1/6K1 w - - 0 1")
        .expect("Invalid FEN");
    let fen = game_state_to_fen_string(gs);
    assert_eq!(fen, "8/8/8/8/8/8/6k1/6K1 w - - 0 1");
}
