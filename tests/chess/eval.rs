use chess::board::evaluator::evaluate_position;
use chess::state::fen::reader::reset_from_fen;

/// tests for the evaluation function
#[test]
fn eval_single_positions() {
    // You can add or change these cases freely. Scores are in centipawns.
    // Tolerance accounts for minor tweaks in the evaluator.

    // Start position ~ equal
    assert_eval(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        0,
        30,
    );

    // Known drawn rook+pawn vs rook motif (approx 0). Using a common test FEN.
    assert_eval(
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        0,
        60,
    );

    // Slight white edge in a typical opening structure
    assert_eval(
        "r1bqkbnr/1ppp1ppp/p1n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4",
        30,
        60,
    );
}

/// Helper to assert evaluation for a single FEN with a target score and tolerance (centipawns).
fn assert_eval(fen: &str, expected_cp: i32, tol_cp: i32) {
    let gs = reset_from_fen(fen).expect("valid FEN");
    let score = evaluate_position(gs.board(), gs.active_color());
    assert!(
        (score - expected_cp).abs() <= tol_cp,
        "FEN: {}\nexpected: {} cp ±{}; got: {} cp",
        fen,
        expected_cp,
        tol_cp,
        score
    );
}
