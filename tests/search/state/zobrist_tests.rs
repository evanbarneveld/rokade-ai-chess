use chess::search::test_support::{compute_zobrist_full, zobrist_update_ep};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn compute_zobrist_full_matches_game_state_key() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let rights = gs.castling_rights();
    let key = compute_zobrist_full(
        gs.board(),
        gs.active_color(),
        &rights,
        gs.en_passant_target(),
    );

    assert_eq!(key, gs.zobrist_key());
}

#[test]
fn zobrist_update_ep_matches_full_recompute() {
    let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let rights = gs.castling_rights();
    let old_ep = gs.en_passant_target();
    assert!(old_ep.is_some(), "expected en passant target in FEN");

    let key_with_ep = compute_zobrist_full(gs.board(), gs.active_color(), &rights, old_ep);
    let key_without_ep = compute_zobrist_full(gs.board(), gs.active_color(), &rights, None);
    let updated = zobrist_update_ep(key_with_ep, old_ep, None);

    assert_eq!(updated, key_without_ep);
}
