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
    println!("{}", game.board());
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
#[test]
fn test_ambiguous_move_due_to_pinned_pieces() {
    let fen = "r2qkb1r/ppp2ppp/2n5/1B1npb2/8/2N1PN2/PP1P1PPP/R1BQK2R b KQkq - 5 7";
    let mv = "Ne7";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    println!("Fen: {}", fen);
    println!("Move: {}", mv);
    println!("{}", game.board());
    assert!(game.move_piece_san(mv));
}

#[test]
fn test_ambigous_rook_error() {
    let fen = "2r2bk1/ppp3pp/4rq2/8/2BQ4/2P5/PP3PPP/R3R1K1 b - - 0 21";
    let mv = "Re8";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    println!("Fen: {}", fen);
    println!("Move: {}", mv);
    println!("{}", game.board());
    assert!(game.move_piece_san(mv));
    println!("{}", game.board());
}

#[test]
fn black_en_passant_error() {
    let fen = "3r4/pp5p/5k2/5ppP/2P2N1K/4r1P1/PP5R/8 w - g6 0 29";
    let mv = "hxg6";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    println!("Fen: {}", fen);
    println!("Move: {}", mv);
    println!("{}", game.board());
    assert!(game.move_piece_san(mv));
    println!("{}", game.board());
}

/*
Invalid move at game #1982, ply #57: 'hxg6'. FEN before move: 3r4/pp5p/5k2/5ppP/2P2N1K/4r1P1/PP5R/8 w - g6 0 29
PGN: 1. f3 e5 2. Kf2 d5 3. e3 Bd6 4. g3 Ne7 5. d4 exd4 6. exd4 O-O 7. c3 c5 8. Kg2 Nbc6 9. dxc5 Bxc5 10. Bd3 Ne5 11. Nh3 Nxd3 12. Qxd3 Bf5 13. Qd1 Ng6 14. Nf4 Qd7 15. h4 Rfe8 16. Nxd5 Ne5 17. Nf4 Nxf3 18. Qxd7 Bxd7 19. Nd2 Nxd2 20. Bxd2 Bc6 21. Kh3 Bxh1 22. Rxh1 Rad8 23. Bc1 Be3 24. Bxe3 Rxe3 25. Rh2 f5 26. h5 Kf7 27. Kh4 Kf6 28. c4 g5 29. hxg6 hxg6 30. Nd5 Rxd5 31. cxd5 Re8 32. Kh3 Rh8 33. Kg2 Rxh2 34. Kxh2 Ke5 35. Kh3 g5 *


 */