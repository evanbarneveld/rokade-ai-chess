use crate::piece::pieces::Color;
use crate::search::core::alphabeta::alphabeta;
use crate::search::context::SearchContext;
use crate::search::evaluation::heuristics::SearchHeuristics;
use crate::search::state::tt::TranspositionTable;
use crate::search::state::rep_stack::RepetitionStack;
use crate::search::core::advanced_search::NULL_MOVE_PRUNING_ENABLED;
use crate::search::state::zobrist::zobrist_update_ep;
use crate::state::game_state::GameState;
use crate::board::evaluator::evaluate_position;

const NULL_MOVE_PRUNING_START_DEPTH: usize = 6;
const NULL_MOVE_STATIC_EVAL_MARGIN: i32 = 40;
const NULL_MOVE_ENDGAME_PHASE: i32 = 6;

#[inline]
pub fn prune_null_moves(
    ctx: &SearchContext,
    heuristics: &mut SearchHeuristics,
    game_state: &mut GameState,
    depth: usize,
    alpha: i32,
    beta: i32,
    ply: i32,
    tt: &TranspositionTable,
    rep_stack: &mut RepetitionStack,
    allow_null_move: bool,
    prev_move: Option<((usize, usize), (usize, usize))>,
) -> Option<i32> {
    if !NULL_MOVE_PRUNING_ENABLED || !allow_null_move {
        return None;
    }
    let to_move = game_state.active_color();
    let halfmove_clock = game_state.half_move_clock();
    let phase = game_state.board().game_phase_light();

    // -----------------
    // Null-move pruning
    // -----------------
    // Conditions to attempt null move:
    // - Sufficient remaining depth
    // - Side to move is not in check
    // - Halfmove clock not already at draw threshold
    // - Avoid in likely zugzwang scenarios (very low material) — here we approximate by requiring some non-pawn material
    // Null-move pruning (safer settings): require a bit more depth and cap reduction
    if depth >= NULL_MOVE_PRUNING_START_DEPTH {
        let in_check = game_state.mutable_board().is_side_in_check(to_move);
        if !in_check && halfmove_clock < 100 {
            // O(1) material check: require presence of any piece other than kings/pawns
            // to avoid zugzwang-related errors in pawn endgames
            if game_state.board().has_non_pawn_material(to_move) {
                if phase <= NULL_MOVE_ENDGAME_PHASE && depth < 8 {
                    return None;
                }

                let static_eval = evaluate_position(game_state.board(), to_move);
                let is_strong = if to_move == Color::White {
                    static_eval >= beta - NULL_MOVE_STATIC_EVAL_MARGIN
                } else {
                    static_eval <= alpha + NULL_MOVE_STATIC_EVAL_MARGIN
                };
                if !is_strong {
                    return None;
                }

                // Reduction R: scale with depth and evaluation; reduce a bit less in endgames
                let mut r: usize = 2;
                if depth >= 8 { r += 1; }
                if depth >= 11 { r += 1; }
                if depth >= 14 { r += 1; }
                if static_eval.abs() >= 800 { r += 1; }
                if phase <= NULL_MOVE_ENDGAME_PHASE { r = r.saturating_sub(1); }
                let min_r = if phase <= NULL_MOVE_ENDGAME_PHASE { 1 } else { 2 };
                r = r.clamp(min_r, 5);
                let max_r = depth.saturating_sub(2);
                if r > max_r { r = max_r; }
                // Probe a null-window Search; use a narrow window around beta (White) or alpha (Black)
                let (null_alpha, null_beta) = if to_move == Color::White {
                    (beta - 1, beta)
                } else {
                    (alpha, alpha + 1)
                };

                // Null move: switch side, clear en passant (it expires after skipping a turn)
                game_state.switch_player_turn_fast();
                let old_hmc = game_state.half_move_clock();
                let old_ep = game_state.en_passant_target();
                game_state.increment_half_move_clock();
                game_state.set_en_passant_target(None);
                game_state.set_zobrist_key(zobrist_update_ep(game_state.zobrist_key(), old_ep, None));

                let score = alphabeta(
                    ctx,
                    heuristics,
                    game_state,
                    depth.saturating_sub(1 + r),
                    null_alpha,
                    null_beta,
                    ply + 1,
                    tt,
                    rep_stack,
                    false, // Prevent consecutive null moves
                    prev_move,
                    0,
                );

                // Unmake null move
                game_state.switch_player_turn_fast();
                game_state.set_half_move_clock(old_hmc);
                game_state.set_en_passant_target(old_ep);
                game_state.set_zobrist_key(zobrist_update_ep(game_state.zobrist_key(), None, old_ep));
                
                if to_move == Color::White {
                    if score >= beta {
                        return Some(beta); // null-move cutoff for White (return bound, not raw score)
                    }
                } else if score <= alpha {
                    return Some(alpha); // null-move cutoff for Black (return bound, not raw score)
                }
            }
        }
    }
    None
}
