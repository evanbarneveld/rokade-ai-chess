use std::collections::HashMap;

/// A stack for repetition detection with O(1) contains check.
/// Uses a HashMap to count occurrences, supporting duplicate keys from game history.
#[derive(Clone)]
pub struct RepetitionStack {
    counts: HashMap<u64, u32>,
    stack: Vec<u64>, // Maintains order for proper push/pop semantics
}

impl RepetitionStack {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
            stack: Vec::new(),
        }
    }

    /// Create from a Vec of Zobrist keys (e.g., from game history)
    pub fn from_vec(keys: Vec<u64>) -> Self {
        let mut counts = HashMap::with_capacity(keys.len());
        for &key in &keys {
            *counts.entry(key).or_insert(0) += 1;
        }
        Self { counts, stack: keys }
    }

    /// Check if a key exists in the stack (O(1))
    #[inline]
    pub fn contains(&self, key: &u64) -> bool {
        self.counts.get(key).is_some_and(|&c| c > 0)
    }

    /// Return how many times a key appears in the stack.
    #[inline]
    pub fn count(&self, key: u64) -> u32 {
        *self.counts.get(&key).unwrap_or(&0)
    }

    /// Return the most recent key, if any.
    #[inline]
    pub fn last(&self) -> Option<u64> {
        self.stack.last().copied()
    }

    /// Push a key onto the stack
    #[inline]
    pub fn push(&mut self, key: u64) {
        *self.counts.entry(key).or_insert(0) += 1;
        self.stack.push(key);
    }

    /// Pop the last key from the stack
    #[inline]
    pub fn pop(&mut self) -> Option<u64> {
        if let Some(key) = self.stack.pop() {
            if let Some(count) = self.counts.get_mut(&key) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.counts.remove(&key);
                }
            }
            Some(key)
        } else {
            None
        }
    }
}

impl Default for RepetitionStack {
    fn default() -> Self {
        Self::new()
    }
}
