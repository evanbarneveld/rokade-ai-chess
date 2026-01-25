#![doc(hidden)]

use crate::state::game_state::GameState;

pub fn book_pick(game_state: &GameState, deterministic: bool) -> Option<((usize, usize), (usize, usize))> {
    crate::book::book::book_pick(game_state, deterministic)
}
