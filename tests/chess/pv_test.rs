use std::sync::Arc;
use chess::search::uci_feedback::set_info_callback;

#[test]
fn depth2_pv_first_move_is_exd5() {
    use chess::Chess;
    let fen = "8/8/4p3/3Q4/8/8/8/2K2k2 b - - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen);

    let last_pv: Arc<std::sync::Mutex<Vec<((usize, usize), (usize, usize))>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let last_pv_clone = last_pv.clone();
    set_info_callback(Some(Arc::new(move |_mv, _sc, _depth, pv, _hf| {
        *last_pv_clone.lock().unwrap() = pv; // capture PV from the most recent iteration
    })));

    let history = game.get_history().clone();
    let depth = 2usize;
    let _san = chess::generator::move_generator::generate_move_as_san(
        *game.get_game_state(), &history, depth, 10_000, 1000
    ).unwrap();

    set_info_callback(None);

    // Convert the first PV move to SAN and assert it is exd5
    use chess::board::san_move::convert_move_to_san;
    let gs = *game.get_game_state();
    let last_pv_guard = last_pv.lock().unwrap();
    assert!(!last_pv_guard.is_empty(), "PV should not be empty");
    let first_move_san = convert_move_to_san(gs, Some(last_pv_guard[0])).unwrap();
    assert!(first_move_san.starts_with("e6xd5"), "expected PV to start with exd5, got {}", first_move_san);
}