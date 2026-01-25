use chess::piece::pieces::{Color, PieceType};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn make_and_unmake_move_fast_restore_state() {
    let mut gs = reset_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        .expect("Invalid FEN");
    let undo = gs.make_move_fast((1, 4), (3, 4), None);
    assert_eq!(gs.active_color(), Color::Black);

    gs.unmake_move_fast(undo);
    assert_eq!(gs.active_color(), Color::White);
    let piece = gs.board().get(1, 4).expect("pawn back");
    assert_eq!(piece.get_type(), PieceType::Pawn);
}

#[test]
fn make_move_fast_handles_en_passant_capture() {
    let mut gs = reset_from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1")
        .expect("Invalid FEN");
    let undo = gs.make_move_fast((4, 4), (5, 3), None);
    assert!(gs.board().get(4, 3).is_none());
    assert!(gs.board().get(5, 3).is_some());

    gs.unmake_move_fast(undo);
    assert!(gs.board().get(4, 3).is_some());
    assert!(gs.board().get(5, 3).is_none());
}
