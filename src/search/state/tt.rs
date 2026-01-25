use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
pub(crate) use crate::board::evaluator::MATE_VALUE;

pub const TT_ENTRY_BYTES: usize = 16;
pub const DEFAULT_TT_POW2: u32 = 22;
pub const DEFAULT_HASH_MB: usize =
    ((1usize << DEFAULT_TT_POW2) * TT_ENTRY_BYTES) / (1024 * 1024);

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

/// Entry struct returned by probe() - contains the unpacked data.
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

// ============================================================
// DATA PACKING LAYOUT (64 bits)
// ============================================================
// | Bits  | Field     | Range                     |
// |-------|-----------|---------------------------|
// | 0-23  | score     | ±8M cp (offset encoded)   |
// | 24-31 | depth     | -128 to 127 (offset 128)  |
// | 32-33 | bound     | 0=Exact, 1=Lower, 2=Upper |
// | 34-39 | best_from | 0-63 (square index)       |
// | 40-45 | best_to   | 0-63 (square index)       |
// | 46-53 | age       | 0-255                     |
// | 54-63 | reserved  | future use                |
// ============================================================

const SCORE_OFFSET: i32 = 8_388_608; // 2^23 to make score positive for packing
const DEPTH_OFFSET: i16 = 128;       // to make depth positive for packing

/// Pack entry data into a u64
#[inline]
fn pack_data(score: i32, depth: i16, bound: Bound, best_from: u8, best_to: u8, age: u8) -> u64 {
    // Use i64 arithmetic to avoid overflow, then mask to 24 bits
    let score_i64 = (score as i64) + (SCORE_OFFSET as i64);
    let score_packed = (score_i64 as u64) & 0xFF_FFFF; // 24 bits

    let depth_i32 = (depth as i32) + (DEPTH_OFFSET as i32);
    let depth_packed = (depth_i32 as u64) & 0xFF;       // 8 bits

    let bound_packed = (bound as u64) & 0x3;            // 2 bits
    let from_packed = (best_from as u64) & 0x3F;        // 6 bits
    let to_packed = (best_to as u64) & 0x3F;            // 6 bits
    let age_packed = (age as u64) & 0xFF;               // 8 bits

    score_packed
        | (depth_packed << 24)
        | (bound_packed << 32)
        | (from_packed << 34)
        | (to_packed << 40)
        | (age_packed << 46)
}

/// Unpack entry data from a u64
#[inline]
fn unpack_data(data: u64) -> (i32, i16, Bound, u8, u8, u8) {
    let score_packed = (data & 0xFF_FFFF) as i32;
    let score = score_packed - SCORE_OFFSET;

    let depth_packed = ((data >> 24) & 0xFF) as i16;
    let depth = depth_packed - DEPTH_OFFSET;

    let bound_packed = ((data >> 32) & 0x3) as u8;
    let bound = match bound_packed {
        0 => Bound::Exact,
        1 => Bound::Lower,
        _ => Bound::Upper,
    };

    let best_from = ((data >> 34) & 0x3F) as u8;
    let best_to = ((data >> 40) & 0x3F) as u8;
    let age = ((data >> 46) & 0xFF) as u8;

    (score, depth, bound, best_from, best_to, age)
}

/// Atomic entry using XOR-based corruption detection.
/// Stores key XOR data in one word, and data in another.
/// If a torn read occurs, the XOR validation will fail and we treat it as a miss.
struct AtomicEntry {
    key_xor: AtomicU64, // key ^ data (for validation)
    data: AtomicU64,    // packed data
}

impl AtomicEntry {
    fn new() -> Self {
        AtomicEntry {
            key_xor: AtomicU64::new(0),
            data: AtomicU64::new(pack_data(0, -1, Bound::Exact, 0, 0, 0)),
        }
    }
}

impl Default for AtomicEntry {
    fn default() -> Self {
        Self::new()
    }
}

/// Lock-free transposition table using atomic operations.
/// Uses XOR-based corruption detection: stores (key ^ data) and data separately.
/// On read, we verify that stored_key_xor ^ stored_data == expected_key.
pub struct AtomicTranspositionTable {
    entries: Box<[AtomicEntry]>,
    mask: usize,
    age: AtomicU64,
    used_slots: AtomicUsize,
}

// Safety: AtomicEntry contains only atomic types which are Send+Sync
unsafe impl Send for AtomicTranspositionTable {}
unsafe impl Sync for AtomicTranspositionTable {}

impl AtomicTranspositionTable {
    /// Create a table with 2^pow2 entries.
    /// pow2=21 gives ~2M entries (~32MB with 16 bytes per entry).
    pub fn with_capacity_pow2(pow2: u32) -> Self {
        let size = 1usize << pow2;
        let mut entries = Vec::with_capacity(size);
        for _ in 0..size {
            entries.push(AtomicEntry::new());
        }
        Self {
            entries: entries.into_boxed_slice(),
            mask: size - 1,
            age: AtomicU64::new(0),
            used_slots: AtomicUsize::new(0),
        }
    }

    pub fn new_with_default_size() -> Self {
        Self::with_capacity_pow2(DEFAULT_TT_POW2)
    }

    #[inline]
    fn index(&self, key: u64) -> usize {
        (key as usize) & self.mask
    }

    #[inline]
    pub fn next_age(&self) {
        self.age.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    fn current_age(&self) -> u8 {
        (self.age.load(Ordering::Relaxed) & 0xFF) as u8
    }

    pub fn clear(&self) {
        let default_data = pack_data(0, -1, Bound::Exact, 0, 0, 0);
        for entry in self.entries.iter() {
            entry.key_xor.store(default_data, Ordering::Relaxed); // key=0, so key_xor = 0 ^ data = data
            entry.data.store(default_data, Ordering::Relaxed);
        }
        self.age.store(0, Ordering::Relaxed);
        self.used_slots.store(0, Ordering::Relaxed);
    }

    /// Probe the table for an entry. Returns Some(Entry) if found and valid.
    /// Uses Acquire ordering to ensure we see all writes that happened before the store.
    #[inline]
    pub fn probe(&self, key: u64) -> Option<Entry> {
        if !crate::search::advanced_search::TRANSPOSITION_TABLE_ENABLED {
            return None;
        }

        let idx = self.index(key);
        let entry = &self.entries[idx];

        // Read both words - order matters for consistency
        let data = entry.data.load(Ordering::Acquire);
        let key_xor = entry.key_xor.load(Ordering::Acquire);

        // Validate: key_xor should equal key ^ data
        let stored_key = key_xor ^ data;
        if stored_key != key {
            return None; // Key mismatch or torn read
        }

        let (score, depth, bound, best_from, best_to, age) = unpack_data(data);

        // depth < 0 means empty slot
        if depth < 0 {
            return None;
        }

        Some(Entry {
            key,
            score,
            depth,
            bound,
            best_from,
            best_to,
            age,
        })
    }

    /// Store an entry in the table.
    /// Uses Release ordering so subsequent Acquire reads see our writes.
    #[inline]
    pub fn store(
        &self,
        key: u64,
        depth: i16,
        bound: Bound,
        score: i32,
        best_from: Option<u8>,
        best_to: Option<u8>,
    ) {
        if !crate::search::advanced_search::TRANSPOSITION_TABLE_ENABLED {
            return;
        }

        let idx = self.index(key);
        let entry = &self.entries[idx];

        // Read existing entry to check replacement policy
        let old_data = entry.data.load(Ordering::Relaxed);
        let old_key_xor = entry.key_xor.load(Ordering::Relaxed);
        let (_, old_depth, _, old_best_from, old_best_to, old_age) = unpack_data(old_data);
        let old_key = old_key_xor ^ old_data;

        let current_age = self.current_age();

        // Replacement policy: replace if empty, deeper, or older
        let should_replace = old_depth < 0
            || depth > old_depth
            || (depth == old_depth && current_age.wrapping_sub(old_age) > 8);

        if !should_replace {
            return;
        }

        // Preserve best move from existing entry if not provided
        let (bf, bt) = match (best_from, best_to) {
            (Some(f), Some(t)) => (f, t),
            _ if old_key == key => (old_best_from, old_best_to),
            _ => (0, 0),
        };

        let was_empty = old_depth < 0;
        let data = pack_data(score, depth, bound, bf, bt, current_age);
        let key_xor = key ^ data;

        // Write data first, then key_xor (so readers see consistent data)
        entry.data.store(data, Ordering::Release);
        entry.key_xor.store(key_xor, Ordering::Release);

        if was_empty {
            self.used_slots.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Return UCI-style hashfull (permill 0..1000) based on approximate occupancy.
    #[inline]
    pub fn hashfull_permille(&self) -> u16 {
        if !crate::search::advanced_search::TRANSPOSITION_TABLE_ENABLED {
            return 0;
        }
        if self.entries.is_empty() {
            return 0;
        }
        let len = self.entries.len();
        let samples = len.min(1024);
        let stride = (len / samples).max(1);
        let mut used = 0usize;
        let mut idx = (self.age.load(Ordering::Relaxed) as usize) & self.mask;

        for _ in 0..samples {
            let data = self.entries[idx].data.load(Ordering::Relaxed);
            let (_, depth, _, _, _, _) = unpack_data(data);
            if depth >= 0 {
                used += 1;
            }
            idx = (idx + stride) & self.mask;
        }

        let v = (used as u128) * 1000u128 / (samples as u128);
        v.min(1000) as u16
    }
}

// ============================================================
// LEGACY ALIAS - kept for gradual migration
// ============================================================
pub type TranspositionTable = AtomicTranspositionTable;

#[inline]
pub fn pow2_for_hash_mb(mb: usize) -> u32 {
    let bytes = mb.max(1) * 1024 * 1024;
    let entries = bytes / TT_ENTRY_BYTES;
    let mut pow2: u32 = 0;
    let max_pow2 = (usize::BITS - 1) as u32;
    while pow2 < max_pow2 && (1usize << pow2) <= entries {
        pow2 += 1;
    }
    pow2.saturating_sub(1).clamp(4, 30)
}

#[inline]
pub fn encode_move(from: (usize, usize), to: (usize, usize)) -> (u8, u8) {
    let f = (from.0 as u8) * 8 + (from.1 as u8);
    let t = (to.0 as u8) * 8 + (to.1 as u8);
    (f, t)
}

#[inline]
pub fn decode_move(f: u8, t: u8) -> ((usize, usize), (usize, usize)) {
    let fr = (f / 8) as usize;
    let fc = (f % 8) as usize;
    let tr = (t / 8) as usize;
    let tc = (t % 8) as usize;
    ((fr, fc), (tr, tc))
}

// Mate score normalization helpers (store relative to ply to avoid horizon artifacts)
pub const MATE_TB: i32 = MATE_VALUE - 1_000; // table boundary for recognizing mate scores

#[inline]
pub fn to_tt_score(score: i32, ply: i32) -> i32 {
    if score > MATE_TB {
        return score + ply;
    }
    if score < -MATE_TB {
        return score - ply;
    }
    score
}

#[inline]
pub fn from_tt_score(score: i32, ply: i32) -> i32 {
    if score > MATE_TB {
        return score - ply;
    }
    if score < -MATE_TB {
        return score + ply;
    }
    score
}
