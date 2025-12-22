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

    // CamelCase aliases in case external code expects these names
    pub fn getMove(&self, index: usize) -> Option<&String> { self.get_move(index) }
    pub fn addMove<S: Into<String>>(&mut self, mv: S) { self.add_move(mv) }
    pub fn undoMove(&mut self) -> Option<String> { self.undo_move() }
}
