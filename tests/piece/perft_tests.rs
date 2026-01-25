use chess::piece::perft::{perft_count, perft_count_parallel};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn perft_count_depth_1_matches_expected() {
    let gs = reset_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        .expect("Invalid FEN");
    assert_eq!(perft_count(&gs, 1), 20);
    assert_eq!(perft_count_parallel(&gs, 1), 20);
}
