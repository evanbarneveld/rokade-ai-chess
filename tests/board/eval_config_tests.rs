//! Tests for evaluation configuration flags.
//!
//! Note: Most eval_config tests are unit tests in src/board/eval_config.rs.
//! Integration tests here are limited because the global flags are shared
//! across parallel test threads.

use chess::board::eval_config::{set_eval_flags, EvalFlags};
use chess::board::evaluator::evaluate_position;
use chess::piece::pieces::Color;
use chess::state::game_state::GameState;

#[test]
fn test_disabling_all_flags_still_evaluates() {
    // This test is safe because it doesn't depend on initial state
    let gs = GameState::default();
    let board = gs.board();

    // Disable all flags
    set_eval_flags(EvalFlags::empty());
    let score_none = evaluate_position(board, Color::White);

    // Enable all flags
    set_eval_flags(EvalFlags::ALL);
    let score_all = evaluate_position(board, Color::White);

    // With no heuristics, score should be different (smaller magnitude)
    // Just verify both evaluate without crashing
    assert!(
        score_none.abs() <= score_all.abs() + 50,
        "Minimal eval should not exceed full eval by much"
    );
}
