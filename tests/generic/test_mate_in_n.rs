use chess::Chess;
use chess::generator::move_generator::generate_move_as_san;

#[test]
fn test_mate_in_2_brilliant_queen_offer() {
    let fen = "k1n3rr/Pp3p2/3q4/3N4/3Pp2p/1Q2P1p1/3B1PP1/R4RK1 w - - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    let history = game.get_history().clone();

    let ctx = crate::generic::blunder_tests::deterministic_ctx();
    let san_move = generate_move_as_san(
        &ctx,
        game.get_search_mode(),
        game.get_game_state(),
        &history,
        4,
        0,
        1000,
    ).unwrap();
    println!("Selected move: {:?}", san_move);

    /*
    let ranks = debug_rank_root_moves(&ctx, game.get_game_state(), &history, 4);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in &ranks {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }
    */

    assert_eq!(san_move, "Qxb7+");
}

#[test]
fn test_mate_in_3_failure_1() {
    let fen = "r1bq1k1r/pp2R1pp/2pp1p2/1n1N4/8/3P1Q2/PPP2PPP/R1B3K1 w - - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    let history = game.get_history().clone();

    let ctx = crate::generic::blunder_tests::deterministic_ctx();
    let san_move = generate_move_as_san(
        &ctx,
        game.get_search_mode(),
        game.get_game_state(),
        &history,
        6,
        0,
        1000,
    ).unwrap();
    println!("Selected move: {:?}", san_move);

    /*
    let ranks = debug_rank_root_moves(&ctx, game.get_game_state(), &history, 6);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in &ranks {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }*/

    assert_eq!(san_move, "Qxf6+");
}

//TODO #[test]
fn test_mate_in_4_failure_1() {
    /*
    FEN: 5rbk/2pq3p/5PQR/p7/3p3R/1P4N1/P5PP/6K1 w - - 0 1
    Solution: 1. Nf5 Rf7 2. Rg4 Qe8 3. Qg7+ Rxg7 4. fxg7#
     */
    let fen = "5rbk/2pq3p/5PQR/p7/3p3R/1P4N1/P5PP/6K1 w - - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    let history = game.get_history().clone();

    let ctx = crate::generic::blunder_tests::deterministic_ctx();
    let san_move = generate_move_as_san(
        &ctx,
        game.get_search_mode(),
        game.get_game_state(),
        &history,
        8,
        15000, //this mate_in_4 requires at least +/- 15-20 seconds!!!
        1000,
    ).unwrap();
    println!("Selected move: {:?}", san_move);

    /*let ranks = debug_rank_root_moves(&ctx, game.get_game_state(), &history, 8);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in &ranks {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }*/

    assert_eq!(san_move, "Nf5");
}

//TODO #[test]
fn test_mate_in_4_failure_2() {
    /*
        Fraser vs Farrow, corr., 1896
        1rb3k1/ppN2R1p/2n1P1p1/6p1/6B1/8/PPP3PP/6K1 w - - 0 1
        1. Nd5 Bxe6 2. Bxe6 Kh8 3. Nf6 (Ne5 or whatever) 4. Rxh7#
     */
    let fen = "1rb3k1/ppN2R1p/2n1P1p1/6p1/6B1/8/PPP3PP/6K1 w - - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    let history = game.get_history().clone();

    let ctx = crate::generic::blunder_tests::deterministic_ctx();
    let san_move = generate_move_as_san(
        &ctx,
        game.get_search_mode(),
        game.get_game_state(),
        &history,
        10,
        60000, //this mate_in_4 requires at least +/- 15-20 seconds!!!
        1000,
    ).unwrap();
    println!("Selected move: {:?}", san_move);

    /*let ranks = debug_rank_root_moves(&ctx, game.get_game_state(), &history, 8);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in &ranks {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }*/

    assert_eq!(san_move, "Nd5");
}
