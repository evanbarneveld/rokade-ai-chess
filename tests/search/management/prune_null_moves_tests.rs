use chess::board::evaluator::{MAX_EVAL_VALUE, MIN_EVAL_VALUE};
use chess::search::context::SearchContext;
use chess::search::test_support::{prune_null_moves, RepetitionStack, SearchHeuristics};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn prune_null_moves_skips_when_depth_is_too_low() {
    let fen = "6k1/8/8/8/8/8/6Q1/6K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let tt = ctx.tt();
    let mut heuristics = SearchHeuristics::new(64);
    let mut rep_stack = RepetitionStack::new();

    let result = prune_null_moves(
        &ctx,
        &mut heuristics,
        &mut gs,
        5,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        0,
        tt,
        &mut rep_stack,
        true,
        None,
    );

    assert!(result.is_none());
}

#[test]
fn prune_null_moves_skips_when_in_check() {
    let fen = "6k1/6q1/8/8/8/8/8/6K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let tt = ctx.tt();
    let mut heuristics = SearchHeuristics::new(64);
    let mut rep_stack = RepetitionStack::new();

    let result = prune_null_moves(
        &ctx,
        &mut heuristics,
        &mut gs,
        6,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        0,
        tt,
        &mut rep_stack,
        true,
        None,
    );

    assert!(result.is_none());
}

#[test]
fn prune_null_moves_skips_without_non_pawn_material() {
    let fen = "8/8/8/8/8/8/4P3/4K2k w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let tt = ctx.tt();
    let mut heuristics = SearchHeuristics::new(64);
    let mut rep_stack = RepetitionStack::new();

    let result = prune_null_moves(
        &ctx,
        &mut heuristics,
        &mut gs,
        6,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        0,
        tt,
        &mut rep_stack,
        true,
        None,
    );

    assert!(result.is_none());
}

#[test]
fn prune_null_moves_returns_beta_cutoff_for_white() {
    let fen = "6k1/8/8/8/8/8/6Q1/6K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let tt = ctx.tt();
    let mut heuristics = SearchHeuristics::new(64);
    let mut rep_stack = RepetitionStack::new();

    let alpha = MIN_EVAL_VALUE + 1;
    let beta = MIN_EVAL_VALUE + 200;
    let before = (
        gs.active_color(),
        gs.half_move_clock(),
        gs.en_passant_target(),
        gs.zobrist_key(),
    );

    let result = prune_null_moves(
        &ctx,
        &mut heuristics,
        &mut gs,
        8,
        alpha,
        beta,
        0,
        tt,
        &mut rep_stack,
        true,
        None,
    );

    assert_eq!(result, Some(beta));
    assert_eq!(
        before,
        (
            gs.active_color(),
            gs.half_move_clock(),
            gs.en_passant_target(),
            gs.zobrist_key()
        )
    );
}

#[test]
fn prune_null_moves_returns_alpha_cutoff_for_black() {
    let fen = "6k1/6q1/8/8/8/8/8/6K1 b - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let tt = ctx.tt();
    let mut heuristics = SearchHeuristics::new(64);
    let mut rep_stack = RepetitionStack::new();

    let alpha = 10000;
    let beta = alpha + 1;
    let result = prune_null_moves(
        &ctx,
        &mut heuristics,
        &mut gs,
        8,
        alpha,
        beta,
        0,
        tt,
        &mut rep_stack,
        true,
        None,
    );

    assert_eq!(result, Some(alpha));
}

#[test]
fn prune_null_moves_skips_when_static_eval_not_strong() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let tt = ctx.tt();
    let mut heuristics = SearchHeuristics::new(64);
    let mut rep_stack = RepetitionStack::new();

    let alpha = MIN_EVAL_VALUE + 1;
    let beta = 150;

    let result = prune_null_moves(
        &ctx,
        &mut heuristics,
        &mut gs,
        6,
        alpha,
        beta,
        0,
        tt,
        &mut rep_stack,
        true,
        None,
    );

    assert!(result.is_none());
}

#[test]
fn prune_null_moves_skips_in_endgame_at_shallow_depth() {
    let fen = "8/8/8/8/8/8/7B/6Kk w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let tt = ctx.tt();
    let mut heuristics = SearchHeuristics::new(64);
    let mut rep_stack = RepetitionStack::new();

    let result = prune_null_moves(
        &ctx,
        &mut heuristics,
        &mut gs,
        6,
        MIN_EVAL_VALUE + 1,
        0,
        0,
        tt,
        &mut rep_stack,
        true,
        None,
    );

    assert!(result.is_none());
}
