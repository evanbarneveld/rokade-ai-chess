#[derive(Debug, Default, Clone)]
pub struct History {
    moves: Vec<String>,
}

impl History {
    pub fn new() -> Self {
        Self { moves: Vec::new() }
    }

    // Clears all recorded moves
    pub fn reset(&mut self) {
        self.moves.clear();
    }

    // Returns a reference to the move at the given index, if it exists
    pub fn get_move(&self, index: usize) -> Option<&String> {
        self.moves.get(index)
    }

    // Adds a move to the history (SAN or other chosen notation)
    pub fn add_move<S: Into<String>>(&mut self, mv: S) {
        self.moves.push(mv.into());
    }

    // Undoes the last move and returns it, if any
    pub fn undo_move(&mut self) -> Option<String> {
        self.moves.pop()
    }

    // Optional helpers
    pub fn len(&self) -> usize { self.moves.len() }
    pub fn is_empty(&self) -> bool { self.moves.is_empty() }

    // Returns a human-readable list of moves in standard move-pair notation,
    // e.g.: "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6"
    pub fn show_history(&self) -> String {
        if self.moves.is_empty() {
            return String::from("<no moves>");
        }

        let mut parts: Vec<String> = Vec::new();
        let mut move_number = 1;
        let mut i = 0;
        while i < self.moves.len() {
            let white = &self.moves[i];
            let segment = if i + 1 < self.moves.len() {
                let black = &self.moves[i + 1];
                format!("{}. {} {}", move_number, white, black)
            } else {
                // No black move yet
                format!("{}. {}", move_number, white)
            };
            parts.push(segment);

            move_number += 1;
            i += 2; // advance by a move pair
        }

        parts.join(" ")
    }

    // Returns a read-only view of the raw move list
    pub fn moves(&self) -> &[String] { &self.moves }

    // CamelCase aliases in case external code expects these names
    pub fn getMove(&self, index: usize) -> Option<&String> { self.get_move(index) }
    pub fn addMove<S: Into<String>>(&mut self, mv: S) { self.add_move(mv) }
    pub fn undoMove(&mut self) -> Option<String> { self.undo_move() }
    pub fn showHistory(&self) -> String { self.show_history() }
}
