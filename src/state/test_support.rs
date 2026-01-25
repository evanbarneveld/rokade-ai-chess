#![doc(hidden)]

use crate::state::game_state::GameState;

pub use crate::state::castling::CastlingRights;

pub fn game_state_to_fen_string(game_state: GameState) -> String {
    crate::state::fen::writer::game_state_to_fen_string(game_state)
}
