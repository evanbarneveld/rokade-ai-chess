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
    assert!(g.move_piece_san("e2e4"));
    assert_eq!(g.to_fen(), "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1".to_string());
}

#[test]
fn test_pawn_capture() {
    let mut game = Chess::new();
    game.set_starting_fen("rnbqkbnr/p1pppppp/P7/8/8/1p6/1PPPPPPP/RNBQKBNR w KQkq - 0 4");
    println!("{}", game.board());
    assert!( game.move_piece_san("cxb3"));
    println!("{}", game.board());
    assert_eq!(game.to_fen(), "rnbqkbnr/p1pppppp/P7/8/8/1P6/1P1PPPPP/RNBQKBNR b KQkq - 0 4");
}

#[test]
fn test_en_passant_pawn_capture() {
    let mut game = Chess::new();
    game.set_starting_fen("rnbqkbnr/1ppppp1p/8/p3P3/6pP/1P6/P1PP1PP1/RNBQKBNR b KQkq h3 0 4");
    println!("{}", game.board());
    assert!( game.move_piece_san("g4xh3"));
    println!("{}", game.board());
    assert_eq!(game.to_fen(), "rnbqkbnr/1ppppp1p/8/p3P3/8/1P5p/P1PP1PP1/RNBQKBNR w KQkq - 0 5");
}

#[test]
fn test_en_passant_capture_black() {
    let mut game = Chess::new();
    game.set_starting_fen("rn2k1nr/ppp2ppp/8/2bqpb2/2Pp4/1K3P2/PP1PP1PP/RNBQ1BNR b kq c3 0 7");
    println!("{}", game.board());
    assert!(game.move_piece_san("dxc3"));
}

#[test]
fn test_pawn_move_over_piece() {
    let mut game = Chess::new();
    game.set_starting_fen("rnbqkbn1/ppppppp1/3r4/7p/7P/3R4/PPPPPPP1/RNBQKBN1 w KQkq h6 4 4");
    println!("{}", game.board());
    assert!( !game.move_piece_san("d2d4"));
    assert!(game.move_piece_san("a2a4"));
    assert!( !game.move_piece_san("d7d5"));
}

#[test]
fn test_castling() {
    let mut game = Chess::new();
    game.set_starting_fen("r1bqkbnr/1ppp1ppp/p1n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4");
    println!("{}", game.board());
    assert!(game.move_piece_san("O-O"));
    println!("{}", game.board());
    assert_eq!(game.to_fen(), "r1bqkbnr/1ppp1ppp/p1n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 1 4");
}

#[test]
fn test_castling_black() {
    let mut game = Chess::new();
    game.set_starting_fen("r3kbnr/1pp2ppp/p1n1b3/3pp1q1/P1B1P3/5N1P/1PPP1PP1/RNBQ1RK1 b kq - 0 7");
    println!("{}", game.board());
    assert!(game.move_piece_san("O-O-O"));
    println!("{}", game.board());
    assert_eq!(game.to_fen(), "2kr1bnr/1pp2ppp/p1n1b3/3pp1q1/P1B1P3/5N1P/1PPP1PP1/RNBQ1RK1 w - - 1 8");
}

//#[test]
fn test_black_promotion_error() {
    let fen = "r1bqkbnr/ppp2ppp/2n5/1B1pp3/8/1P2P3/PBPP1PPP/RN1QK1NR b KQkq - 3 4";
    let mv = "Ne7";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    println!("Fen: {}", fen);
    println!("Move: {}", mv);
    println!("{}", game.board());
    assert!(game.move_piece_san(mv));
}
