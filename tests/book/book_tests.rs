use chess::book::test_support::book_pick;
use chess::state::fen::reader::reset_from_fen;

#[test]
fn book_pick_returns_weighted_best_move_deterministically() {
    let gs = reset_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        .expect("Invalid FEN");
    let mv = book_pick(&gs, true).expect("expected book move");
    assert_eq!(mv, ((1, 4), (3, 4))); // e2e4
}
