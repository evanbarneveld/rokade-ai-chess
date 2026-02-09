pub mod core;
pub mod management;
mod evaluation;
pub(crate) mod state;
pub mod integration;
pub mod context;
#[doc(hidden)]
pub mod test_support;

use core::simple_search;
use crate::search::core::advanced_search;
use crate::search::integration::lazy_smp;
pub use context::{SearchContext, InfoCb};

pub trait Search {
    fn find_best_move(
        ctx: &SearchContext,
        game_state: &crate::state::game_state::GameState,
        history: &crate::history::history::History,
        search_depth: usize,
        playing_strength: usize,
    ) -> Option<((usize, usize), (usize, usize), Option<char>, i32, usize)>;
}

// Public toggle to select which search to use at runtime
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SearchMode {
    Normal,
    Test,
}

/// Returns some move if the best move was found or none if no move was available.
/// Format of some move is
///  ((usize, usize), (usize, usize), i32, usize)
///
/// from square (rank, col) to square (rank, col), score, and used depth
pub fn find_best_move_with_mode(
    ctx: &SearchContext,
    mode: SearchMode,
    game_state: &crate::state::game_state::GameState,
    history: &crate::history::history::History,
    search_depth: usize,
    playing_strength: usize,
) -> Option<((usize, usize), (usize, usize), Option<char>, i32, usize)> {
    match mode {
        // Use Lazy SMP for normal search mode (parallel search with shared TT)
        SearchMode::Normal => lazy_smp::lazy_smp_search(
            ctx,
            game_state,
            history,
            search_depth,
            playing_strength,
        ),
        SearchMode::Test => simple_search::SimpleSearch::find_best_move(
            ctx,
            game_state,
            history,
            search_depth,
            playing_strength,
        ),
    }
}

