use chess::piece::piece_mover::PieceMover;
use chess::state::fen::reader::reset_from_fen;

#[test]
fn piece_mover_executes_legal_pawn_move() {
    let mut gs = reset_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        .expect("Invalid FEN");
    let ok = PieceMover::move_piece(&mut gs, (1, 4), (3, 4), false, None);
    assert!(ok);
    assert_eq!(gs.en_passant_target(), Some((2, 4)));
}

#[test]
fn piece_mover_rejects_illegal_move() {
    let mut gs = reset_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        .expect("Invalid FEN");
    let ok = PieceMover::move_piece(&mut gs, (1, 4), (4, 4), false, None);
    assert!(!ok);
}
