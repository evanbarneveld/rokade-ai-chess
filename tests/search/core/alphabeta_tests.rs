use chess::board::evaluator::{MAX_EVAL_VALUE, MIN_EVAL_VALUE, MATE_VALUE};
use chess::search::context::SearchContext;
use chess::search::core::advanced_search::SEARCH_ABORTED;
use chess::search::core::alphabeta::alphabeta;
use chess::search::core::advanced_search::find_all_valid_moves;
use chess::search::test_support::{calculate_lmr_reduction, check_extension_budget, is_singular_extension, should_check_extend, should_late_move_prune, RepetitionStack, SearchHeuristics};
use chess::state::fen::reader::reset_from_fen;
use std::time::Duration;
use chess::piece::pieces::{Color, opposite_color};

fn run_alphabeta(fen: &str, depth: usize) -> i32 {
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let mut heuristics = SearchHeuristics::new(128);
    let tt = ctx.tt();
    let mut rep_stack = RepetitionStack::new();
    alphabeta(
        &ctx,
        &mut heuristics,
        &mut gs,
        depth,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        0,
        tt,
        &mut rep_stack,
        true,
        None,
        check_extension_budget(),
    )
}

fn run_alphabeta_with_nodes(fen: &str, depth: usize, alpha: i32, beta: i32, ply: i32) -> (i32, u64) {
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let mut heuristics = SearchHeuristics::new(128);
    let tt = ctx.tt();
    let mut rep_stack = RepetitionStack::new();
    ctx.reset_search_telemetry();

    let score = alphabeta(
        &ctx,
        &mut heuristics,
        &mut gs,
        depth,
        alpha,
        beta,
        ply,
        tt,
        &mut rep_stack,
        true,
        None,
        check_extension_budget(),
    );

    (score, ctx.get_nodes())
}

fn lmr_reduction_for_move(
    fen: &str,
    from: (usize, usize),
    to: (usize, usize),
    depth: usize,
    move_index: i32,
    prev_move: Option<((usize, usize), (usize, usize))>,
    heuristics: &SearchHeuristics,
) -> usize {
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let pre_board = *gs.board();
    let pre_ep = gs.en_passant_target();
    let moved_pre = pre_board.get(from.0, from.1);
    let to_move = gs.active_color();
    let u = gs.make_move_fast(from, to, None);
    let gives_check = gs.mutable_board().is_side_in_check(opposite_color(to_move));
    let is_ep = pre_ep.is_some()
        && pre_ep == Some(to)
        && moved_pre.map(|p| p.get_type()) == Some(chess::piece::pieces::PieceType::Pawn)
        && pre_board.get(to.0, to.1).is_none();
    let is_capture = pre_board.get(to.0, to.1).is_some() || is_ep;
    let quiet = !is_capture;
    let allow_reduce = !gives_check;
    let moved_pt = gs.board().get(to.0, to.1).map(|p| p.get_type());
    let captured = if is_ep {
        pre_board.get(from.0, to.1)
    } else {
        pre_board.get(to.0, to.1)
    };
    let phase = gs.board().game_phase_light();
    let child_depth = depth.saturating_sub(1);

    let reduction = calculate_lmr_reduction(
        child_depth,
        move_index,
        quiet,
        gives_check,
        allow_reduce,
        to_move,
        from,
        to,
        moved_pt,
        gs.board(),
        phase,
        captured,
        heuristics,
        0,
        prev_move,
    );

    gs.unmake_move_fast(u);
    reduction
}

#[test]
fn alphabeta_returns_mate_score_for_black_checkmated() {
    let fen = "7k/6Q1/7K/8/8/8/8/8 b - - 0 1";
    let score = run_alphabeta(fen, 1);
    assert!(score >= MATE_VALUE - 5, "expected mate score for Black to move, got {}", score);
}

#[test]
fn alphabeta_returns_mate_score_for_white_checkmated() {
    let fen = "8/8/8/8/8/6k1/7q/7K w - - 0 1";
    let score = run_alphabeta(fen, 1);
    assert!(score <= -MATE_VALUE + 5, "expected mate score for White to move, got {}", score);
}

#[test]
fn alphabeta_returns_zero_for_stalemate() {
    let fen = "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1";
    let score = run_alphabeta(fen, 1);
    assert_eq!(score, 0);
}

#[test]
fn alphabeta_returns_zero_for_repetition() {
    let fen = "8/8/8/8/8/8/6k1/6K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let mut heuristics = SearchHeuristics::new(128);
    let tt = ctx.tt();
    let mut rep_stack = RepetitionStack::new();
    let key = gs.zobrist_key();
    rep_stack.push(key);

    let score = alphabeta(
        &ctx,
        &mut heuristics,
        &mut gs,
        1,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        0,
        tt,
        &mut rep_stack,
        true,
        None,
        check_extension_budget(),
    );
    assert_eq!(score, 0);
}

#[test]
fn alphabeta_returns_zero_for_fifty_move_rule() {
    let fen = "8/8/8/8/8/8/6k1/6K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    gs.set_half_move_clock(100);
    let ctx = SearchContext::new();
    let mut heuristics = SearchHeuristics::new(128);
    let tt = ctx.tt();
    let mut rep_stack = RepetitionStack::new();

    let score = alphabeta(
        &ctx,
        &mut heuristics,
        &mut gs,
        1,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        0,
        tt,
        &mut rep_stack,
        true,
        None,
        check_extension_budget(),
    );
    assert_eq!(score, 0);
}

#[test]
fn alphabeta_returns_search_aborted_when_time_is_up() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    ctx.set_time_budget_ms(1);
    std::thread::sleep(Duration::from_millis(10));

    let mut heuristics = SearchHeuristics::new(128);
    let tt = ctx.tt();
    let mut rep_stack = RepetitionStack::new();

    let score = alphabeta(
        &ctx,
        &mut heuristics,
        &mut gs,
        2,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        0,
        tt,
        &mut rep_stack,
        true,
        None,
        check_extension_budget(),
    );
    assert_eq!(score, SEARCH_ABORTED);
}

#[test]
fn lmr_reduction_skips_in_endgame() {
    let fen = "8/8/8/8/8/8/4P3/4K2k w - - 0 1";
    let heuristics = SearchHeuristics::new(8);
    let reduction = lmr_reduction_for_move(fen, (1, 4), (2, 4), 8, 6, None, &heuristics);
    assert_eq!(reduction, 0);
}

#[test]
fn lmr_reduction_increases_with_depth_and_index() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let heuristics = SearchHeuristics::new(16);
    let r1 = lmr_reduction_for_move(fen, (0, 6), (2, 5), 8, 6, None, &heuristics);
    let r2 = lmr_reduction_for_move(fen, (0, 6), (2, 5), 12, 12, None, &heuristics);
    assert!(r1 > 0);
    assert!(r2 > r1);
}

#[test]
fn lmr_reduction_respects_killer_and_counter_moves() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let from = (0, 6);
    let to = (2, 5);
    let prev_from = (1, 4);
    let prev_to = (3, 4);

    let base_heuristics = SearchHeuristics::new(16);
    let base = lmr_reduction_for_move(fen, from, to, 12, 12, Some((prev_from, prev_to)), &base_heuristics);

    let mut killer_heuristics = SearchHeuristics::new(16);
    killer_heuristics.add_killer(0, from, to);
    let killer = lmr_reduction_for_move(fen, from, to, 12, 12, Some((prev_from, prev_to)), &killer_heuristics);
    assert!(killer <= base);
    if base > 1 {
        assert!(killer < base);
    }

    let mut counter_heuristics = SearchHeuristics::new(16);
    counter_heuristics.set_counter_move(Color::White, prev_from, prev_to, from, to);
    let counter = lmr_reduction_for_move(fen, from, to, 12, 12, Some((prev_from, prev_to)), &counter_heuristics);
    assert!(counter <= base);
}

#[test]
fn razoring_reduces_nodes_on_fail_low() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let (_score_razor, nodes_razor) = run_alphabeta_with_nodes(fen, 1, 10_000, 10_001, 4);
    let (_score_full, nodes_full) = run_alphabeta_with_nodes(
        fen,
        1,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        4,
    );

    assert!(nodes_full > 1);
    assert!(nodes_razor < nodes_full);
}

#[test]
fn reverse_futility_pruning_reduces_nodes_on_fail_high() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let (_score_rfp, nodes_rfp) = run_alphabeta_with_nodes(
        fen,
        2,
        -401,
        -400,
        2,
    );
    let (_score_full, nodes_full) = run_alphabeta_with_nodes(
        fen,
        2,
        MIN_EVAL_VALUE + 1,
        MAX_EVAL_VALUE - 1,
        2,
    );

    assert!(nodes_full > 1);
    assert!(nodes_rfp < nodes_full);
}

#[test]
fn lmp_prunes_late_quiet_moves() {
    assert!(should_late_move_prune(1, 12, true, false, false, false));
    assert!(should_late_move_prune(2, 16, true, false, false, false));
}

#[test]
fn lmp_skips_checks_pv_or_in_check() {
    assert!(!should_late_move_prune(2, 20, true, true, false, false));
    assert!(!should_late_move_prune(2, 20, true, false, true, false));
    assert!(!should_late_move_prune(2, 20, true, false, false, true));
    assert!(!should_late_move_prune(3, 20, true, false, false, false));
}

#[test]
fn check_extension_triggers_for_shallow_checks() {
    assert!(should_check_extend(1, true, false));
    assert!(should_check_extend(2, false, true));
    assert!(should_check_extend(3, true, true));
}

#[test]
fn check_extension_skips_deep_or_non_checks() {
    assert!(!should_check_extend(0, true, false));
    assert!(!should_check_extend(3, false, false));
    assert!(!should_check_extend(4, true, false));
}

#[test]
fn singular_extension_true_for_large_score_gap() {
    let fen = "8/8/8/8/8/8/4K3/6k1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let tt = ctx.tt();
    let mut rep_stack = RepetitionStack::new();
    let moves = find_all_valid_moves(&mut gs);
    assert!(moves.len() > 1);
    let best_move = (moves[0].0, moves[0].1);
    let to_move = gs.active_color();

    let singular = is_singular_extension(
        &ctx,
        &mut gs,
        6,
        2,
        tt,
        &mut rep_stack,
        to_move,
        best_move,
        10_000,
        &moves,
        None,
    );

    assert!(singular);
}

#[test]
fn singular_extension_false_for_nearby_scores() {
    let fen = "8/8/8/8/8/8/4K3/6k1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let tt = ctx.tt();
    let mut rep_stack = RepetitionStack::new();
    let moves = find_all_valid_moves(&mut gs);
    assert!(moves.len() > 1);
    let best_move = (moves[0].0, moves[0].1);
    let to_move = gs.active_color();

    let singular = is_singular_extension(
        &ctx,
        &mut gs,
        6,
        2,
        tt,
        &mut rep_stack,
        to_move,
        best_move,
        0,
        &moves,
        None,
    );

    assert!(!singular);
}
