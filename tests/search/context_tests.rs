use chess::search::context::SearchContext;

#[test]
fn search_context_tracks_time_budget_and_nodes() {
    let ctx = SearchContext::new();
    ctx.set_deterministic(true);
    let base_ms = 10;
    ctx.set_time_budget_ms(base_ms);
    assert_eq!(ctx.time_budget_ms(), base_ms + (base_ms / 2));

    ctx.bump_node();
    ctx.bump_node();
    assert_eq!(ctx.get_nodes(), 2);

    ctx.set_order_book_enabled(false);
    assert!(!ctx.get_order_book_enabled());
}
