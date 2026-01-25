use chess::generator::move_generator::generate_move_as_san;
use chess::history::history::History;
use chess::search::{SearchContext, SearchMode};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn generate_move_as_san_returns_some_move() {
    let gs = reset_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        .expect("Invalid FEN");
    let history = History::new();
    let ctx = SearchContext::new();
    ctx.set_deterministic(true);
    ctx.set_order_book_enabled(false);

    let san = generate_move_as_san(&ctx, SearchMode::Test, &gs, &history, 1, 0, 1000);
    assert!(san.is_some());
}
