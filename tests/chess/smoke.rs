use chess::Chess;
use chess::piece::pieces::Color;
use chess::state::outcome::OutcomeType;

#[test]
fn initial_fen() {
    let g = Chess::new();
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
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert!( game.move_piece_san("cxb3"));
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert_eq!(game.to_fen(), "rnbqkbnr/p1pppppp/P7/8/8/1P6/1P1PPPPP/RNBQKBNR b KQkq - 0 4");
}

#[test]
fn test_en_passant_pawn_capture() {
    let mut game = Chess::new();
    game.set_starting_fen("rnbqkbnr/1ppppp1p/8/p3P3/6pP/1P6/P1PP1PP1/RNBQKBNR b KQkq h3 0 4");
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert!( game.move_piece_san("g4xh3"));
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert_eq!(game.to_fen(), "rnbqkbnr/1ppppp1p/8/p3P3/8/1P5p/P1PP1PP1/RNBQKBNR w KQkq - 0 5");
}

#[test]
fn test_en_passant_capture_black() {
    let mut game = Chess::new();
    game.set_starting_fen("rn2k1nr/ppp2ppp/8/2bqpb2/2Pp4/1K3P2/PP1PP1PP/RNBQ1BNR b kq c3 0 7");
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert!(game.move_piece_san("dxc3"));
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
}

#[test]
fn test_pawn_move_over_piece() {
    let mut game = Chess::new();
    game.set_starting_fen("rnbqkbn1/ppppppp1/3r4/7p/7P/3R4/PPPPPPP1/RNBQKBN1 w KQkq h6 4 4");
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert!( !game.move_piece_san("d2d4"));
    assert!(game.move_piece_san("a2a4"));
    assert!( !game.move_piece_san("d7d5"));
}

#[test]
fn test_castling() {
    let mut game = Chess::new();
    game.set_starting_fen("r1bqkbnr/1ppp1ppp/p1n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4");
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert!(game.move_piece_san("O-O"));
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert_eq!(game.to_fen(), "r1bqkbnr/1ppp1ppp/p1n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 1 4");
}

#[test]
fn test_castling_black() {
    let mut game = Chess::new();
    game.set_starting_fen("r3kbnr/1pp2ppp/p1n1b3/3pp1q1/P1B1P3/5N1P/1PPP1PP1/RNBQ1RK1 b kq - 0 7");
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert!(game.move_piece_san("O-O-O"));
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
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
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
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
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert!(game.move_piece_san(mv));
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
}

#[test]
fn black_en_passant_error() {
    let fen = "3r4/pp5p/5k2/5ppP/2P2N1K/4r1P1/PP5R/8 w - g6 0 29";
    let mv = "hxg6";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    println!("Fen: {}", fen);
    println!("Move: {}", mv);
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert!(game.move_piece_san(mv));
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
}

#[test]
fn test_check_mate() {
    let fen = "r4rk1/ppp2ppp/2n5/3pp3/4n2q/7P/PPPPP1BP/RNBQ2KR b - - 4 11";
    let mv = "Qf2";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    println!("Fen: {}", fen);
    println!("Move: {}", mv);
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert!(game.move_piece_san(mv));
    println!("{}", game.board().get_board_display_string(Some(&history)));
    let history = game.get_history().clone();
    game.get_game_state().recompute_outcome(&history);
    let outcome = game.get_game_state().get_outcome().unwrap();
    println!("Outcome: {:?}", outcome);
    assert_eq!(outcome, OutcomeType::Checkmate { winner: Color::Black})
}

#[test]
fn test_check_mate2() {
    let fen = "rn2k1nr/ppp2ppp/8/2b1pb2/3q4/1K3P2/PP1PP1PP/RNBQ1BNR b kq - 2 9";
    let mv = "Qb4+";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    println!("Fen: {}", fen);
    println!("Move: {}", mv);
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert!(game.move_piece_san(mv));
    println!("{}", game.board().get_board_display_string(Some(&history)));
    let history = game.get_history().clone();
    game.get_game_state().recompute_outcome(&history);
    let outcome = game.get_game_state().get_outcome().unwrap();
    println!("Outcome: {:?}", outcome);
    assert_eq!(outcome, OutcomeType::Checkmate { winner: Color::Black})
}

#[test]
fn test_check_with_o_o_o() {
    let fen = "r3kb1r/pp2pppp/n1p2n2/5b2/2P4N/2N3P1/PP2PP1P/R1BK1B1R b kq - 4 8";
    let mv = "O-O-O+";
    //let mv = "e2";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    println!("Fen: {}", fen);
    println!("Move: {}", mv);
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    assert!(game.move_piece_san(mv));
    println!("{}", game.board().get_board_display_string(Some(&history)));
    let history = game.get_history().clone();
    game.get_game_state().recompute_outcome(&history);
    let outcome = game.get_game_state().get_outcome().unwrap();
    println!("Outcome: {:?}", outcome);
    assert_eq!(outcome, OutcomeType::InCheck)
}

/*

1. Nf3 d6 2. b3 e5 3. Bb2 e4 4. Nd4 c5 5. Nb5 d5 6. e3 Nc6 7. c4 d4 8.
exd4 cxd4 9. d3 Bb4+ 10. Ke2 Qe7 11. Nxd4 exd3+ 12. Kxd3 Nxd4 13. Kxd4 Bf5
14. Qe2 O-O-O# 0-1

 */