use crate::history::history::History;
use crate::piece::pieces::Color;
use crate::state::game_state::GameState;

// Root repetition-avoidance bias when a move would immediately create 3-fold
pub(crate) const REP_AVOIDANCE_BIAS_CP: i32 = 2000;

#[inline]
pub(crate) fn apply_repetition_avoidance_bias(
    mut adjusted: i32,
    game_state: &GameState,
    history: &History,
    _active_color: Color,
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
    score_raw: i32,
) -> i32 {
    let mut gs = *game_state;
    gs.make_move_fast(from, to, promo);

    // Use zobrist key instead of expensive FEN string generation
    let zobrist_key = gs.zobrist_key();
    let count = history.zobrist_repetition_count(zobrist_key);

    if count >= 2 {
        // Root repetition-avoidance bias:
        // If we are winning (adjusted > 0), penalize the draw to encourage continuing.
        // If we are losing (adjusted <= 0), don't penalize it (we want the draw).
        // Clamp the score to a small range [-10, +10] to ensure a draw by repetition
        // is always preferred over a clear loss.

        if adjusted > 0 {
            // Winning position - discourage repetition
            adjusted -= REP_AVOIDANCE_BIAS_CP;
        }

        // Clamp to [-10, +10] range to ensure draw preference over clear loss
        adjusted = adjusted.clamp(-10, 10);

        // If the raw search score is 0 (draw/even), don't let heuristics make it non-zero.
        // This prevents root-level heuristics from being overly optimistic about drawn positions.
        if score_raw == 0 {
            adjusted = 0;
        }
    }
    adjusted
}
