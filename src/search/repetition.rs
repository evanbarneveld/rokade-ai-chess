use crate::history::history::History;
use crate::piece::pieces::Color;
use crate::state::fen::writer::game_state_to_fen_string;
use crate::state::game_state::GameState;

// Root repetition-avoidance bias when a move would immediately create 3-fold
pub(crate) const REP_AVOIDANCE_BIAS_CP: i32 = 2000;

#[inline]
pub(crate) fn apply_repetition_avoidance_bias(
    mut adjusted: i32,
    game_state: &GameState,
    history: &History,
    active_color: Color,
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
    score_raw: i32,
) -> i32 {
    let mut gs = *game_state;
    gs.make_move_fast(from, to, promo);
    
    let fen = game_state_to_fen_string(gs);
    let truncated = fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ");
    let count = history.fen_repetition_count(&truncated);
    let sa = if active_color == Color::White {
        adjusted
    } else {
        -adjusted
    };
    if count >= 2 {
        // Root repetition-avoidance bias:
        // If we are winning (sa > 0), we penalize the draw to encourage continuing the game.
        // If we are losing (sa <= 0), we don't penalize it (we want the draw).
        // IN ALL CASES, we floor the score at a slight penalty (-10 cp) to ensure
        // that a draw is always preferred over a clear loss, even if root-level
        // heuristics (like self-hang) are very negative.
        if active_color == Color::White {
            if sa > 0 { adjusted -= REP_AVOIDANCE_BIAS_CP; }
            // For a draw by repetition, we want to be extremely careful not to avoid it
            // if it's our best saving grace. We floor it at -10 CP and then
            // potentially override other root bonuses if they are too optimistic.
            adjusted = adjusted.max(-10);
            if score_raw == 0 && adjusted > 0 { adjusted = 0; }
        } else {
            if sa > 0 { adjusted += REP_AVOIDANCE_BIAS_CP; }
            adjusted = adjusted.min(10);
            if score_raw == 0 && adjusted < 0 { adjusted = 0; }
        }
    }
    adjusted
}
