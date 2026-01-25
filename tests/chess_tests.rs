use chess::Chess;

#[test]
fn chess_defaults_are_initialized() {
    let chess = Chess::new();
    assert_eq!(
        chess.to_fen(),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
    );
}

#[test]
fn chess_clamps_playing_strength() {
    let mut chess = Chess::new();
    chess.set_playing_strength(0);
    assert_eq!(chess.get_playing_strength(), 1);
}

#[test]
fn chess_move_piece_updates_history() {
    let mut chess = Chess::new();
    let ok = chess.move_piece((1, 4), (3, 4), None);
    assert!(ok);
    assert_eq!(chess.get_history().len(), 1);
}
