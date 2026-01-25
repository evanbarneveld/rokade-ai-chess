use chess::Chess;
use chess::generator::move_generator::generate_move_as_san;
use chess::search::SearchContext;

#[test]
fn test_pawn_promotion_capture() {
    // White pawn on a7, black rook on b8. White should promote by capturing.
    // Position: 1r6/P7/8/8/8/8/k7/7K w - - 0 1
    let fen = "1r6/P7/8/8/8/8/k7/7K w - - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    let ctx = deterministic_ctx();

    let history = game.get_history().clone();
    let san_move = generate_move_as_san(&ctx, game.get_search_mode(), game.get_game_state(), &history, 3, 1000, 1000).unwrap();
    println!("Selected move: {:?}", san_move);

    // The best move MUST be axb8=Q
    assert!(san_move.contains("axb8"), "Move {} should be axb8", san_move);
    assert!(san_move.contains("="), "Move {} should be a promotion", san_move);
}

fn deterministic_ctx() -> SearchContext {
    let ctx = SearchContext::new();
    ctx.set_deterministic(true);
    ctx
}
