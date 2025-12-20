use chess::Chess;

#[test]
fn initial_fen() {
    let mut g = Chess::new();
    let fen = g.to_fen();
    assert_eq!(fen, Chess::DEFAULT_CHESS_STARTING_FEN);
}

#[test]
fn initial_e2e4_move() {
    let mut g = Chess::new();
    g.move_piece_san("e2e4");
    assert_eq!(g.to_fen(), "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1".to_string());
}