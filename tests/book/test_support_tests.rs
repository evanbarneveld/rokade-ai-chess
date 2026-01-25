use chess::book::test_support::book_pick;
use chess::state::fen::reader::reset_from_fen;

#[test]
fn book_test_support_exposes_book_pick() {
    let gs = reset_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        .expect("Invalid FEN");
    let mv = book_pick(&gs, true);
    assert!(mv.is_some());
}
