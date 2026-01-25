use chess::search::context::SearchContext;
use chess::search::integration::uci_feedback::{emit_info, set_info_callback};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn uci_feedback_emits_info_via_callback() {
    let ctx = SearchContext::new();
    let called = Arc::new(AtomicUsize::new(0));
    let called_clone = Arc::clone(&called);
    let cb = Arc::new(move |_mv, _score, _depth, _pv, _hash| {
        called_clone.fetch_add(1, Ordering::SeqCst);
    });
    set_info_callback(&ctx, Some(cb));

    emit_info(&ctx, (0, 0), (0, 1), None, 10, 1, Vec::new(), 0);
    assert_eq!(called.load(Ordering::SeqCst), 1);
}
