use chess::piece::move_validators::king_move_validator::is_valid_king_move;
use chess::state::fen::reader::reset_from_fen;

#[test]
fn king_move_rejects_moving_into_check() {
    let mut gs = reset_from_fen("4r2k/8/8/8/8/8/8/4K3 w - - 0 1")
        .expect("Invalid FEN");
    let ok = is_valid_king_move(&mut gs, (0, 4), (1, 4));
    assert!(!ok);
}
