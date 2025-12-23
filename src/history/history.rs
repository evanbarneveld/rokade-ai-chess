#[derive(Debug, Default, Clone)]
pub struct History {
    plies: Vec<(String, String)>,
}

impl History {
    pub fn new() -> Self {
        Self {
            plies: (Vec::new()),
        }
    }

    // Clears all recorded moves
    pub fn reset(&mut self) {
        self.plies.clear();
    }

    // Returns a reference to the move at the given index, if it exists
    pub fn get_move(&self, index: usize) -> Option<&(String, String)> {
        self.plies.get(index)
    }

    // Adds a move to the history (SAN or other chosen notation)
    pub fn add_move(&mut self, mv: String, fen: String) {
        self.plies.push((mv, fen));
    }

    // Undoes the last move and returns it, if any
    pub fn undo_move(&mut self) -> Option<(String, String)> {
        self.plies.pop()
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
}
