pub mod advanced_search;
pub(crate) mod tt;
pub(crate) mod zobrist;
pub(crate) mod heuristics;
pub mod playing_strength;
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

// Determinism toggle for search/engine behavior
use std::sync::atomic::{AtomicBool, Ordering};

static DETERMINISTIC: AtomicBool = AtomicBool::new(false);
static PARALLEL_SEARCH: AtomicBool = AtomicBool::new(true);

/// Enable or disable deterministic behavior across the engine.
/// When enabled, all random choices and evaluation noise are suppressed,
/// and components should pick the most stable/best option instead of sampling.
pub fn set_deterministic(on: bool) {
    DETERMINISTIC.store(on, Ordering::Relaxed);
}

/// Returns true when deterministic behavior is enabled.
#[inline]
pub fn is_deterministic() -> bool {
    DETERMINISTIC.load(Ordering::Relaxed)
}

pub fn get_deterministic() -> bool {
    is_deterministic()
}

pub fn set_parallel_search(on: bool) {
    PARALLEL_SEARCH.store(on, Ordering::Relaxed);
}

#[inline]
pub fn is_parallel_search() -> bool {
    PARALLEL_SEARCH.load(Ordering::Relaxed)
}

// Trait interface for Search implementations
// Note: The method is an associated function (no &self) to match the existing API.
// Implementors can forward to their internal logic.
pub trait Search {
    fn find_best_move(
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
    mode: SearchMode,
    game_state: &crate::state::game_state::GameState,
    history: &crate::history::history::History,
    search_depth: usize,
    playing_strength: usize,
) -> Option<((usize, usize), (usize, usize), Option<char>, i32, usize)> {
    match mode {
        SearchMode::Normal => advanced_search::AdvancedSearch::find_best_move(
            game_state,
            history,
            search_depth,
            playing_strength,
        ),
        SearchMode::Test => simple_search::SimpleSearch::find_best_move(
            game_state,
            history,
            search_depth,
            playing_strength,
        ),
    }
}