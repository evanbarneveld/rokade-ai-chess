pub(crate) mod advanced_search;
pub(crate) mod tt;
pub(crate) mod zobrist;
pub(crate) mod heuristics;
pub(crate) mod playing_strength;
pub(crate) mod time_control;
pub mod uci_feedback;
pub(crate) mod threading;
pub(crate) mod telemetry;
pub(crate) mod locking;
pub(crate) mod prune_null_moves;
pub(crate) mod qsearch;
pub(crate) mod see;
mod alphabeta;
mod root_moves;
mod simple_search;

// Trait interface for Search implementations
// Note: The method is an associated function (no &self) to match the existing API.
// Implementors can forward to their internal logic.
pub trait Search {
    fn find_best_move(
        game_state: &crate::state::game_state::GameState,
        history: &crate::history::history::History,
        search_depth: usize,
        playing_strength: usize,
    ) -> Option<((usize, usize), (usize, usize), i32, usize)>;
}

// Public toggle to select which search to use at runtime
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SearchMode {
    Advanced,
    Simple,
}

/// Returns some move if the best move was found or none if no move was available.

/// Format of some move is
///  ((usize, usize), (usize, usize), i32, usize)
///
/// from square (rank, col) to square (rank, col), score, and used depth
pub fn find_best_move_with_mode(
    mode: SearchMode,
    game_state: &crate::state::game_state::GameState,
    history: &crate::history::history::History,
    search_depth: usize,
    playing_strength: usize,
) -> Option<((usize, usize), (usize, usize), i32, usize)> {
    match mode {
        SearchMode::Advanced => advanced_search::AdvancedSearch::find_best_move(
            game_state,
            history,
            search_depth,
            playing_strength,
        ),
        SearchMode::Simple => simple_search::SimpleSearch::find_best_move(
            game_state,
            history,
            search_depth,
            playing_strength,
        ),
    }
}