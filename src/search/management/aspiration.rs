use crate::history::history::History;
use crate::piece::pieces::Color;
use crate::search::evaluation::root_evaluator::evaluate_root_for_bounds;
use crate::search::context::SearchContext;
use crate::search::evaluation::heuristics::SearchHeuristics;
use crate::search::state::tt::TranspositionTable;
use crate::state::game_state::GameState;
pub(crate) use crate::board::evaluator::{MAX_EVAL_VALUE, MIN_EVAL_VALUE};
use crate::search::core::advanced_search::SEARCH_ABORTED;

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
    ctx: &SearchContext,
    active_color: Color,
    root_moves: &Vec<((usize, usize), (usize, usize), Option<char>)>,
    depth_now: usize,
    last_score: i32,
    window: &mut i32,
    tt: &TranspositionTable,
    game_state: &mut GameState,
    history: &History,
    heuristics: &mut SearchHeuristics,
    collect_all_scores: Option<&mut Vec<(((usize, usize), (usize, usize), Option<char>), i32, i32)>>,
) -> (((usize, usize), (usize, usize), Option<char>), i32, i32) {

    let (mut a, mut b) = aspiration_bounds_for_depth(depth_now, last_score, *window);

    #[cfg(feature = "debug-search")] {
        eprintln!("[ASP] depth={} last_score={} window=±{} bounds=[{}, {}]",
            depth_now, last_score, *window, a, b);
    }

    let mut tried = 0;
    loop {
        tried += 1;

        #[cfg(feature = "debug-search")] {
            eprintln!("[ASP] attempt #{} with α={} β={}", tried, a, b);
        }

        let (_mv, _best_adj, best_raw) = evaluate_root_for_bounds(
            ctx,
            active_color,
            root_moves,
            depth_now,
            a,
            b,
            tt,
            game_state,
            history,
            heuristics,
            None, // Don't collect on intermediate aspiration attempts
        );

        if best_raw == SEARCH_ABORTED {
            return (((0, 0), (0, 0), None), SEARCH_ABORTED, SEARCH_ABORTED);
        }

        // Check aspiration result
        if best_raw <= a {
            // fail-low: widen down
            #[cfg(feature = "debug-search")] {
                eprintln!("[ASP] FAIL-LOW: score {} <= α {}, widening window", best_raw, a);
            }
            *window = (*window * 2).min(ASP_WINDOW_MAX_CP);
            let bounds = aspiration_bounds_for_depth(depth_now, last_score, *window);
            a = bounds.0;
            if tried < 3 { continue; }
            // After 3 tries still failing low, fall back to full-width search
            #[cfg(feature = "debug-search")] {
                eprintln!("[ASP] 3 fail-lows, falling back to full-width search");
            }
        } else if best_raw >= b {
            // fail-high: widen up
            #[cfg(feature = "debug-search")] {
                eprintln!("[ASP] FAIL-HIGH: score {} >= β {}, widening window", best_raw, b);
            }
            *window = (*window * 2).min(ASP_WINDOW_MAX_CP);
            let bounds = aspiration_bounds_for_depth(depth_now, last_score, *window);
            b = bounds.1;
            if tried < 3 { continue; }
            // After 3 tries still failing high, fall back to full-width search
            #[cfg(feature = "debug-search")] {
                eprintln!("[ASP] 3 fail-highs, falling back to full-width search");
            }
        } else {
            // Score is within bounds - collect scores on final successful search
            #[cfg(feature = "debug-search")] {
                eprintln!("[ASP] SUCCESS: score {} within [{}, {}] after {} attempt(s)",
                    best_raw, a, b, tried);
            }
            if collect_all_scores.is_some() {
                let (mv_final, adj_final, raw_final) = evaluate_root_for_bounds(
                    ctx,
                    active_color,
                    root_moves,
                    depth_now,
                    a,
                    b,
                    tt,
                    game_state,
                    history,
                    heuristics,
                    collect_all_scores,
                );
                return (mv_final, adj_final, raw_final);
            } else {
                return (_mv, _best_adj, best_raw);
            }
        }

        // At this point we have tried a few widened windows but still failed to land inside bounds.
        // Fall back to a full-width search to ensure a stable result.
        let (fa, fb) = (MIN_EVAL_VALUE + 1, MAX_EVAL_VALUE - 1);
        let (mv2, best_adj2, best_raw2) = evaluate_root_for_bounds(
            ctx,
            active_color,
            root_moves,
            depth_now,
            fa,
            fb,
            tt,
            game_state,
            history,
            heuristics,
            collect_all_scores, // Collect on final full-width search
        );
        if best_raw2 == SEARCH_ABORTED {
            return (((0, 0), (0, 0), None), SEARCH_ABORTED, SEARCH_ABORTED);
        }
        return (mv2, best_adj2, best_raw2);
    }
}
