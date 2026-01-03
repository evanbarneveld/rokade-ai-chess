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
    let fen = "rnb1kbnr/ppp1pp1p/6p1/3q4/3P4/5N2/PPP2PPP/RNBQKB1R b KQkq - 1 4 ";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    set_deterministic(true);
    //get best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 4, 0, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Qe5+"); //bad move
}
