use chess::Chess;
use chess::generator::move_generator::generate_move_as_san;
use chess::search::set_deterministic;

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
    assert_eq!(san_move, "Nf6+");
}