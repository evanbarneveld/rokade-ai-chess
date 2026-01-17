use crate::piece::pieces::{Color, PieceType};
use crate::search::core::alphabeta::alphabeta;
use crate::search::state::tt::TranspositionTable;
use crate::search::state::rep_stack::RepetitionStack;
use crate::search::core::advanced_search::NULL_MOVE_PRUNING_ENABLED;
use crate::state::game_state::GameState;

const NULL_MOVE_PRUNING_START_DEPTH: usize = 4;

#[inline]
pub fn prune_null_moves(
    game_state: &mut GameState,
    depth: usize,
    alpha: i32,
    beta: i32,
    ply: i32,
    tt: &mut TranspositionTable,
    rep_stack: &mut RepetitionStack,
) -> Option<i32> {
    if !NULL_MOVE_PRUNING_ENABLED {
        return None;
    }
    let to_move = game_state.active_color();
    let halfmove_clock = game_state.half_move_clock();

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
            // Quick material heuristic: require presence of any piece other than kings/pawns
            let mut has_non_pawn_minor = false;
            'scan: for r in 0..8 {
                for c in 0..8 {
                    if let Some(p) = game_state.board().get(r, c)
                        && p.get_color() == to_move {
                            match p.get_type() {
                                PieceType::Knight | PieceType::Bishop | PieceType::Rook | PieceType::Queen => {
                                    has_non_pawn_minor = true;
                                    break 'scan;
                                }
                                _ => {}
                            }
                        }
                }
            }
            if has_non_pawn_minor {
                // Reduction R: slightly deeper at high depths with stronger eval
                let r: usize = if depth >= 8 { 3 } else { 2 };
                // Probe a null-window Search; use a narrow window around beta (White) or alpha (Black)
                let (null_alpha, null_beta) = if to_move == Color::White {
                    (beta - 1, beta)
                } else {
                    (alpha, alpha + 1)
                };

                // Null move: switch side, clear en passant (it expires after skipping a turn)
                game_state.switch_player_turn();
                let old_hmc = game_state.half_move_clock();
                let old_ep = game_state.en_passant_target();
                game_state.increment_half_move_clock();
                game_state.set_en_passant_target(None);

                let score = alphabeta(
                    game_state,
                    depth.saturating_sub(1 + r),
                    null_alpha,
                    null_beta,
                    ply + 1,
                    tt,
                    rep_stack,
                );

                // Unmake null move
                game_state.switch_player_turn();
                game_state.set_half_move_clock(old_hmc);
                game_state.set_en_passant_target(old_ep);
                
                if to_move == Color::White {
                    if score >= beta {
                        return Some(score); // null-move cutoff for White
                    }
                } else if score <= alpha {
                    return Some(score); // null-move cutoff for Black
                }
            }
        }
    }
    None
}
