use rand::{rng, Rng};

pub const PLAYING_STRENGTH_MAX: usize = 1000;

// Helper function containing common strength-based selection logic
// Returns the selected index in the sorted moves list
fn select_move_index(list_len: usize, playing_strength: usize) -> usize {
    // Clamp strength to [1..1000]
    let ps = if playing_strength == 0 {
        1
    } else {
        playing_strength.min(1000)
    };

    // Error probability: there's a chance (greater when less skilled) to intentionally choose from the end of the options.
    // This introduces natural errors when the skill level is low.
    let blunder_chance = if ps >= 950 {
        0.0
    } else if ps >= 800 {
        0.01
    } else if ps >= 650 {
        0.03
    } else if ps >= 500 {
        0.05
    } else if ps >= 350 {
        0.10
    } else {
        0.18
    };
    let roll: f32 = rng().random::<f32>();
    if roll < blunder_chance {
        // pick from the bottom bucket (worst moves), limited to 30% of a list but at least 2 moves
        let bucket = (list_len as f32 * 0.30).ceil() as usize;
        let bucket = bucket.max(2).min(list_len);
        let start = list_len - bucket;
        return rng().random_range(start..list_len);
    }

    // Choose from top-K based on strength. For low strength pick from a wider bucket,
    // but still bias the pick toward the best move within that bucket.
    // Map strength to K in [len, 1] roughly: strong -> pick among top 1..3, weak -> wider.
    // Limit randomness to the top 6 to avoid clearly dubious opening moves surfacing too often.
    let max_bucket = list_len.min(6);
    let k = if ps >= 950 {
        1
    } else if ps >= 800 {
        2
    } else if ps >= 650 {
        3
    } else if ps >= 500 {
        4
    } else if ps >= 350 {
        5
    } else if ps >= 200 {
        6
    } else {
        8
    };
    let k = k.min(max_bucket).max(1);

    // Random index within top-k, biased toward 0 (the best move).
    // Use the minimum of two uniform draws to skew toward lower indices.
    let r1: usize = rng().random_range(0..k);
    let r2: usize = rng().random_range(0..k);
    r1.min(r2)
}

// Controlled by the strength parameter, the Search will not always return the best move.
// Selects randomly among the best-scoring moves in a sorted (ascending) move table.
pub fn select_move_based_using_strength(
    sorted_moves: &Vec<((usize, usize), (usize, usize), i32)>,
    playing_strength: usize,
    deterministic: bool,
) -> Option<((usize, usize), (usize, usize))> {
    if sorted_moves.is_empty() {
        return None;
    }

    // Deterministic mode: always pick the best move (index 0) from the already sorted list.
    if deterministic {
        let pick = &sorted_moves[0];
        return Some((pick.0, pick.1));
    }

    let idx = select_move_index(sorted_moves.len(), playing_strength);
    let pick = &sorted_moves[idx];
    Some((pick.0, pick.1))
}

pub fn select_move_based_using_strength_promo(
    sorted_moves: &Vec<((usize, usize), (usize, usize), Option<char>, i32)>,
    playing_strength: usize,
    deterministic: bool,
) -> Option<((usize, usize), (usize, usize), Option<char>)> {
    if sorted_moves.is_empty() {
        return None;
    }

    // Deterministic mode: always pick the best move (index 0) from the already sorted list.
    if deterministic {
        let pick = &sorted_moves[0];
        return Some((pick.0, pick.1, pick.2));
    }

    let idx = select_move_index(sorted_moves.len(), playing_strength);
    let pick = &sorted_moves[idx];
    Some((pick.0, pick.1, pick.2))
}

// Map strength to evaluation noise (centipawns). 0 at 1000, higher at low strengths.
#[inline]
pub fn strength_noise_sigma(ps: usize) -> i32 {
    let ps = ps.min(1000).max(1) as i32;
    // Piecewise linear: ~200cp at ps=1, ~120cp at ps=300, ~0 at 1000
    let sigma = if ps >= 1000 {
        0
    } else if ps >= 700 {
        ((1000 - ps) as f32 * 0.10) as i32
    }
    // up to ~30cp
    else if ps >= 400 {
        ((700 - ps) as f32 * 0.20 + 30.0) as i32
    }
    // ~30..90
    else {
        ((400 - ps) as f32 * 0.30 + 90.0) as i32
    }; // up to ~210
    sigma.max(0)
}

