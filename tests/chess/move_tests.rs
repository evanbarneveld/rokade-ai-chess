use chess::Chess;
use chess::generator::move_generator::generate_move_as_san;
use serial_test::serial;
use chess::search::advanced_search::debug_rank_root_moves;
use chess::search::set_deterministic;

#[test]
#[serial]
fn test_mate_in_2_move1() {
    let fen = "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    // Ensure stable choice
    set_deterministic(true);
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    //get best move
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 5, 1000, 1000).unwrap();
    println!("Best move: {:?}", san_move);
    // the engine chooses Ke1xd1, which is certainly not the best move
    // Expect official SAN with disambiguation and check marker
    assert_eq!(san_move, "Nf6+");
}

#[test]
#[serial]
fn test_knight_saved() {
    let fen = "r1bqkb1r/pppppppp/1nn5/2P1P3/8/5N2/PP1P1PPP/RNBQKB1R b KQkq - 0 5";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    set_deterministic(true);
    //get best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 5, 1000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 4, 1000);
    println!("Root ranks (SAN, adj, raw): {:?}", ranks);
    // The best move should be to evacuate the knight with Nd5.
    assert_eq!(san_move, "Nd5");
}

#[test]
#[serial]
fn test_knight_move() {
    let fen = "rnbqk2r/pppp1ppp/8/4P3/3Q2n1/2N5/PPP2PPP/R1B1KB1R b KQkq - 0 7";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    set_deterministic(true);
    //get best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 5, 0, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 4, 1000);
    println!("Root ranks (SAN, adj, raw): {:?}", ranks);
    // The best move should be to evacuate the knight with Nd5.
    assert_ne!(san_move, "Nf6"); //bad move
    assert_eq!(san_move, "Qh4")    //good move
}

#[test]
#[serial]
fn test_queen_save() {
    let fen = "rnbqk2r/ppppPppp/8/7n/3Q4/2N5/PPP2PPP/R1B1KB1R b KQkq - 0 9";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    set_deterministic(true);
    //get best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 4, 0, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_eq!(san_move, "Qxe7+"); //only good move

    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 4, 1000);
    println!("Root ranks (SAN, adj, raw): {:?}", ranks);
    // The best move saves the queen

}

#[test]
#[serial]
fn test_queen_save2() {
    // in this position the white queen is under attack by a black pawn on e6.
    // therefore, white should move the queen to a safe square.
    // d8 is a bad square, because it can be immediately captured by the black king.
    let fen = "r1b1kB1r/ppp4p/4pp2/3Qp3/8/8/PPnNPPPP/R2K1B1R w kq - 0 11";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    set_deterministic(true);
    //get best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 4, 0, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_eq!(&san_move[0..1], "Q"); //only good move is to move the queen
    assert_ne!(san_move, "Qd8+"); //bad move
}


#[test]
#[serial]
fn test_bad_queen_offer() {
    // in this position the engine generates a move that checks the white king (Qe5+), but doing so the queen
    // can immediately be captured by the knight on f3 (with Nf3xe5)
    // therefore, this is a very bad move that must be avoided.
    let fen = "rnb1kbnr/ppp1pp1p/6p1/3q4/3P4/5N2/PPP2PPP/RNBQKB1R b KQkq - 1 4";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    set_deterministic(true);
    //get best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 4, 0, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Qe5+"); //bad move
}


#[test]
#[serial]
fn test_queen_losing_move() {
    // In this position the engine generates a move (Nd8) that uncovers the black queen on d7.
    // This queen can be captured immediately by the white bishop on b5
    // therefore, this is a very bad move that must be avoided

    /*
    Bad analysis done by our chess engine:

    FEN: r1bnkb1r/pppqpp1p/3p1np1/1B1P4/4P3/P1N2N2/1PP2PPP/R1BQ1RK1 w kq - 5 9

    Chess:
     1	00:00	 0	0	-211.23	Nc6-a5
     2	00:00	 738	23k	-211.23	Nc6-a5 Bb5xd7+
     3	00:00	 3k	    35k	-211.23	Nc6-a5 Bb5xd7+ Bc8xd7
     4	00:01	 41k	41k	-211.48	Nc6-a5 Bb5xd7+ Bc8xd7 b2-b3
     5	00:02	 121k	57k	-211.98	Nc6-d8 Bb5xd7+ Bc8xd7 Qd1-d4
     5	00:02	 122k	57k	-211.98	Nc6-d8


    Here is way better analysis done by AnMon chess in the same position

    FEN: r1b1kb1r/pppq1p1p/2np1np1/1B1Pp3/4P3/P1N2N2/1PP2PPP/R1BQ1RK1 w kq e6 0 9

    AnMon 5.75:
    1+	00:00	 3	13	-2.89	b7xc6
    1	00:00	 4	18	-2.89	b7xc6
    2	00:00	 106	452	-2.86	b7xc6 Bb5-c4
    */

    let fen = "r1b1kb1r/pppqpp1p/2np1np1/1B1P4/4P3/P1N2N2/1PP2PPP/R1BQ1RK1 b kq - 4 8";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    set_deterministic(true);
    //get best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 5, 0, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Nd8"); //bad move
}

#[test]
#[serial]
fn test_unnecessary_king_move_losing_castling_rights() {
    // In this position the black king on e8 captures the white bishop on d7.
    // This loses castling rights. It is best to capture white bishop with Bxd7.
    // Therefore, the king move, is not the best option here and must be avoided.

    /*

    Bad analysis done by our chess engine:

    FEN: r1bn1b1r/pppkpp1p/3p1np1/3P4/4P3/P1N2N2/1PP2PPP/R1BQ1RK1 w - - 0 10

    Chess:
    1	00:00	 0	0	-2.25	Ke8xd7
    2	00:00	 6	6k	-2.35	Ke8xd7 Qd1-e2
    3	00:00	 12	12k	-2.35	Ke8xd7 Qd1-e2 Nf6-g4
    4	00:00	 19	19k	-2.35	Ke8xd7 Qd1-e2 Nf6-g4 Qe2-b5+
    5	00:00	 37	37k	-2.35	Ke8xd7 Qd1-e2 Nf6-g4 Qe2-b5+ c7-c6
    5	00:00	 46	0	-2.35	Ke8xd7
    */

    let fen = "r1bnkb1r/pppBpp1p/3p1np1/3P4/4P3/P1N2N2/1PP2PPP/R1BQ1RK1 b kq - 0 9 ";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    set_deterministic(true);
    //get best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 5, 0, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Nd8"); //bad move
}

/*

In case a new failing test is added, use this prompt for AI code gen:

test '<name-of-test>' is failing.
See the comments in the test: it explains what happens.

There is also the analysis of our chess engine.
Depending on the test, I may have added a good analysis of another
existing chess engine that does a better job in this position.

Please fix the code of the chess engine.
Always prefer a generic solution, rather than a specific one.
The other tests should keep working off course.
Execute the test without asking.
Change code without asking. I will review the code changes once the test works.
If the test still doesn't work after 5 attempts, ask me what I want to do.

If the test is working, run all tests in move_tests.rs to see if there is a regression.
Let me know if all test work, very clearly. For example:

Hey Erik, all tests in move_tests.rs succeed!

*/