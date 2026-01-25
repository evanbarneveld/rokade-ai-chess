use crate::board::san_move::convert_move_to_san;
use crate::search::{find_best_move_with_mode, SearchMode};
use crate::search::context::SearchContext;
use crate::state::game_state::GameState;
use crate::history::history::History;

pub fn generate_move_as_san(
    ctx: &SearchContext,
    mode: SearchMode,
    game_state: &GameState,
    history: &History,
    search_depth: usize,
    move_time_in_ms: usize,
    playing_strength: usize,
) -> Option<String> {
    ctx.set_time_budget_ms(move_time_in_ms);
    let generated_move = find_best_move_with_mode(ctx, mode, game_state, history, search_depth, playing_strength)
        .map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    ctx.clear_time_budget();
    convert_move_to_san(game_state, generated_move)
}
