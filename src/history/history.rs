#[derive(Debug, Default, Clone)]
pub struct History {
    plies: Vec<(String, ((usize, usize),(usize, usize)), String, String, u64)>, // san, move, full_fen, truncated_fen, zobrist
    starting_truncated_fen: Option<String>,
    starting_zobrist: Option<u64>,
    // Tracks how many times each FEN has appeared in the history
    fen_counts: std::collections::HashMap<String, usize>,
}

impl History {
    pub fn new() -> Self {
        Self {
            plies: (Vec::new()),
            fen_counts: std::collections::HashMap::new(),
            starting_truncated_fen: None,
            starting_zobrist: None,
        }
    }

    pub fn set_starting_position(&mut self, fen: String, zobrist: u64) {
        let truncated = fen
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>()
            .join(" ");
        self.starting_truncated_fen = Some(truncated.clone());
        self.starting_zobrist = Some(zobrist);
        *self.fen_counts.entry(truncated).or_insert(0) += 1;
    }

    // Clears all recorded moves
    pub fn reset(&mut self) {
        self.plies.clear();
        self.fen_counts.clear();
        self.starting_truncated_fen = None;
        self.starting_zobrist = None;
    }

    // Returns a reference to the move at the given index, if it exists
    pub fn get_move(&self, index: usize) -> Option<&(String, ((usize, usize),(usize, usize)), String, String, u64)> {
        self.plies.get(index)
    }

    pub fn get_last_move(&self) -> String {
        self.get_move(self.len() - 1).map(|mv| mv.0.clone()).unwrap_or_else(|| String::from("No moves have been made yet"))
    }

    // Adds a move to the history (SAN or other chosen notation)
    pub fn add_move(&mut self, mv: String, board_move:((usize, usize), (usize, usize)), fen: String, zobrist: u64) {
        // Truncate FEN to exclude the last two move counters (half move clock and full move number)
        // Keep only the first four fields: piece placement, active color, castling, en passant target
        let truncated_fen = fen
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>()
            .join(" ");
        // Update repetition counter for the provided FEN
        let entry = self.fen_counts.entry(truncated_fen.clone()).or_insert(0);
        *entry += 1;
        self.plies.push((mv, board_move, fen ,truncated_fen, zobrist));
    }

    // Undoes the last move and returns it, if any
    pub fn undo_move(&mut self) -> Option<(String, String, String, u64)> {
        if let Some((mv, _, fen, truncated_fen, zobrist)) = self.plies.pop() {
            if let Some(count) = self.fen_counts.get_mut(&truncated_fen) {
                if *count > 1 {
                    *count -= 1;
                } else {
                    // Remove to keep the map clean when count reaches zero
                    self.fen_counts.remove(&truncated_fen);
                }
            }
            Some((mv, fen, truncated_fen, zobrist))
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
    pub fn fen_repetition_count(&self, truncated_fen: &str) -> usize {
        *self.fen_counts.get(truncated_fen).unwrap_or(&0)
    }

    // Returns the repetition count of the most recent position (FEN) in history
    // If there is no move yet, returns 0
    pub fn current_repetition_count(&self) -> usize {
        if let Some((_, _, _, truncated_fen, _)) = self.plies.last() {
            self.fen_repetition_count(truncated_fen)
        } else {
            0
        }
    }

    pub fn get_rep_stack(&self) -> Vec<u64> {
        let mut stack = Vec::with_capacity(self.plies.len() + 1);
        if let Some(h) = self.starting_zobrist {
            stack.push(h);
        }
        for p in &self.plies {
            stack.push(p.4);
        }
        stack
    }
}
