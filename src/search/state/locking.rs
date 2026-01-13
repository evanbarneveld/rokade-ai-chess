use std::sync::{Mutex, OnceLock};
use crate::search::state::tt::TranspositionTable;

pub static TT_CELL: OnceLock<Mutex<TranspositionTable>> = OnceLock::new();

pub fn get_tt_mutex() -> &'static Mutex<TranspositionTable> {
    TT_CELL.get_or_init(|| Mutex::new(TranspositionTable::new_with_default_size()))
}
