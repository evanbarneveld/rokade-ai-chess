use chess::board::evaluator::{MAX_EVAL_VALUE, MIN_EVAL_VALUE};
use chess::history::history::History;
use chess::search::context::SearchContext;
use chess::search::core::advanced_search::find_all_valid_moves;
use chess::search::test_support::{
    aspiration_bounds_for_depth,
    evaluate_root_for_bounds,
    probe_with_aspiration,
    SearchHeuristics,
};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn aspiration_bounds_shallow_depth_returns_full_window() {
    let (a, b) = aspiration_bounds_for_depth(3, 120, 25);
    assert_eq!(a, MIN_EVAL_VALUE + 1);
    assert_eq!(b, MAX_EVAL_VALUE - 1);
}

#[test]
fn aspiration_bounds_clamps_near_edges() {
    let (a_high, b_high) = aspiration_bounds_for_depth(4, MAX_EVAL_VALUE - 5, 30);
    assert_eq!(a_high, MAX_EVAL_VALUE - 35);
    assert_eq!(b_high, MAX_EVAL_VALUE - 1);

    let (a_low, b_low) = aspiration_bounds_for_depth(4, MIN_EVAL_VALUE + 5, 30);
    assert_eq!(a_low, MIN_EVAL_VALUE + 1);
    assert_eq!(b_low, MIN_EVAL_VALUE + 35);
}

#[test]
fn probe_with_aspiration_expands_window_after_fail_low() {
    let fen = "4k3/8/8/8/8/8/8/R3K3 w - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    ctx.set_deterministic(true);

    let mut gs_moves = gs;
    let root_moves = find_all_valid_moves(&mut gs_moves);
    assert!(!root_moves.is_empty(), "expected root moves");

    let tt = ctx.tt();
    let history = History::new();
    let mut gs_full = gs;
    let mut heur_full = SearchHeuristics::new(64);
    let (_mv_full, _adj_full, raw_full) = evaluate_root_for_bounds(
        &ctx,
        gs.active_color(),
        &root_moves,
        4,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        tt,
        &mut gs_full,
        &history,
        &mut heur_full,
        None,
    );

    let mut window = 10;
    let last_score = raw_full + 10000;
    let mut gs_probe = gs;
    let mut heur_probe = SearchHeuristics::new(64);
    let (_mv, _adj, raw) = probe_with_aspiration(
        &ctx,
        gs.active_color(),
        &root_moves,
        4,
        last_score,
        &mut window,
        tt,
        &mut gs_probe,
        &history,
        &mut heur_probe,
        None,
    );

    assert_eq!(raw, raw_full);
    assert!(window > 10, "expected window to expand after fail-low");
}

#[test]
fn probe_with_aspiration_succeeds_inside_bounds() {
    let fen = "4k3/8/8/8/8/8/8/R3K3 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    ctx.set_deterministic(true);

    let mut gs_moves = gs;
    let root_moves = find_all_valid_moves(&mut gs_moves);
    assert!(!root_moves.is_empty(), "expected root moves");

    let tt = ctx.tt();
    let history = History::new();
    let mut window = 20000;
    let mut heur = SearchHeuristics::new(64);
    let (_mv, _adj, raw) = probe_with_aspiration(
        &ctx,
        gs.active_color(),
        &root_moves,
        4,
        0,
        &mut window,
        tt,
        &mut gs,
        &history,
        &mut heur,
        None,
    );

    assert_eq!(window, 20000);
    assert!(raw >= -20000 && raw <= 20000, "expected raw score inside window");
}
