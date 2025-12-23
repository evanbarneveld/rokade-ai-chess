use crate::parser::pgn_document::PGNDocument;

pub struct PgnPlayer {}

impl PgnPlayer {
    pub fn play(path: &str, game: &mut crate::Chess) -> Result<(), String> {
        match PGNDocument::from_file(path) {
            Ok(mut doc) => {
                while let Some(mv) = doc.next_move() {
                    if !game.move_piece_san(&mv) {
                        let msg = format!("Illegal or invalid move from PGN: '{}'. Stopping pgn replay.\n", mv);
                        return Err(msg);
                    }
                }
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}
