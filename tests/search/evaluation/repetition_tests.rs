use chess::history::history::History;
use chess::search::test_support::apply_repetition_avoidance_bias;
use chess::state::fen::reader::reset_from_fen;
use chess::state::test_support::game_state_to_fen_string;
use chess::piece::pieces::Color;

#[test]
fn repetition_bias_clamps_scores_for_repeated_positions() {
    let gs = reset_from_fen("8/8/8/8/8/8/6k1/6K1 w - - 0 1").expect("Invalid FEN");
    let mut history = History::new();

    let mut after = gs;
    let _u = after.make_move_fast((0, 6), (1, 6), None);
    let key = after.zobrist_key();
    let fen_after = game_state_to_fen_string(after);
    history.set_starting_position(fen_after.clone(), key);
    history.add_move("Kg2".to_string(), ((0, 6), (1, 6)), fen_after, key);

    let adjusted = apply_repetition_avoidance_bias(
        100,
        &gs,
        &history,
        Color::White,
        (0, 6),
        (1, 6),
        None,
        50,
    );
    assert!(adjusted.abs() <= 10);
}

#[test]
fn repetition_bias_resets_when_raw_score_is_zero() {
    let gs = reset_from_fen("8/8/8/8/8/8/6k1/6K1 w - - 0 1").expect("Invalid FEN");
    let mut history = History::new();

    let mut after = gs;
    let _u = after.make_move_fast((0, 6), (1, 6), None);
    let key = after.zobrist_key();
    let fen_after = game_state_to_fen_string(after);
    history.set_starting_position(fen_after.clone(), key);
    history.add_move("Kg2".to_string(), ((0, 6), (1, 6)), fen_after, key);

    let adjusted = apply_repetition_avoidance_bias(
        5,
        &gs,
        &history,
        Color::White,
        (0, 6),
        (1, 6),
        None,
        0,
    );
    assert_eq!(adjusted, 0);
}
