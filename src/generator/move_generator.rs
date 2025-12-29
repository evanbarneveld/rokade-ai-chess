use crate::board::san_move::convert_move_to_san;
use crate::search::search::find_move;
use crate::state::game_state::GameState;

pub fn generate_move_as_san(game_state: GameState, search_depth: usize, playing_strength: usize) -> Option<String> {
    let generated_move = find_move(game_state, search_depth, playing_strength);
    convert_move_to_san(game_state, generated_move)
}
