use chess::Chess;
use chess::search::advanced_search::{debug_rank_root_moves};
use chess::search::set_deterministic;
use serial_test::serial;

#[test]
#[serial]
fn test_prefer_draw_over_loss() {
    let mut game = Chess::new();
    // 7k/Q7/8/8/8/8/r5r1/7K w - - 0 1
    // Initial position: Queen on a7, King on h8.
    game.set_starting_fen("7k/Q7/8/8/8/8/r5r1/7K w - - 0 1").expect("bad fen");
    set_deterministic(true);

    println!("Initial FEN: {}", game.to_fen());
    
    // Manual moves:
    // a7-a8+, h8-h7, a8-a7+, h7-h8
    let moves = [
        "Qa8+", "Kh7", "Qa7+", "Kh8", "Qa8+", "Kh7"
    ];
    
    for m in moves {
        assert!(game.move_piece_san(m), "Move {} failed", m);
    }
    
    // Now White is in Pos where Qa7+ leads to 3rd repetition of start position.
    
    let history = game.get_history().clone();
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 4, 1000);
    for r in ranks {
        println!("Rank: {:?}", r);
    }

    let search_res = chess::search::find_best_move_with_mode(
        game.get_search_mode(),
        game.get_game_state(),
        &history,
        4,
        1000
    );
    
    let ((f_from, f_to), (t_from, t_to), _promo, score, _depth) = search_res.expect("No move found");
    println!("Selected move: {:?}->{:?}, score: {}", (f_from, f_to), (t_from, t_to), score);

    // The engine should prefer a draw (score >= -10) over any losing move.
    // In this position, non-draw moves like Qf3 or Qc6 are seen as losing (-100 or worse).
    assert!(score >= -10, "Engine should prefer draw (score >= -10) but chose move with score {}", score);
}
