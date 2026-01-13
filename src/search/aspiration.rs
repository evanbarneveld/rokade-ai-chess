use crate::history::history::History;
use crate::piece::pieces::Color;
use crate::search::root_evaluator::evaluate_root_for_bounds;
use crate::search::tt::TranspositionTable;
use crate::state::game_state::GameState;
pub(crate) use crate::board::evaluator::{MAX_EVAL_VALUE, MIN_EVAL_VALUE};
use crate::search::advanced_search::SEARCH_ABORTED;

// Iterative deepening aspiration window (in centipawns)
// With a stronger, more stable evaluator we can start tighter and cap lower.
pub(crate) const ASP_WINDOW_INIT_CP: i32 = 30; // initial aspiration half-window
pub(crate) const ASP_WINDOW_MAX_CP: i32 = 400; // maximum expanded half-window

#[inline]
pub(crate) fn aspiration_bounds_for_depth(depth_now: usize, last_score: i32, window: i32) -> (i32, i32) {
    // Use full window for very shallow depths where last_score is unreliable
    if depth_now <= 3 {
        (MIN_EVAL_VALUE + 1, MAX_EVAL_VALUE - 1)
    } else {
        (
            (last_score - window).max(MIN_EVAL_VALUE + 1),
            (last_score + window).min(MAX_EVAL_VALUE - 1),
        )
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) fn probe_with_aspiration(
    active_color: Color,
    root_moves: &Vec<((usize, usize), (usize, usize), Option<char>)>,
    depth_now: usize,
    last_score: i32,
    window: &mut i32,
    tt: &mut TranspositionTable,
    game_state: &mut GameState,
    history: &History,
) -> (((usize, usize), (usize, usize), Option<char>), i32, i32) {
    let board = game_state.board();
    let (mut a, mut b) = aspiration_bounds_for_depth(depth_now, last_score, *window);

    let mut tried = 0;
    loop {
        tried += 1;

        let (_mv, _best_adj, best_raw) = evaluate_root_for_bounds(
            active_color,
            root_moves,
            depth_now,
            a,
            b,
            tt,
            game_state,
            history,
        );

        if best_raw == SEARCH_ABORTED {
            return (((0, 0), (0, 0), None), SEARCH_ABORTED, SEARCH_ABORTED);
        }

        // Check aspiration result
        if best_raw <= a {
            // fail-low: widen down
            *window = (*window * 2).min(ASP_WINDOW_MAX_CP);
            let bounds = aspiration_bounds_for_depth(depth_now, last_score, *window);
            a = bounds.0;
            if tried < 3 { continue; }
        } else if best_raw >= b {
            // fail-high: widen up
            *window = (*window * 2).min(ASP_WINDOW_MAX_CP);
            let bounds = aspiration_bounds_for_depth(depth_now, last_score, *window);
            b = bounds.1;
            if tried < 3 { continue; }
        }
        // At this point we have tried a few widened windows but still failed to land inside bounds.
        // To ensure a stable PV update at this depth, fall back to a full-width search at once.
         {
            // Reset to the full window and a modest aspiration window for subsequent depths
            *window = (*window).max(ASP_WINDOW_INIT_CP);
            let (fa, fb) = (MIN_EVAL_VALUE + 1, MAX_EVAL_VALUE - 1);
            let (mv2, best_adj2, best_raw2) = evaluate_root_for_bounds(
                active_color,
                root_moves,
                depth_now,
                fa,
                fb,
                tt,
                game_state,
                history,
            );
            if best_raw2 == SEARCH_ABORTED {
                return (((0, 0), (0, 0), None), SEARCH_ABORTED, SEARCH_ABORTED);
            }
            return (mv2, best_adj2, best_raw2);
        }
    }
}
