#[derive(Debug, Default, Clone)]
pub struct History {
    plies: Vec<(String, String)>,
    // Tracks how many times each FEN has appeared in the history
    fen_counts: std::collections::HashMap<String, usize>,
}

impl History {
    pub fn new() -> Self {
        Self {
            plies: (Vec::new()),
            fen_counts: std::collections::HashMap::new(),
        }
    }

    // Clears all recorded moves
    pub fn reset(&mut self) {
        self.plies.clear();
        self.fen_counts.clear();
    }

    // Returns a reference to the move at the given index, if it exists
    pub fn get_move(&self, index: usize) -> Option<&(String, String)> {
        self.plies.get(index)
    }

    // Adds a move to the history (SAN or other chosen notation)
    pub fn add_move(&mut self, mv: String, fen: String) {
        // Truncate FEN to exclude the last two move counters (halfmove clock and fullmove number)
        // Keep only the first four fields: piece placement, active color, castling, en passant target
        let truncated_fen = fen
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>()
            .join(" ");
        // Update repetition counter for the provided FEN
        let entry = self.fen_counts.entry(truncated_fen.clone()).or_insert(0);
        *entry += 1;

        self.plies.push((mv, truncated_fen));
    }

    // Undoes the last move and returns it, if any
    pub fn undo_move(&mut self) -> Option<(String, String)> {
        if let Some((mv, fen)) = self.plies.pop() {
            if let Some(count) = self.fen_counts.get_mut(&fen) {
                if *count > 1 {
                    *count -= 1;
                } else {
                    // Remove to keep the map clean when count reaches zero
                    self.fen_counts.remove(&fen);
                }
            }
            Some((mv, fen))
        } else {
            None
        }
    }

    // Optional helpers
    pub fn len(&self) -> usize {
        self.plies.len()
    }

    // Returns a human-readable list of moves in standard move-pair notation,
    // e.g.: "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6"
    pub fn show_history(&self) -> String {
        if self.plies.is_empty() {
            return String::from("<no moves>");
        }

        let mut parts: Vec<String> = Vec::new();
        let mut move_number = 1;
        let mut i = 0;
        while i < self.plies.len() {
            let white = &self.plies[i].0;
            let segment = if i + 1 < self.plies.len() {
                let black = &self.plies[i + 1].0;
                format!("{}. {} {}", move_number, white, black)
            } else {
                // No black move yet
                format!("{}. {}", move_number, white)
            };
            parts.push(segment);

            move_number += 1;
            i += 2; // advance by a move pair
        }

        parts.join(" ") + "\n"
    }

    // Returns how many times a given FEN has appeared in the history so far
    pub fn fen_repetition_count(&self, fen: &str) -> usize {
        *self.fen_counts.get(fen).unwrap_or(&0)
    }

    // Returns the repetition count of the most recent position (FEN) in history
    // If there is no move yet, returns 0
    pub fn current_repetition_count(&self) -> usize {
        if let Some((_, fen)) = self.plies.last() {
            self.fen_repetition_count(fen)
        } else {
            0
        }
    }
}
