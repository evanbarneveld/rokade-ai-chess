//! Lazy SMP (Symmetric Multi-Processing) parallel search implementation.
//!
//! Multiple threads search independently from the root, sharing a lock-free
//! transposition table. This is a simple but effective parallelization strategy
//! used by modern chess engines like Stockfish.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::history::history::History;
use crate::search::context::SearchContext;
use crate::search::core::advanced_search::{find_best_move, MAX_PLAYING_STRENGTH};
use crate::state::game_state::GameState;

/// Minimum depth to enable Lazy SMP (below this, serial search is faster)
const LAZY_SMP_MIN_DEPTH: usize = 5;

/// Minimum time budget (ms) to enable Lazy SMP
const LAZY_SMP_MIN_TIME_MS: usize = 300;

/// Maximum helper threads to spawn (diminishing returns beyond this)
const MAX_HELPER_THREADS: usize = 3;

/// Performs Lazy SMP parallel search.
///
/// Spawns multiple threads that search independently, all sharing the same
/// transposition table. Helper threads skip early depths to create search
/// diversity and avoid duplicating work.
///
/// Returns the result from the main thread (thread 0), which benefits from
/// TT entries populated by helper threads.
pub fn lazy_smp_search(
    ctx: &SearchContext,
    game_state: &GameState,
    history: &History,
    search_depth: usize,
    playing_strength: usize,
) -> Option<((usize, usize), (usize, usize), Option<char>, i32, usize)> {
    // Check if we should use Lazy SMP
    let num_threads = rayon::current_num_threads();
    let time_budget = ctx.time_budget_ms();

    // Use serial search if:
    // - Only 1 thread available
    // - Depth too shallow
    // - Not enough time
    // - Parallel search disabled
    if num_threads <= 1
        || search_depth < LAZY_SMP_MIN_DEPTH
        || (time_budget > 0 && time_budget < LAZY_SMP_MIN_TIME_MS)
        || !ctx.is_parallel_search()
    {
        return find_best_move(ctx, game_state, history, search_depth, playing_strength);
    }

    // Get shared TT for helper threads
    let shared_tt = ctx.shared_tt();

    // Shared stop flag for coordination
    let stop_flag = Arc::new(AtomicBool::new(false));

    // Spawn helper threads (threads 1..N)
    // Each helper searches with the shared TT
    let mut handles = Vec::with_capacity(MAX_HELPER_THREADS);

    let helper_count = (num_threads - 1).min(MAX_HELPER_THREADS);

    for thread_id in 1..=helper_count {
        let gs_clone = *game_state;
        let hist_clone = history.clone();
        let stop = Arc::clone(&stop_flag);
        let tt_clone = Arc::clone(&shared_tt);

        // Copy time settings
        let time_budget_ms = ctx.time_budget_ms();

        let handle = thread::spawn(move || {
            // Create a helper context that shares the TT
            let helper_ctx = SearchContext::new_smp_helper(tt_clone);

            // Copy time budget to helper
            if time_budget_ms > 0 {
                helper_ctx.set_time_budget_ms(time_budget_ms);
            }

            // Helper threads search at slightly different depths for diversity
            // Thread 1: same depth, Thread 2: depth-1 (if > 4), Thread 3: depth+1
            let adjusted_depth = match thread_id {
                1 => search_depth,
                2 if search_depth > 4 => search_depth - 1,
                3 => search_depth.saturating_add(1).min(search_depth + 2),
                _ => search_depth,
            };

            // Check if already stopped
            if stop.load(Ordering::Relaxed) {
                return;
            }

            // Search with full strength (helpers always use full strength)
            let _ = find_best_move(
                &helper_ctx,
                &gs_clone,
                &hist_clone,
                adjusted_depth,
                MAX_PLAYING_STRENGTH,
            );

            // Helper's result is stored in the shared TT, which is the main benefit
        });

        handles.push(handle);
    }

    // Main thread searches with the original context
    let main_result = find_best_move(ctx, game_state, history, search_depth, playing_strength);

    // Signal helpers to stop (they may already be done)
    stop_flag.store(true, Ordering::Relaxed);

    // Wait for helpers to finish
    for handle in handles {
        let _ = handle.join();
    }

    // Return main thread's result
    // The main thread benefits from TT entries populated by helpers
    main_result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::game_state::GameState;
    use crate::history::history::History;

    #[test]
    fn test_lazy_smp_returns_valid_move() {
        let ctx = SearchContext::new();
        ctx.set_time_budget_ms(5000);
        ctx.set_order_book_enabled(false); // Disable book to force search
        let gs = GameState::default();
        let history = History::new();

        let result = lazy_smp_search(&ctx, &gs, &history, 6, MAX_PLAYING_STRENGTH);

        assert!(result.is_some(), "Lazy SMP should return a move");
        let (from, to, _promo, _score, depth) = result.unwrap();
        assert!(from != to, "Move should be valid");
        assert!(depth >= 1, "Should search at least depth 1");
    }

    #[test]
    fn test_lazy_smp_falls_back_to_serial_at_low_depth() {
        let ctx = SearchContext::new();
        ctx.set_order_book_enabled(false); // Disable book to force search
        let gs = GameState::default();
        let history = History::new();

        // Depth 3 should use serial search
        let result = lazy_smp_search(&ctx, &gs, &history, 3, MAX_PLAYING_STRENGTH);

        assert!(result.is_some(), "Should return a move even at low depth");
    }
}
