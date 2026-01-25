use chess::generator::move_generator::generate_move_as_san;
use chess::search::{SearchContext, SearchMode};
use chess::state::fen::reader::reset_from_fen;
use chess::history::history::History;
use chess::search::integration::playing_strength::PLAYING_STRENGTH_MAX;

// Guard: from the initial position at shallow depth, the engine should not choose a quiet queen move
// This is a regression test against early queen wandering.
#[test]
fn test_no_quiet_queen_from_start() {
    let ctx = deterministic_ctx();
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let history = History::new();
    let depth = 4usize;
    let san = generate_move_as_san(&ctx, SearchMode::Normal, &gs, &history, depth, 500, PLAYING_STRENGTH_MAX)
        .expect("expected a move");
    // Reject quiet queen moves like Qd1, Qe2, Qf3, Qh5 without capture 'x'
    let is_queen_move = san.starts_with('Q');
    let is_capture = san.contains('x');
    assert!(
        !(is_queen_move && !is_capture),
        "Expected not to pick a quiet queen move from the start at depth {}, got {}",
        depth,
        san
    );
}

// Guard: after 1.e4 (Black to move), engine should not choose a quiet queen move at shallow depth
#[test]
fn test_no_quiet_queen_after_e4_black_to_move() {
    let ctx = deterministic_ctx();
    let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let history = History::new();
    let depth = 4usize;
    let san = generate_move_as_san(&ctx, SearchMode::Normal, &gs, &history, depth, 500, PLAYING_STRENGTH_MAX)
        .expect("expected a move");
    let is_queen_move = san.starts_with('Q');
    let is_capture = san.contains('x');
    assert!(
        !(is_queen_move && !is_capture),
        "Expected Black not to pick a quiet queen move after 1.e4 at depth {}, got {}",
        depth,
        san
    );
}

// Guard: in Sicilian structure after 1.e4 c5 (White to move), avoid quiet queen moves at shallow depth
#[test]
fn test_no_quiet_queen_in_sicilian_white_to_move() {
    let ctx = deterministic_ctx();
    // After 1.e4 c5
    let fen = "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq c6 0 2";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let history = History::new();
    let depth = 4usize;
    let san = generate_move_as_san(&ctx, SearchMode::Normal, &gs, &history, depth, 500, PLAYING_STRENGTH_MAX)
        .expect("expected a move");
    let is_queen_move = san.starts_with('Q');
    let is_capture = san.contains('x');
    assert!(
        !(is_queen_move && !is_capture),
        "Expected White not to pick a quiet queen move after 1...c5 at depth {}, got {}",
        depth,
        san
    );
}

fn deterministic_ctx() -> SearchContext {
    let ctx = SearchContext::new();
    ctx.set_deterministic(true);
    ctx
}
