
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Copy, Clone)]
pub struct Entry {
    pub key: u64,
    pub score: i32,
    pub depth: i16,
    pub bound: Bound,
    pub best_from: u8, // 0..63
    pub best_to: u8,   // 0..63
    pub age: u8,
}

impl Default for Entry {
    fn default() -> Self {
        Entry { key: 0, score: 0, depth: -1, bound: Bound::Exact, best_from: 0, best_to: 0, age: 0 }
    }
}

pub struct TranspositionTable {
    entries: Box<[Entry]>,
    mask: usize,
    age: u8,
}

impl TranspositionTable {
    // Target ~128MB. With Entry ~24 bytes (after alignment), choose 2^22 ≈ 4,194,304 entries (~100MB).
    pub fn with_capacity_pow2(pow2: u32) -> Self {
        let size = 1usize << pow2;
        let entries = vec![Entry::default(); size].into_boxed_slice();
        Self { entries, mask: size - 1, age: 0 }
    }

    pub fn new_size_18() -> Self { Self::with_capacity_pow2(18) }

    #[inline]
    fn index(&self, key: u64) -> usize { (key as usize) & self.mask }

    #[inline]
    pub fn next_age(&mut self) { self.age = self.age.wrapping_add(1); }

    #[inline]
    pub fn probe(&self, key: u64) -> Option<&Entry> {
        let idx = self.index(key);
        let e = &self.entries[idx];
        if e.depth >= 0 && e.key == key { Some(e) } else { None }
    }

    #[inline]
    pub fn store(&mut self, key: u64, depth: i16, bound: Bound, score: i32, best_from: Option<u8>, best_to: Option<u8>) {
        let idx = self.index(key);
        let e = &mut self.entries[idx];
        // Replace if empty, deeper, or older (simple replacement scheme favoring depth)
        if e.depth < 0 || depth > e.depth || (depth == e.depth && self.age.wrapping_sub(e.age) > 8) {
            e.key = key;
            e.depth = depth;
            e.bound = bound;
            e.score = score;
            if let (Some(f), Some(t)) = (best_from, best_to) { e.best_from = f; e.best_to = t; }
            e.age = self.age;
        }
    }
}

#[inline]
pub fn encode_move(from: (usize, usize), to: (usize, usize)) -> (u8, u8) {
    let f = (from.0 as u8) * 8 + (from.1 as u8);
    let t = (to.0 as u8) * 8 + (to.1 as u8);
    (f, t)
}

#[inline]
pub fn decode_move(f: u8, t: u8) -> ((usize, usize), (usize, usize)) {
    let fr = (f / 8) as usize; let fc = (f % 8) as usize;
    let tr = (t / 8) as usize; let tc = (t % 8) as usize;
    ((fr, fc), (tr, tc))
}

// Mate score normalization helpers (store relative to ply to avoid horizon artifacts)
pub const MATE_VALUE: i32 = 30_000;
pub const MATE_TB: i32 = MATE_VALUE - 1_000; // table boundary for recognizing mate scores

#[inline]
pub fn to_tt_score(score: i32, ply: i32) -> i32 {
    if score > MATE_TB { return score + ply; }
    if score < -MATE_TB { return score - ply; }
    score
}

#[inline]
pub fn from_tt_score(score: i32, ply: i32) -> i32 {
    if score > MATE_TB { return score - ply; }
    if score < -MATE_TB { return score + ply; }
    score
}
