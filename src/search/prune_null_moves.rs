use crate::board::Board;
use crate::piece::pieces::{opposite_color, Color, PieceType};
use crate::search::alphabeta::alphabeta;
use crate::search::tt::TranspositionTable;
use crate::search::search::NULL_MOVE_PRUNING_ENABLED;

const NULL_MOVE_PRUNING_START_DEPTH: usize = 4;

#[inline]
pub fn prune_null_moves(board: &mut Board, to_move: Color, depth: usize, beta: i32, ply: i32, tt: &mut TranspositionTable, halfmove_clock: u32, rep_stack: &mut Vec<u64>) -> Option<i32> {
    if !NULL_MOVE_PRUNING_ENABLED {
        return None;
    }
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
        let in_check = board.is_side_in_check(to_move);
        if !in_check && halfmove_clock < 100 {
            // Quick material heuristic: require presence of any piece other than kings/pawns
            let mut has_non_pawn_minor = false;
            'scan: for r in 0..8 {
                for c in 0..8 {
                    if let Some(p) = board.get(r, c) {
                        if p.get_color() == to_move {
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
            }
            if has_non_pawn_minor {
                // Reduction R: slightly deeper at high depths with stronger eval
                let r: usize = if depth >= 8 { 3 } else { 2 };
                let undo: Option<()> = None;
                // Make a null move: switch side to move without changing board, but we still push repetition key
                // We reuse halfmove_clock (null move does not reset it)
                // To keep Book/Board API simple, emulate by switching to_move only in recursive call
                // and NOT modifying the board.
                // Probe a null-window search; use (beta-1, beta) window which is standard for NMP
                let score = alphabeta(
                    board,
                    opposite_color(to_move),
                    depth.saturating_sub(1 + r),
                    beta - 1,
                    beta,
                    ply + 1,
                    tt,
                    halfmove_clock.saturating_add(1),
                    rep_stack,
                );
                let _ = undo; // placeholder for symmetry with real moves; no board change was made
                if score >= beta {
                    return Some(score); // null-move cutoff
                }
            }
        }
    }
    None
}
