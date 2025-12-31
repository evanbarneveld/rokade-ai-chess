use crate::board::san_move::convert_move_to_san;
use crate::search::search::find_move_with_info;
use crate::state::game_state::GameState;
use crate::history::history::History;
use crate::search::time_control::{clear_time_budget, set_time_budget_ms};

pub fn generate_move_as_san(game_state: GameState, history: &History, search_depth: usize, movetime: usize, playing_strength: usize) -> Option<String> {
    set_time_budget_ms(movetime);
    let generated_move = match find_move_with_info(game_state, history, search_depth, playing_strength) {
        Some((from, to, _score_cp, _depth_used)) => Some((from, to)),
        None => None,
    };
    clear_time_budget();
    convert_move_to_san(game_state, generated_move)
}
