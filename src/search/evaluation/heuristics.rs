use crate::piece::pieces::Color;

const HISTORY_CAP: i32 = 1_000_000; // prevent runaway history scores
const CONT_HISTORY_DIM: usize = 64;
const CONT_HISTORY_SIZE: usize = 2 * CONT_HISTORY_DIM * CONT_HISTORY_DIM * CONT_HISTORY_DIM;

// History and Killer tables for move ordering
pub struct SearchHeuristics {
    // history[side][from][to] -> score
    history: [[[i32; 64]; 64]; 2],
    // two killer moves per ply, stored as (from*8+to)
    killers: Vec<[i16; 2]>,
    // countermove for prev move: counter_moves[side][prev_from][prev_to] -> move (from*64+to)
    counter_moves: [[[i16; 64]; 64]; 2],
    // continuation history: [side][prev_to][from][to] -> score (flattened)
    continuation_history: Vec<i32>,
}

impl SearchHeuristics {
    pub fn new(max_ply: usize) -> Self {
        SearchHeuristics {
            history: [[[0; 64]; 64]; 2],
            killers: vec![[ -1, -1 ]; max_ply.max(64)],
            counter_moves: [[[-1; 64]; 64]; 2],
            continuation_history: vec![0; CONT_HISTORY_SIZE],
        }
    }
    pub fn clear(&mut self) {
        self.history = [[[0; 64]; 64]; 2];
        for k in &mut self.killers {
            *k = [-1, -1];
        }
        for side in &mut self.counter_moves {
            for row in side.iter_mut() {
                row.fill(-1);
            }
        }
        self.continuation_history.fill(0);
    }
    #[inline]
    fn idx_side(side: Color) -> usize { if let Color::White = side { 0 } else { 1 } }
    #[inline]
    fn flat(from: (usize, usize), to: (usize, usize)) -> i16 {
        let from_sq = (from.0 * 8 + from.1) as i16;
        let to_sq = (to.0 * 8 + to.1) as i16;
        from_sq * 64 + to_sq
    }
    #[inline]
    fn sq_index(sq: (usize, usize)) -> usize {
        sq.0 * 8 + sq.1
    }
    #[inline]
    fn continuation_index(side: Color, prev_to: (usize, usize), from: (usize, usize), to: (usize, usize)) -> usize {
        let s = Self::idx_side(side);
        let prev = Self::sq_index(prev_to);
        let f = Self::sq_index(from);
        let t = Self::sq_index(to);
        ((s * CONT_HISTORY_DIM + prev) * CONT_HISTORY_DIM + f) * CONT_HISTORY_DIM + t
    }

    pub fn add_killer(&mut self, ply: usize, from: (usize, usize), to: (usize, usize)) {
        if ply >= self.killers.len() { return; }
        let m = Self::flat(from, to);
        let k = &mut self.killers[ply];
        if k[0] != m {
            k[1] = k[0];
            k[0] = m;
        }
    }
    pub fn is_killer(&self, ply: usize, from: (usize, usize), to: (usize, usize)) -> bool {
        if ply >= self.killers.len() { return false; }
        let m = Self::flat(from, to);
        let k = self.killers[ply];
        k[0] == m || k[1] == m
    }
    /// Get killer moves as coordinates for the move picker
    pub fn get_killers(&self, ply: usize) -> [Option<((usize, usize), (usize, usize))>; 2] {
        if ply >= self.killers.len() {
            return [None, None];
        }
        let k = self.killers[ply];
        [
            Self::unflat(k[0]),
            Self::unflat(k[1]),
        ]
    }
    #[inline]
    fn unflat(m: i16) -> Option<((usize, usize), (usize, usize))> {
        if m < 0 { return None; }
        let from_sq = (m / 64) as usize;
        let to_sq = (m % 64) as usize;
        Some(((from_sq / 8, from_sq % 8), (to_sq / 8, to_sq % 8)))
    }
    pub fn set_counter_move(
        &mut self,
        side: Color,
        prev_from: (usize, usize),
        prev_to: (usize, usize),
        from: (usize, usize),
        to: (usize, usize),
    ) {
        let s = Self::idx_side(side);
        let pf = Self::sq_index(prev_from);
        let pt = Self::sq_index(prev_to);
        self.counter_moves[s][pf][pt] = Self::flat(from, to);
    }
    pub fn is_counter_move(
        &self,
        side: Color,
        prev_from: (usize, usize),
        prev_to: (usize, usize),
        from: (usize, usize),
        to: (usize, usize),
    ) -> bool {
        let s = Self::idx_side(side);
        let pf = Self::sq_index(prev_from);
        let pt = Self::sq_index(prev_to);
        self.counter_moves[s][pf][pt] == Self::flat(from, to)
    }
    pub fn add_history(&mut self, side: Color, from: (usize, usize), to: (usize, usize), bonus: i32) {
        let s = Self::idx_side(side);
        let f = from.0*8 + from.1; let t = to.0*8 + to.1;
        let entry = &mut self.history[s][f][t];
        *entry += bonus;
        // cap to avoid runaway values
        let cap = HISTORY_CAP;
        if *entry > cap { *entry = cap; }
        if *entry < -cap { *entry = -cap; }
    }
    pub fn add_continuation_history(
        &mut self,
        side: Color,
        prev_to: (usize, usize),
        from: (usize, usize),
        to: (usize, usize),
        bonus: i32,
    ) {
        let idx = Self::continuation_index(side, prev_to, from, to);
        let entry = &mut self.continuation_history[idx];
        *entry += bonus;
        let cap = HISTORY_CAP;
        if *entry > cap { *entry = cap; }
        if *entry < -cap { *entry = -cap; }
    }
    pub fn history_score(&self, side: Color, from: (usize, usize), to: (usize, usize)) -> i32 {
        let s = Self::idx_side(side);
        let f = from.0*8 + from.1; let t = to.0*8 + to.1;
        self.history[s][f][t]
    }
    pub fn continuation_score(
        &self,
        side: Color,
        prev_to: (usize, usize),
        from: (usize, usize),
        to: (usize, usize),
    ) -> i32 {
        let idx = Self::continuation_index(side, prev_to, from, to);
        self.continuation_history[idx]
    }
}
