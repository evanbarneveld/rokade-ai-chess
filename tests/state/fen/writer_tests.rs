use chess::state::fen::reader::reset_from_fen;
use chess::state::test_support::game_state_to_fen_string;

#[test]
fn game_state_to_fen_round_trips() {
    let fen = "8/8/8/8/8/8/6k1/6K1 w - - 12 34";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let out = game_state_to_fen_string(gs);
    assert_eq!(out, fen);
}
