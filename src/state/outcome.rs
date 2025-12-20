use crate::piece::pieces::Color;

#[derive(Debug, Clone)]
pub struct GameOutcome {
    winner: Option<Color>,
    is_stalemate: bool,
    is_threefold_repetition: bool,
    is_insufficient_material: bool,
    is_draw: bool
}
