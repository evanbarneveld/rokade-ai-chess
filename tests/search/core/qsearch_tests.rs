use chess::board::evaluator::{evaluate_position, MAX_EVAL_VALUE, MIN_EVAL_VALUE, MATE_VALUE};
use chess::search::core::advanced_search::SEARCH_ABORTED;
use chess::search::context::SearchContext;
use chess::search::test_support::{qsearch, qsearch_with_quiescence, RepetitionStack};
use chess::state::fen::reader::reset_from_fen;
use std::time::Duration;

#[test]
fn qsearch_considers_en_passant_capture() {
    let fen = "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let stand_pat = evaluate_position(gs.board(), gs.active_color());
    let mut rep_stack = RepetitionStack::new();

    let score = qsearch(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
    );

    assert!(
        score > stand_pat,
        "expected qsearch to prefer en passant capture over stand-pat eval"
    );
}

#[test]
fn qsearch_returns_mate_for_checkmated_black() {
    let fen = "7k/6Q1/6K1/8/8/8/8/8 b - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let mut rep_stack = RepetitionStack::new();

    let score = qsearch(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
    );

    assert!(
        score >= MATE_VALUE - 200,
        "expected mate score for Black to move, got {}",
        score
    );
}

#[test]
fn qsearch_returns_stand_pat_for_stalemate() {
    let fen = "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let stand_pat = evaluate_position(gs.board(), gs.active_color());
    let mut rep_stack = RepetitionStack::new();

    let score = qsearch(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
    );

    assert_eq!(score, stand_pat);
}

#[test]
fn qsearch_returns_zero_for_repetition() {
    let fen = "8/8/8/8/8/8/6k1/6K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let mut rep_stack = RepetitionStack::new();
    let key = gs.zobrist_key();
    rep_stack.push(key);
    rep_stack.push(key);

    let score = qsearch(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
    );

    assert_eq!(score, 0);
}

#[test]
fn qsearch_respects_fifty_move_rule() {
    let fen = "8/8/8/8/8/8/6k1/6K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    gs.set_half_move_clock(100);
    let ctx = SearchContext::new();
    let mut rep_stack = RepetitionStack::new();

    let score = qsearch(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
    );

    assert_eq!(score, 0);
}

#[test]
fn qsearch_returns_search_aborted_when_time_is_up() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    ctx.set_time_budget_ms(1);
    std::thread::sleep(Duration::from_millis(10));
    let mut rep_stack = RepetitionStack::new();

    let score = qsearch(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
    );

    assert_eq!(score, SEARCH_ABORTED);
}

#[test]
fn qsearch_maximizing_stand_pat_beta_cutoff_returns_stand_pat() {
    let fen = "6k1/8/8/8/8/8/6Q1/6K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let stand_pat = qsearch_with_quiescence(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut RepetitionStack::new(),
        false,
    );
    let mut rep_stack = RepetitionStack::new();

    let score = qsearch(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        stand_pat - 1,
        &mut rep_stack,
    );

    assert_eq!(score, stand_pat);
}

#[test]
fn qsearch_minimizing_stand_pat_alpha_cutoff_returns_stand_pat() {
    let fen = "7k/8/8/8/8/8/6Q1/6K1 b - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let stand_pat = qsearch_with_quiescence(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut RepetitionStack::new(),
        false,
    );
    let mut rep_stack = RepetitionStack::new();

    let score = qsearch(
        &ctx,
        &mut gs,
        stand_pat + 1,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
    );

    assert_eq!(score, stand_pat);
}

#[test]
fn qsearch_maximizing_delta_pruning_returns_stand_pat() {
    let fen = "6k1/8/8/8/8/8/6Q1/6K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let stand_pat = qsearch_with_quiescence(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut RepetitionStack::new(),
        false,
    );
    let mut rep_stack = RepetitionStack::new();
    let alpha = (stand_pat + 2000).min(MAX_EVAL_VALUE - 2);

    let score = qsearch(
        &ctx,
        &mut gs,
        alpha,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
    );

    assert_eq!(score, stand_pat);
}

#[test]
fn qsearch_minimizing_delta_pruning_returns_stand_pat() {
    let fen = "7k/8/8/8/8/8/6Q1/6K1 b - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let stand_pat = qsearch_with_quiescence(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut RepetitionStack::new(),
        false,
    );
    let mut rep_stack = RepetitionStack::new();
    let beta = stand_pat.saturating_sub(2000);

    let score = qsearch(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        beta,
        &mut rep_stack,
    );

    assert_eq!(score, stand_pat);
}

#[test]
fn qsearch_in_check_with_evasion_is_not_mate() {
    let fen = "4k3/8/8/8/8/8/4r3/4K3 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let mut rep_stack = RepetitionStack::new();

    let score = qsearch(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
    );

    assert!(
        score > -MATE_VALUE + 1000,
        "expected non-mate score for check position, got {}",
        score
    );
}

#[test]
fn qsearch_filters_bad_capture_by_see() {
    let fen = "6k1/8/2p5/3p4/8/3Q4/8/6K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let stand_pat = evaluate_position(gs.board(), gs.active_color());
    let mut rep_stack = RepetitionStack::new();

    let score = qsearch(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
    );

    assert_eq!(score, stand_pat);
}

#[test]
fn qsearch_adds_quiet_passed_pawn_push() {
    let fen = "7k/8/4P3/4K3/8/8/8/8 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let stand_pat = evaluate_position(gs.board(), gs.active_color());
    let mut rep_stack = RepetitionStack::new();

    let score = qsearch(
        &ctx,
        &mut gs,
        stand_pat + 1,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
    );

    assert!(score > stand_pat);
}

#[test]
fn qsearch_ignores_single_top_repetition() {
    let fen = "7k/8/8/8/8/8/6Q1/6K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let mut rep_stack = RepetitionStack::new();
    let key = gs.zobrist_key();
    rep_stack.push(key);

    let score = qsearch(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
    );

    assert!(score > 0);
}

#[test]
fn qsearch_with_quiescence_disabled_returns_static_eval() {
    let fen = "6k1/8/8/8/8/8/6Q1/6K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let stand_pat = evaluate_position(gs.board(), gs.active_color());
    let mut rep_stack = RepetitionStack::new();

    let score = qsearch_with_quiescence(
        &ctx,
        &mut gs,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        &mut rep_stack,
        false,
    );

    assert_eq!(score, stand_pat);
}
