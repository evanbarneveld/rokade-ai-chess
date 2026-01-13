//! Root-level heuristic modules for move scoring adjustments.
//!
//! These heuristics are applied at the root of the search tree to fine-tune
//! move ordering and evaluation beyond the raw alphabeta scores.

pub mod utils;
pub mod knight_evacuation;
pub mod threat_resolution;
pub mod endgame_scaling;
pub mod king_safety;
pub mod check_hanging;
pub mod queen_pressure;

// Re-export commonly used items
pub use utils::simulate_move;
pub use knight_evacuation::knight_evacuations_priority;
pub use threat_resolution::threat_resolution_and_evacuation;
pub use endgame_scaling::endgame_50move_scaling;
pub use king_safety::king_safety_root_heuristics;
pub use check_hanging::self_hang_or_check_mobility;
pub use queen_pressure::queen_kingside_pressure_bonus;
