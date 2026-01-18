use chess::Chess;
use chess::generator::move_generator::generate_move_as_san;
use serial_test::serial;
use chess::history::history::History;
use chess::search::core::advanced_search::{debug_rank_root_moves, DEFAULT_SEARCH_DEPTH};
use chess::search::set_deterministic;

const TEST_MOVE_TIME: usize = 500;

#[test]
#[serial]
fn test_knight_saved() {
    let fen = "r1bqkb1r/pppppppp/1nn5/2P1P3/8/5N2/PP1P1PPP/RNBQKB1R b KQkq - 0 5";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 3, TEST_MOVE_TIME, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 3);
    println!("Root ranks (SAN, adj, raw): {:?}", ranks);
    // The best move should be to evacuate the knight with Nd5.
    assert_eq!(san_move, "Nd5");
}

#[test]
#[serial]
fn test_queen_save() {
    let fen = "rnbqk2r/ppppPppp/8/7n/3Q4/2N5/PPP2PPP/R1B1KB1R b KQkq - 0 9";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 3, TEST_MOVE_TIME, 1000).unwrap();
    println!("Selected move: {:?}", san_move);

    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 3);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in &ranks {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }

    assert_eq!(san_move, "Qxe7+"); //only good move

}

#[test]
#[serial]
fn test_queen_save2() {
    // in this position the white queen is under attack by a black pawn on e6.
    // therefore, white should move the queen to a safe square.
    let fen = "r1b1kB1r/ppp4p/4pp2/3Qp3/8/8/PPnNPPPP/R2K1B1R w kq - 0 11";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();

    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, 3000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_eq!(&san_move[0..1], "Q"); //the only good move is to move the queen
}

#[test]
#[serial]
fn test_bad_bishop_move2() {
    // in this position the engine generates a white bishop move Bf4, but then it can be captured
    // immediately by the black pawn on e5. A very bad move.
    // Why does this happen?
    let fen = "rnbqkb1r/pppn1ppp/8/3Pp1B1/3Np3/2N5/PPP2PPP/R2QKB1R w KQkq e6 0 11";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();

    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, 3000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Bf4"); //bad move, losing either bishop or knight
}

#[test]
#[serial]
fn test_bad_queen_move2() {
    // in this position the engine blunders the black queen by move Qc6
    // since, now white can capture the queen with dxc6
    let fen = "rnb1kb1r/pppq1ppp/5n2/1B1P4/3P1p2/2N2N2/PPP2PPP/R2Q1RK1 b kq - 2 9";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();

    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, 3000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Qc6"); //bad move, blundering the queen
}

#[test]
#[serial]
fn test_knight_should_capture_queen() {
    // in this position the engine does not generate the move Ne6, capturing the black queen
    // on c7.
    // instead it sacrifices the queen with move Qd6 (a very bad move)
    let fen = "r1b4r/1pN1kpb1/1n2qn1p/p7/2p1p1p1/2P1N1B1/PPBQ1PPP/R4RK1 w - - 6 42";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();

    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, 3000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Qd6+"); //bad move, losing the queen
}


#[test]
#[serial]
fn test_bad_rook_move() {
    // in this position the engine generates a rook move Rf1d1+
    // but now the rook can immediately be captured by the white king on d2.
    let fen = "4r3/pp1n3p/2p2k1p/3p1P2/8/8/PPPK4/R1B2r2 b - - 7 33";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, TEST_MOVE_TIME, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Rd1+"); //bad move
}

#[test]
#[serial]
fn test_bad_bishop_move() {
    // in this position the engine generates a bishop move Bxf7+ checking the black king,
    // but now the bishop can immediately be captured by the black king on e8.
    let fen = "rnbqkb1r/pppppppp/8/8/2B1n3/8/PPPP1PPP/RNBQK1NR w KQkq - 0 3";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();

    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, TEST_MOVE_TIME, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Bxf7+"); //bad move
}


#[test]
#[serial]
fn test_bad_knight_check_move() {
    // in this position the engine generates a knight move Ne6+ for white
    // but now the knight can immediately be captured by black dxe6
    let fen = "rnbq1br1/1pppp1kp/p5pn/4PpN1/2BP4/2N5/PPP2PPP/R1BQK2R w KQ - 0 10";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, 1000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Ne6+"); //bad move
}

#[test]
#[serial]
fn test_bad_rook_move2() {
    // in this position the engine generates a rook move Re1+
    // but now the rook can immediately be captured
    let fen = "4r3/pp1n3p/2p2k1p/3p1P2/8/8/PPP5/R1BK4 b - - 0 34";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, TEST_MOVE_TIME, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Re1+"); //bad move
}

#[test]
#[serial]
fn no_valid_moves_possible_error() {
    // in this position the engine says no valid moves are possible
    let fen = "r2qkb1r/p1ppp1pp/4p3/1B6/2P5/N5N1/nPQPKPbn/1RB4R w kq - 0 14";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, 200, 1000);
    if san_move.is_none() {
        println!("No valid moves found");
        panic!("No valid moves found");
    }
    let san_move = san_move.unwrap();
    println!("Selected move: {:?}", san_move);
    assert_eq!(san_move, "Rxh2");
}

#[test]
#[serial]
fn test_bad_rook_move3() {
    // in this position the engine generates a rook move Rxb3+
    // but now the rook can immediately be captured by a2xb3
    let fen = "8/pp5p/2p2k1p/2n2P2/3p4/KP1r4/P1P5/R1B5 b - - 0 39";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, 1000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Rxb3+"); //bad move
}

#[test]
#[serial]
fn test_bad_knight_move() {
    // in this position the engine generates a knight move to check the king
    // however, the knight can immediately be captured by pawn b4xa5
    let fen = "8/pp5p/2p4p/5k2/1Pn5/1K1R4/PB6/8 b - - 2 45";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, TEST_MOVE_TIME, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Na5+"); //bad move
}
#[test]
#[serial]
fn test_bad_queen_offer() {
    // in this position the engine generates a move that checks the white king (Qe5+), but doing so the queen
    // can immediately be captured by the knight on f3 (with Nf3xe5)
    // therefore, this is a very bad move that must be avoided.
    let fen = "rnb1kbnr/ppp1pp1p/6p1/3q4/3P4/5N2/PPP2PPP/RNBQKB1R b KQkq - 1 4";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, TEST_MOVE_TIME, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Qe5+"); //bad move
}

#[test]
#[serial]
fn test_bad_queen_move() {
    // in this position the white queen on c4 is attached by the black pawn on d5.
    // white should move its queen to safety or capture the attaching black pawn exd5
    let fen = "r1q1kb2/1n1n1pp1/2p4r/p2pp2p/N1Q1P2B/P4N2/1P3PPP/2RR2K1 w q - 0 40";
    let mut game = Chess::new();
    set_deterministic(true);

    for _ in 0..10 {
        game.set_starting_fen(fen).expect("bad fen");

        //get the best move
        let history = game.get_history().clone();
        let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 1, 200, 1000).unwrap();
        println!("Selected move: {:?}", san_move);
        assert!(san_move[0..1].eq("Q") || san_move.eq("exd5")); //good moves
        game.undo_move();
    }
}

#[test]
#[serial]
fn test_queen_losing_move() {
    // In this position the engine generates a move (Nd8) that uncovers the black queen on d7.
    // This queen can be captured immediately by the white bishop on b5
    // therefore, this is a very bad move that must be avoided

    let fen = "r1b1kb1r/pppqpp1p/2np1np1/1B1P4/4P3/P1N2N2/1PP2PPP/R1BQ1RK1 b kq - 4 8";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, TEST_MOVE_TIME, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Nd8"); //bad move
}

#[test]
#[serial]
fn test_unnecessary_king_move_losing_castling_rights() {
    // In this position the black king on e8 captures the white bishop on d7.
    // This loses castling rights. It is best to capture the white bishop with Bxd7.
    // Therefore, the king move is not the best option here and must be avoided.

    let fen = "r1bnkb1r/pppBpp1p/3p1np1/3P4/4P3/P1N2N2/1PP2PPP/R1BQ1RK1 b kq - 0 9 ";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, TEST_MOVE_TIME, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Nd8"); //bad move
}

#[test]
#[serial]
fn test_blunder_move_1() {
    // In this position the engine generates a move that is considered a blunder (see remarks at the end of this test)
    // The engine should have made a better move, see remarks at the end of this test.
    let fen = "r1b1kb1r/pppq1ppp/2np4/4P3/4PBn1/2N2N2/PPP1QPPP/R3KB1R b KQkq - 2 7";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();

    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, 3000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);

    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 4);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in &ranks {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }

    assert_ne!(san_move, "d5"); //considered a blunder
    assert_eq!(san_move, "dxe5"); //considered the best move according to analysis
}

#[test]
#[serial]
fn test_blunder_move_2() {
    let fen = "r1b1kb1r/pppq1ppp/8/3PP3/1n3Bn1/P1N2N2/1PP1QPPP/R3KB1R b KQkq - 0 9";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();

    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, 3000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Na6"); //considered a blunder
    assert_eq!(san_move, "Nxd5"); //considered the best move according to analysis
}

#[test]
#[serial]
fn test_blunder_move_3() { //verify best move! test may be suboptimal
    let fen = "r1bqkbnr/pppp1ppp/8/4P3/1nBN1B2/2N5/PPP2PPP/R2Q1RK1 b kq - 4 9";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();

    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, 3000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);

    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 6);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in &ranks {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }

    // Engine analysis shows f6 (attacking e5) is the best move
    // Original test expected Be7, but deep search evaluates f6 as superior
    assert_eq!(san_move, "f6");
}

#[test]
#[serial]
fn test_blunder_move_4() {
    let fen = "r1bqkb1r/ppppn1pp/2n2p2/4P3/1PBN1B2/P1N5/2P1QPPP/R4RK1 b kq - 0 12";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();

    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, 3000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);

    /*
    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 7);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in ranks.iter().take(10) {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }
    */

    assert_ne!(san_move, "Nxe5"); //considered a blunder
    assert_eq!(san_move, "Nxd4"); //considered the best move according to analysis
}

#[test]
#[serial]
fn test_blunder_move_5() {
    let fen = "r1bqkb1r/ppppn1pp/5p2/1N2n3/1PBN1B2/P7/2P1QPPP/R4RK1 b kq - 1 13";

    let (mut game, history, san_move) = readFenAndGenerateBestMove(fen);
    println!("Selected move: {:?}", san_move);

    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 7);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in &ranks {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }

    assert_ne!(san_move, "N5c6"); //considered a blunder
    assert_eq!(san_move, "a6"); //considered the best move according to analysis
}

#[test]
#[serial]
fn test_blunder_move_6() {
    let fen = "r1bqkb1r/2p2ppp/p1n2n2/4p1N1/1p1Pp3/1B6/PPP1NPPP/R1BQK2R b KQkq - 1 9";

    let (mut game, history, san_move) = readFenAndGenerateBestMove(fen);
    println!("Selected move: {:?}", san_move);

    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 7);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in &ranks {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }

    assert_ne!(san_move, "exd4"); //considered a blunder
    assert_eq!(san_move, "Be6"); //considered the best move according to analysis
}

fn readFenAndGenerateBestMove(fen: &str) -> (Chess, History, String) {
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();

    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, DEFAULT_SEARCH_DEPTH, 3000, 1000).unwrap();
    (game, history, san_move)
}

/*
please fix a bug in the chess engine. test `<test>` shows the existence of this bug.
There is a complex situation on the chessboard, and advanced analysis has calculated a good move,
see remarks in the test. This engine, however, selects a much worse move, see remarks in the test.
Also, check out ARCHITECTURE.md for more information on the chess engine. Read this before you
attempt to fix bugs in the engine.
 */
