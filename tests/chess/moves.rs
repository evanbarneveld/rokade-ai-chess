use chess::Chess;
use chess::generator::move_generator::generate_move_as_san;
use chess::search::set_deterministic;
use chess::search::advanced_search::debug_rank_root_moves;

#[test]
fn test_mate_in_2_move1() {
    let fen = "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    // Ensure stable choice
    set_deterministic(true);
    let history = game.get_history().clone();
    println!("{}", game.board().get_board_display_string(Some(&history)));
    //get best move
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 20, 10000, 1000).unwrap();
    println!("Best move: {:?}", san_move);
    // the engine chooses Ke1xd1, which is certainly not the best move
    // Expect official SAN with disambiguation and check marker
    assert_eq!(san_move, "Nf6+");
}

#[test]
fn test_knight_lost() {
    let fen = "r1bqkb1r/pppppppp/1nn5/2P1P3/8/5N2/PP1P1PPP/RNBQKB1R b KQkq - 0 5";
    let mut game = Chess::new();
    game.set_starting_fen(fen);
    set_deterministic(true);
    //get best move
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 5, 0, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 5, 1000);
    println!("Root ranks (SAN, adj, raw): {:?}", ranks);
    // The best move should be to evacuate the knight with Nd5.
    assert_eq!(san_move, "Nd5");
}