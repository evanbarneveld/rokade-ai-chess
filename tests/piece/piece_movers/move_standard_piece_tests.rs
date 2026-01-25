use chess::piece::piece_movers::move_standard_piece::move_standard_piece;
use chess::state::fen::reader::reset_from_fen;

#[test]
fn move_standard_piece_clears_en_passant_and_updates_clock() {
    let mut gs = reset_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        .expect("Invalid FEN");
    gs.set_en_passant_target(Some((2, 4)));
    let ok = move_standard_piece(&mut gs, (0, 1), (2, 2), false);
    assert!(ok);
    assert!(gs.en_passant_target().is_none());
    assert_eq!(gs.half_move_clock(), 1);
}
