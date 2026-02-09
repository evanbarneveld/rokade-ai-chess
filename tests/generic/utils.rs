use chess::Chess;
use chess::generator::move_generator::generate_move_as_san;
use chess::history::history::History;
use chess::search::core::advanced_search::DEFAULT_SEARCH_DEPTH;

pub fn read_fen_and_generate_best_move(fen: &str, move_time:usize) -> (Chess, History, String) {
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    game.set_deterministic(true);
    //get the best move
    let history = game.get_history().clone();
    let gs = *game.get_game_state();
    let ctx = game.search_context();

    let san_move = generate_move_as_san(
        &ctx,
        game.get_search_mode(),
        &gs,
        &history,
        DEFAULT_SEARCH_DEPTH,
        move_time,
        1000,
    ).unwrap();
    (game, history, san_move)
}
