use chess::board::evaluator::{MAX_EVAL_VALUE, MIN_EVAL_VALUE};
use chess::history::history::History;
use chess::search::core::advanced_search::{find_all_valid_moves, SEARCH_ABORTED};
use chess::search::SearchContext;
use chess::search::test_support::{
    evaluate_root_for_bounds,
    has_search_aborted,
    set_test_sleep_after_pv_ms,
    SearchHeuristics,
};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn has_search_aborted_detects_abort() {
    let results = vec![
        ((0, 0), (0, 1), None, 10, 5),
        ((0, 1), (0, 2), None, 20, SEARCH_ABORTED),
    ];
    assert!(has_search_aborted(&results));
}

#[test]
fn has_search_aborted_false_when_clean() {
    let results = vec![
        ((0, 0), (0, 1), None, 10, 5),
        ((0, 1), (0, 2), None, 20, 15),
    ];
    assert!(!has_search_aborted(&results));
}

struct TestSleepGuard;

impl TestSleepGuard {
    fn new(ms: u64) -> Self {
        set_test_sleep_after_pv_ms(ms);
        Self
    }
}

impl Drop for TestSleepGuard {
    fn drop(&mut self) {
        set_test_sleep_after_pv_ms(0);
    }
}

#[test]
fn evaluate_root_for_bounds_parallel_aborts_on_timeout() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = SearchContext::new();
    ctx.set_deterministic(true);
    ctx.set_parallel_search(true);
    ctx.set_time_budget_ms(50);

    let root_moves = find_all_valid_moves(&mut gs);
    assert!(root_moves.len() >= 4);

    let _guard = TestSleepGuard::new(100);
    let mut heuristics = SearchHeuristics::new(128);
    let (_bm, best_adj, best_raw) = evaluate_root_for_bounds(
        &ctx,
        gs.active_color(),
        &root_moves,
        6,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        ctx.tt(),
        &mut gs,
        &history,
        &mut heuristics,
        None,
    );

    assert_eq!(best_raw, SEARCH_ABORTED);
    assert_eq!(best_adj, SEARCH_ABORTED);
}
