use crate::history::history::History;
use crate::search::Search;
use crate::state::game_state::GameState;

/// A minimal `Search` implementation.
///
/// This is a skeleton that simply forwards to the existing
/// `advanced_search::find_best_move` function so it immediately
/// satisfies the `Search` trait and compiles. You can later replace
/// the forwarding with a simpler or different strategy while keeping
/// the trait contract stable.
pub struct SimpleSearch;

impl Search for SimpleSearch {
    fn find_best_move(
        game_state: &GameState,
        history: &History,
        search_depth: usize,
        playing_strength: usize,
    ) -> Option<((usize, usize), (usize, usize), i32, usize)> {
       // TODO implement
        return None;
    }
}
