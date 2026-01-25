use chess::history::history::History;

#[test]
fn history_tracks_repetition_counts_and_undo() {
    let mut history = History::new();
    let fen = "8/8/8/8/8/8/6k1/6K1 w - - 0 1".to_string();
    let key = 42u64;
    history.set_starting_position(fen.clone(), key);

    history.add_move("e2e4".to_string(), ((1, 4), (3, 4)), fen.clone(), key);
    assert_eq!(history.len(), 1);
    assert_eq!(history.zobrist_repetition_count(key), 2);
    assert_eq!(history.current_repetition_count(), 2);

    let undone = history.undo_move().expect("expected undo");
    assert_eq!(undone.0, "e2e4");
    assert_eq!(history.zobrist_repetition_count(key), 1);
}
