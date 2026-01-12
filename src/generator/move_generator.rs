use crate::board::san_move::convert_move_to_san;
use crate::search::{find_best_move_with_mode, SearchMode};
use crate::state::game_state::GameState;
use crate::history::history::History;
use crate::search::time_control::{clear_time_budget, set_time_budget_ms};

pub fn generate_move_as_san(mode: SearchMode, game_state: GameState, history: &History, search_depth: usize, move_time_in_ms: usize, playing_strength: usize) -> Option<String> {
    set_time_budget_ms(move_time_in_ms);
    let generated_move = match find_best_move_with_mode(mode, &game_state, history, search_depth, playing_strength) {
        Some((from, to, promo, _score_cp, _depth_used)) => Some((from, to, promo)),
        None => None,
    };
    clear_time_budget();
    convert_move_to_san(game_state, generated_move)
}
