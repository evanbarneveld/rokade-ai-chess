use crate::piece::pieces::Color;

const HISTORY_CAP: i32 = 1_000_000; // prevent runaway history scores

// History and Killer tables for move ordering
pub struct SearchHeuristics {
    // history[side][from][to] -> score
    history: [[[i32; 64]; 64]; 2],
    // two killer moves per ply, stored as (from*8+to)
    killers: Vec<[i16; 2]>,
}

impl SearchHeuristics {
    pub fn new(max_ply: usize) -> Self {
        SearchHeuristics {
            history: [[[0; 64]; 64]; 2],
            killers: vec![[ -1, -1 ]; max_ply.max(64)],
        }
    }
    pub fn clear(&mut self) {
        self.history = [[[0; 64]; 64]; 2];
        for k in &mut self.killers {
            *k = [-1, -1];
        }
    }
    #[inline]
    fn idx_side(side: Color) -> usize { if let Color::White = side { 0 } else { 1 } }
    #[inline]
    fn flat(from: (usize, usize), to: (usize, usize)) -> i16 { ((from.0*8 + from.1)*8 + (to.0*8 + to.1)) as i16 }

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
    pub fn history_score(&self, side: Color, from: (usize, usize), to: (usize, usize)) -> i32 {
        let s = Self::idx_side(side);
        let f = from.0*8 + from.1; let t = to.0*8 + to.1;
        self.history[s][f][t]
    }
}
