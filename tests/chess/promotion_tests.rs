use chess::Chess;
use chess::generator::move_generator::generate_move_as_san;
use serial_test::serial;
use chess::search::set_deterministic;

#[test]
#[serial]
fn test_pawn_promotion() {
    // White pawn on a7, just needs to move to a8 to promote.
    // Position: 8/P7/8/8/8/8/k7/7K w - - 0 1
    let fen = "8/P7/8/8/8/8/k7/7K w - - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    
    let history = game.get_history().clone();
    // Use depth 3, strength 1000
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 3, 1000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    
    // The best move MUST be a8=Q or similar promotion
    assert!(san_move.contains("="), "Move {} should be a promotion", san_move);
    assert!(san_move.contains("a8"), "Move {} should be to a8", san_move);
}

#[test]
#[serial]
fn test_pawn_promotion_capture() {
    // White pawn on a7, black rook on b8. White should promote by capturing.
    // Position: 1r6/P7/8/8/8/8/k7/7K w - - 0 1
    let fen = "1r6/P7/8/8/8/8/k7/7K w - - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    set_deterministic(true);
    
    let history = game.get_history().clone();
    let san_move = generate_move_as_san(game.get_search_mode(), *game.get_game_state(), &history, 3, 1000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);
    
    // The best move MUST be axb8=Q
    assert!(san_move.contains("axb8"), "Move {} should be axb8", san_move);
    assert!(san_move.contains("="), "Move {} should be a promotion", san_move);
}
