use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};
use crate::board::evaluation_helpers::{
    chebyshev_dist, is_piece, opponent, square_attacked_by_enemy_pawn, PawnFileCounts, taper_general,
};

const DOUBLED_PAWN_PENALTY_MG: i32 = 10;
const DOUBLED_PAWN_PENALTY_EG: i32 = 16;
const ISOLATED_PAWN_PENALTY_MG: i32 = 12;
const ISOLATED_PAWN_PENALTY_EG: i32 = 18;
const BACKWARD_PAWN_PENALTY_MG: i32 = 14;
const BACKWARD_PAWN_PENALTY_EG: i32 = 20;
const PAWN_MAJORITY_BONUS_MG: i32 = 4;
const PAWN_MAJORITY_BONUS_EG: i32 = 8;

pub fn evaluate_pawn(
    board: &Board,
    row: usize,
    col: usize,
    color: Color,
    phase: i32,
    king_w: Option<(usize, usize)>,
    king_b: Option<(usize, usize)>,
    att_w: &[[bool; 8]; 8],
    att_b: &[[bool; 8]; 8],
    pawn_counts: &PawnFileCounts,
) -> i32 {
    let mut val = 0;
    // File bonuses from a..h: a/h negative, c/f small positive, d/e strong positive
    const FILE_BONUS: [i32; 8] = [-30, -10, 10, 25, 25, 10, -10, -30];
    val += (FILE_BONUS[col] * phase) / 24;

    // Mild penalty for advanced rook pawns in opening
    if phase > 12
        && (col == 0 || col == 7)
    {
        let advancement = match color {
            Color::White => row as i32,
            Color::Black => (7 - row) as i32,
        };
        if advancement >= 3 {
            val -= (15 * phase) / 24;
        }
    }

    if is_doubled_pawn_fast(pawn_counts, row, col, color) {
        val -= taper_general(DOUBLED_PAWN_PENALTY_MG, DOUBLED_PAWN_PENALTY_EG, phase);
    }
    if is_isolated_pawn_fast(pawn_counts, col, color) {
        val -= taper_general(ISOLATED_PAWN_PENALTY_MG, ISOLATED_PAWN_PENALTY_EG, phase);
    }
    if is_backward_pawn(board, row, col, color, phase) {
        val -= taper_general(BACKWARD_PAWN_PENALTY_MG, BACKWARD_PAWN_PENALTY_EG, phase);
    }
    if is_passed_pawn(board, row, col, color) {
        val += evaluate_passed_pawn(board, row, col, color, phase, king_w, king_b, att_w, att_b);
    }
    val
}
use crate::board::evaluators::evaluate_king::is_king_in_front_of_pawn;

/// Fast doubled pawn check using precomputed pawn counts
#[inline]
pub fn is_doubled_pawn_fast(pawn_counts: &PawnFileCounts, _row: usize, col: usize, color: Color) -> bool {
    let count = match color {
        Color::White => pawn_counts.white[col],
        Color::Black => pawn_counts.black[col],
    };
    count > 1
}

/// Fast isolated pawn check using precomputed pawn counts
#[inline]
pub fn is_isolated_pawn_fast(pawn_counts: &PawnFileCounts, col: usize, color: Color) -> bool {
    let (left_has_pawn, right_has_pawn) = match color {
        Color::White => (
            col > 0 && pawn_counts.white[col - 1] > 0,
            col < 7 && pawn_counts.white[col + 1] > 0,
        ),
        Color::Black => (
            col > 0 && pawn_counts.black[col - 1] > 0,
            col < 7 && pawn_counts.black[col + 1] > 0,
        ),
    };
    !left_has_pawn && !right_has_pawn
}

// Keep old versions for backward compatibility if needed elsewhere
#[allow(dead_code)]
pub fn is_doubled_pawn(board: &Board, row: usize, col: usize, color: Color) -> bool {
    for r in 0..8 {
        if r == row { continue; }
        if is_piece(board, r, col, color, PieceType::Pawn) { return true; }
    }
    false
}

#[allow(dead_code)]
pub fn is_isolated_pawn(board: &Board, col: usize, color: Color) -> bool {
    for dc in [-1i32, 1] {
        let nc = col as i32 + dc;
        if !(0..=7).contains(&nc) { continue; }
        for r in 0..8 {
            if is_piece(board, r, nc as usize, color, PieceType::Pawn) { return false; }
        }
    }
    true
}

pub fn is_passed_pawn(board: &Board, row: usize, col: usize, color: Color) -> bool {
    let dir: i32 = if color == Color::White { 1 } else { -1 };
    let mut r = row as i32 + dir;
    while (0..8).contains(&r) {
        for dc in [-1i32, 0, 1] {
            let nc = col as i32 + dc;
            if !(0..8).contains(&nc) { continue; }
            if is_piece(board, r as usize, nc as usize, opponent(color), PieceType::Pawn) { return false; }
        }
        r += dir;
    }
    true
}

pub fn has_enemy_pawn_ahead_same_file(board: &Board, row: usize, col: usize, color: Color) -> bool {
    let dir: i32 = if color == Color::White { 1 } else { -1 };
    let mut r = row as i32 + dir;
    while (0..8).contains(&r) {
        if is_piece(board, r as usize, col, opponent(color), PieceType::Pawn) { return true; }
        r += dir;
    }
    false
}

pub fn friendly_pawn_adjacent_behind_limited(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> bool {
    // Consider only the nearest pawn on adjacent files within a distance cap depending on phase
    let cap = if phase >= 12 { 2 } else { 4 };
    for dc in [-1i32, 1] {
        let nc_i = col as i32 + dc;
        if !(0..=7).contains(&nc_i) { continue; }
        let nc = nc_i as usize;
        let mut best_dist: Option<i32> = None;
        for r in 0..8 {
            if is_piece(board, r, nc, color, PieceType::Pawn) {
                let d = match color {
                    Color::White => row as i32 - r as i32,
                    Color::Black => r as i32 - row as i32,
                };
                if d >= 0
                    && best_dist.is_none_or(|bd| d < bd)
                {
                    best_dist = Some(d);
                }
            }
        }
        if let Some(d) = best_dist
            && d <= cap
        {
            return true;
        }
    }
    false
}

pub fn is_backward_pawn(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> bool {
    if !is_piece(board, row, col, color, PieceType::Pawn) { return false; }
    if is_passed_pawn(board, row, col, color) { return false; }
    if !has_enemy_pawn_ahead_same_file(board, row, col, color) { return false; }
    let dir: i32 = if color == Color::White { 1 } else { -1 };
    let fr_i = row as i32 + dir;
    if !(0..=7).contains(&fr_i) { return false; }
    let fr = fr_i as usize;
    let front_blocked_by_enemy = matches!(board.get(fr, col), Some(p) if p.get_color() != color);
    let front_controlled_by_enemy_pawn = square_attacked_by_enemy_pawn(board, fr, col, opponent(color));
    if !(front_blocked_by_enemy || front_controlled_by_enemy_pawn) { return false; }
    if friendly_pawn_adjacent_behind_limited(board, row, col, color, phase) { return false; }
    true
}

pub fn pawn_file_counts(board: &Board) -> PawnFileCounts {
    let mut w = [0i32; 8];
    let mut b = [0i32; 8];
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c)
                && matches!(p.get_type(), PieceType::Pawn)
            {
                if p.get_color() == Color::White { w[c] += 1; } else { b[c] += 1; }
            }
        }
    }
    PawnFileCounts { white: w, black: b }
}

pub fn evaluate_passed_pawn(
    board: &Board,
    row: usize,
    col: usize,
    color: Color,
    phase: i32,
    king_w: Option<(usize, usize)>,
    king_b: Option<(usize, usize)>,
    att_w: &[[bool; 8]; 8],
    att_b: &[[bool; 8]; 8],
) -> i32 {
    let eg = 24 - phase;
    let adv: i32 = match color { Color::White => row as i32, Color::Black => (7 - row) as i32 };
    let mg_base = 4 + 3 * adv;
    let eg_base = 6 + 4 * adv;
    let mut score = (mg_base * phase + eg_base * eg) / 24;

    if has_clear_promotion_path(board, row, col, color) {
        score += (8 * eg) / 24;
    }

    let close_bonus = (adv.saturating_sub(4) * 6).max(0);
    score += (close_bonus * eg) / 24;

    let next_r_opt = match color {
        Color::White => if row < 7 { Some(row + 1) } else { None },
        Color::Black => if row > 0 { Some(row - 1) } else { None },
    };
    if let Some(nr) = next_r_opt {
        if let Some(blocker) = board.get(nr, col) {
            let block_pen = if blocker.get_color() != color {
                match blocker.get_type() {
                    PieceType::Pawn => 22,
                    PieceType::Knight | PieceType::Bishop => 18,
                    PieceType::Rook => 12,
                    PieceType::Queen => 10,
                    PieceType::King => 26,
                }
            } else {
                10
            };
            score -= (block_pen * eg) / 24;
        } else {
            let enemy_king = match color { Color::White => king_b, Color::Black => king_w };
            let mut safe_bonus = 10;
            if let Some((ek_r, ek_c)) = enemy_king {
                let dr = (ek_r as i32 - nr as i32).abs();
                let dc = (ek_c as i32 - col as i32).abs();
                let in_front = (color == Color::White && ek_r >= nr) || (color == Color::Black && ek_r <= nr);
                if dr <= 1 && dc <= 1 && in_front { safe_bonus = 0; }
            }
            let enemy = opponent(color);
            let attacked_general = match enemy { Color::White => att_w[nr][col], Color::Black => att_b[nr][col] };
            if attacked_general || square_attacked_by_enemy_pawn(board, nr, col, enemy) {
                safe_bonus = safe_bonus.min(2);
            }
            score += (safe_bonus * eg) / 24;
        }
    }

    let mut support = 0i32;
    let behind_r_opt = match color { Color::White => row.checked_sub(1), Color::Black => if row < 7 { Some(row + 1) } else { None } };
    if let Some(br) = behind_r_opt {
        for dc in [-1i32, 1] {
            let nc_i = col as i32 + dc;
            if !(0..=7).contains(&nc_i) { continue; }
            let nc = nc_i as usize;
            if let Some(p) = board.get(br, nc)
                && p.get_color() == color && matches!(p.get_type(), PieceType::Pawn)
            {
                support += 1;
            }
        }
    }
    for dc in [-1i32, 1] {
        let nc_i = col as i32 + dc;
        if !(0..=7).contains(&nc_i) { continue; }
        let nc = nc_i as usize;
        if let Some(p) = board.get(row, nc)
            && p.get_color() == color && matches!(p.get_type(), PieceType::Pawn)
        {
            support += 1;
        }
    }
    if support > 0 { score += (8 * support * eg) / 24; }

    let connected_bonus = connected_passed_pawn_bonus(board, row, col, color, phase);
    score += connected_bonus;

    if let (Some((fk_r, fk_c)), Some((ek_r, ek_c))) = (
        match color { Color::White => king_w, Color::Black => king_b },
        match color { Color::White => king_b, Color::Black => king_w },
    ) {
        let pawn_sq = (row as i32, col as i32);
        let fk_d = chebyshev_dist((fk_r as i32, fk_c as i32), pawn_sq);
        let ek_d = chebyshev_dist((ek_r as i32, ek_c as i32), pawn_sq);
        let prox = (12 - 2 * fk_d).max(0);
        score += (prox * eg) / 48;
        if is_king_in_front_of_pawn((ek_r, ek_c), row, col, color) && ek_d <= fk_d {
            let block_pen = (14 - 2 * ek_d).max(0);
            score -= (block_pen * eg) / 48;
        }
 
        // King distance ratio: bonus when own king is closer, penalty when enemy king is closer
        let dist_diff = ek_d - fk_d; // Positive = friendly king closer
        let ratio_bonus = (dist_diff * 3).clamp(-15, 15);
        score += (ratio_bonus * eg) / 24;
    }

    // Tarrasch rule: rook behind passed pawn is powerful
    score += tarrasch_rule_bonus(board, row, col, color, eg);

    let cap: i32 = 90;
    if score > cap { cap } else { score }
}

/// Tarrasch rule: Rook behind a passed pawn (either color) is powerful.
/// - Own rook behind: +12cp (endgame scaled)
/// - Enemy rook behind: -12cp
/// - Enemy rook in front (blocking on the file): -8cp
fn tarrasch_rule_bonus(board: &Board, row: usize, col: usize, color: Color, eg: i32) -> i32 {
    const ROOK_BEHIND_BONUS: i32 = 12;
    const ENEMY_ROOK_BEHIND_PENALTY: i32 = 12;
    const ENEMY_ROOK_FRONT_PENALTY: i32 = 8;

    let mut bonus = 0;

    // Determine direction "behind" the pawn (towards starting rank)
    let behind_range: Box<dyn Iterator<Item = usize>> = match color {
        Color::White => Box::new((0..row).rev()),  // Behind white pawn is lower rows
        Color::Black => Box::new((row + 1)..8),    // Behind black pawn is higher rows
    };

    // Check for rooks behind the pawn on the same file
    for r in behind_range {
        if let Some(p) = board.get(r, col) {
            if p.get_type() == PieceType::Rook {
                if p.get_color() == color {
                    // Own rook behind: good
                    bonus += (ROOK_BEHIND_BONUS * eg) / 24;
                } else {
                    // Enemy rook behind: bad
                    bonus -= (ENEMY_ROOK_BEHIND_PENALTY * eg) / 24;
                }
            }
            break; // Stop at first piece on the file behind
        }
    }

    // Check for enemy rook in front (blocking)
    let front_range: Box<dyn Iterator<Item = usize>> = match color {
        Color::White => Box::new((row + 1)..8),    // In front of white pawn is higher rows
        Color::Black => Box::new((0..row).rev()),  // In front of black pawn is lower rows
    };

    let enemy = opponent(color);
    for r in front_range {
        if let Some(p) = board.get(r, col) {
            if p.get_type() == PieceType::Rook && p.get_color() == enemy {
                // Enemy rook blocking from in front
                bonus -= (ENEMY_ROOK_FRONT_PENALTY * eg) / 24;
            }
            break; // Stop at first piece on the file in front
        }
    }

    bonus
}

fn connected_passed_pawn_bonus(
    board: &Board,
    row: usize,
    col: usize,
    color: Color,
    phase: i32,
) -> i32 {
    let mut best = 0i32;
    for dc in [-1i32, 1] {
        let nc_i = col as i32 + dc;
        if !(0..=7).contains(&nc_i) {
            continue;
        }
        let nc = nc_i as usize;
        for dr in [-1i32, 0, 1] {
            let nr_i = row as i32 + dr;
            if !(0..=7).contains(&nr_i) {
                continue;
            }
            let nr = nr_i as usize;
            if is_piece(board, nr, nc, color, PieceType::Pawn)
                && is_passed_pawn(board, nr, nc, color)
            {
                let adv_self = match color { Color::White => row as i32, Color::Black => (7 - row) as i32 };
                let adv_other = match color { Color::White => nr as i32, Color::Black => (7 - nr) as i32 };
                let mg = 8 + (adv_self + adv_other) / 2;
                let eg = mg + 6;
                let bonus = taper_general(mg, eg, phase);
                if bonus > best {
                    best = bonus;
                }
            }
        }
    }
    best
}

pub fn has_clear_promotion_path(board: &Board, row: usize, col: usize, color: Color) -> bool {
    if !is_piece(board, row, col, color, PieceType::Pawn) { return false; }
    let (start, end, step): (i32, i32, i32) = match color {
        Color::White => (row as i32 + 1, 7, 1),
        Color::Black => (row as i32 - 1, 0, -1),
    };
    let mut r = start;
    while (step > 0 && r <= end) || (step < 0 && r >= end) {
        if board.get(r as usize, col).is_some() { return false; }
        r += step;
    }
    true
}

pub fn find_passed_pawn_on_file(board: &Board, file: usize, color: Color) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for r in 0..8 {
        if let Some(p) = board.get(r, file)
            && p.get_color() == color
            && matches!(p.get_type(), PieceType::Pawn)
            && is_passed_pawn(board, r, file, color)
        {
            best = Some(match (best, color) {
                (None, _) => (r, file),
                (Some((br, _)), Color::White) => if r > br { (r, file) } else { (br, file) },
                (Some((br, _)), Color::Black) => if r < br { (r, file) } else { (br, file) },
            });
        }
    }
    best
}

pub fn is_hole_square(board: &Board, row: usize, col: usize, color: Color) -> bool {
    // A simplified hole: square in a central band not controllable by own pawn now nor by a pawn from behind on adjacent files
    // Quick current control test
    let own = color;
    let cur_ctrl = match own {
        Color::White => {
            if row > 0 {
                (col > 0 && is_piece(board, row-1, col-1, own, PieceType::Pawn)) ||
                (col < 7 && is_piece(board, row-1, col+1, own, PieceType::Pawn))
            } else { false }
        }
        Color::Black => {
            if row < 7 {
                (col > 0 && is_piece(board, row+1, col-1, own, PieceType::Pawn)) ||
                (col < 7 && is_piece(board, row+1, col+1, own, PieceType::Pawn))
            } else { false }
        }
    };
    if cur_ctrl { return false; }
    // Is there any own pawn on adjacent files behind the square that could, in principle, advance to control it?
    for dc in [-1i32, 1] {
        let nc_i = col as i32 + dc;
        if !(0..=7).contains(&nc_i) { continue; }
        let nc = nc_i as usize;
        match color {
            Color::White => {
                // any white pawn strictly behind the square on an adjacent file
                for r in 0..row { if is_piece(board, r, nc, color, PieceType::Pawn) { return false; } }
            }
            Color::Black => {
                for r in (row+1)..8 { if is_piece(board, r, nc, color, PieceType::Pawn) { return false; } }
            }
        }
    }
    true
}

pub fn is_hole_square_limited(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> bool {
    // Not a hole if currently controllable by own pawn
    if !is_hole_square(board, row, col, color) { return false; }
    // Distance caps for potential future control by an advancing pawn on adjacent files
    let cap = if phase >= 12 { 2 } else { 4 };
    for dc in [-1i32, 1] {
        let nc_i = col as i32 + dc;
        if !(0..=7).contains(&nc_i) { continue; }
        let nc = nc_i as usize;
        match color {
            Color::White => {
                // nearest white pawn strictly behind the target square on file nc
                let mut nearest: Option<usize> = None;
                for r in (0..row).rev() { if is_piece(board, r, nc, Color::White, PieceType::Pawn) { nearest = Some(r); break; } }
                if let Some(pr) = nearest {
                    let dist = row as i32 - pr as i32;
                    if dist <= cap {
                        // ensure no own pawn on same file between pr and row that blocks its advance
                        let mut blocked = false;
                        for br in (pr+1)..row { if is_piece(board, br, nc, Color::White, PieceType::Pawn) { blocked = true; break; } }
                        if !blocked { return false; }
                    }
                }
            }
            Color::Black => {
                let mut nearest: Option<usize> = None;
                for r in (row+1)..8 { if is_piece(board, r, nc, Color::Black, PieceType::Pawn) { nearest = Some(r); break; } }
                if let Some(pr) = nearest {
                    let dist = pr as i32 - row as i32;
                    if dist <= cap {
                        let mut blocked = false;
                        for br in (row+1)..pr { if is_piece(board, br, nc, Color::Black, PieceType::Pawn) { blocked = true; break; } }
                        if !blocked { return false; }
                    }
                }
            }
        }
    }
    true
}

/// Count pawn islands for a given color.
/// A pawn island is a group of pawns on adjacent files with no gaps.
/// More islands = weaker, fragmented structure.
pub fn count_pawn_islands(pawn_counts: &PawnFileCounts, color: Color) -> i32 {
    let counts = match color {
        Color::White => &pawn_counts.white,
        Color::Black => &pawn_counts.black,
    };

    let mut islands = 0;
    let mut in_island = false;

    for &count in counts.iter() {
        if count > 0 {
            if !in_island {
                islands += 1;
                in_island = true;
            }
        } else {
            in_island = false;
        }
    }

    islands
}

/// Evaluate pawn islands penalty.
/// Penalize each island beyond the first (-8 cp per extra island).
pub fn evaluate_pawn_islands(pawn_counts: &PawnFileCounts, color: Color, phase: i32) -> i32 {
    let islands = count_pawn_islands(pawn_counts, color);
    let penalty = (islands - 1).max(0) * 8;
    -(penalty * phase) / 24
}

/// Check if a pawn is protected by a friendly pawn (part of a chain).
fn is_pawn_protected(board: &Board, row: usize, col: usize, color: Color) -> bool {
    let behind_row = match color {
        Color::White => row.checked_sub(1),
        Color::Black => if row < 7 { Some(row + 1) } else { None },
    };

    if let Some(br) = behind_row {
        for dc in [-1i32, 1] {
            let nc = col as i32 + dc;
            if (0..=7).contains(&nc) && is_piece(board, br, nc as usize, color, PieceType::Pawn) {
                return true;
            }
        }
    }
    false
}

/// Check if a pawn is a chain base (protects another pawn but is not protected itself).
fn is_chain_base(board: &Board, row: usize, col: usize, color: Color) -> bool {
    if is_pawn_protected(board, row, col, color) {
        return false;
    }

    // Check if this pawn protects another pawn (making it a base)
    let front_row = match color {
        Color::White => if row < 7 { Some(row + 1) } else { None },
        Color::Black => row.checked_sub(1),
    };

    if let Some(fr) = front_row {
        for dc in [-1i32, 1] {
            let nc = col as i32 + dc;
            if (0..=7).contains(&nc) && is_piece(board, fr, nc as usize, color, PieceType::Pawn) {
                return true;
            }
        }
    }
    false
}

/// Evaluate pawn chain structure for a color.
/// - Bonus for protected pawns in chains (+6 cp per pawn, scaling with advancement)
/// - Penalty for weak chain bases that can be attacked (-10 cp)
pub fn evaluate_pawn_chains(board: &Board, color: Color, phase: i32) -> i32 {
    let mut score = 0;

    for row in 0..8 {
        for col in 0..8 {
            if !is_piece(board, row, col, color, PieceType::Pawn) {
                continue;
            }

            let advancement = match color {
                Color::White => row as i32,
                Color::Black => (7 - row) as i32,
            };

            if is_pawn_protected(board, row, col, color) {
                // Bonus for being part of a chain, scaled by advancement
                let bonus = 6 + advancement;
                score += (bonus * phase) / 24;
            } else if is_chain_base(board, row, col, color) {
                // Check if the base can be attacked by enemy pawns
                let enemy = opponent(color);
                let can_be_attacked = square_attacked_by_enemy_pawn(board, row, col, enemy);

                if can_be_attacked {
                    // Weak base penalty
                    score -= (10 * phase) / 24;
                }
            }
        }
    }

    score
}

/// Check if a pawn can capture an enemy pawn (has tension).
fn pawn_has_tension(board: &Board, row: usize, col: usize, color: Color) -> bool {
    let capture_row = match color {
        Color::White => if row < 7 { Some(row + 1) } else { None },
        Color::Black => row.checked_sub(1),
    };

    if let Some(cr) = capture_row {
        let enemy = opponent(color);
        for dc in [-1i32, 1] {
            let nc = col as i32 + dc;
            if (0..=7).contains(&nc) && is_piece(board, cr, nc as usize, enemy, PieceType::Pawn) {
                return true;
            }
        }
    }
    false
}

/// Evaluate pawn tension for a color.
/// Tension pawns (pawns that can capture enemy pawns) provide dynamic potential.
/// +5 cp per pawn with tension, with small bonus for advanced tension.
pub fn evaluate_pawn_tension(board: &Board, color: Color, phase: i32) -> i32 {
    let mut score = 0;

    for row in 0..8 {
        for col in 0..8 {
            if !is_piece(board, row, col, color, PieceType::Pawn) {
                continue;
            }

            if pawn_has_tension(board, row, col, color) {
                let advancement = match color {
                    Color::White => row as i32,
                    Color::Black => (7 - row) as i32,
                };

                // Base tension bonus + small advancement bonus
                let bonus = 5 + (advancement / 2);
                score += (bonus * phase) / 24;
            }
        }
    }

    score
}

/// Evaluate pawn storm toward enemy king.
/// Pawns advancing toward the enemy king's file (within 2 files) get a bonus.
/// More effective in middlegame and when kings are on opposite sides.
pub fn evaluate_pawn_storm(
    board: &Board,
    color: Color,
    phase: i32,
    enemy_king: Option<(usize, usize)>,
    own_king: Option<(usize, usize)>,
) -> i32 {
    let enemy_king_pos = match enemy_king {
        Some(pos) => pos,
        None => return 0,
    };

    let own_king_pos = match own_king {
        Some(pos) => pos,
        None => return 0,
    };

    let enemy_king_file = enemy_king_pos.1 as i32;
    let own_king_file = own_king_pos.1 as i32;

    // Pawn storms are most effective when kings are on opposite sides
    let kings_opposite_sides = (enemy_king_file <= 2 && own_king_file >= 5)
        || (enemy_king_file >= 5 && own_king_file <= 2);

    let storm_multiplier = if kings_opposite_sides { 2 } else { 1 };

    let mut score = 0;

    for row in 0..8 {
        for col in 0..8 {
            if !is_piece(board, row, col, color, PieceType::Pawn) {
                continue;
            }

            let file_distance = (col as i32 - enemy_king_file).abs();

            // Only consider pawns within 2 files of enemy king
            if file_distance > 2 {
                continue;
            }

            let advancement = match color {
                Color::White => row as i32,
                Color::Black => (7 - row) as i32,
            };

            // Pawns need to be advanced (at least rank 4 for white, rank 5 for black)
            if advancement < 3 {
                continue;
            }

            // Bonus based on advancement and proximity to enemy king file
            // Closer to king file = higher bonus
            let proximity_bonus = 3 - file_distance; // 3 for same file, 2 for 1 away, 1 for 2 away
            let storm_bonus = (advancement - 2) * proximity_bonus * storm_multiplier;

            score += (storm_bonus * phase) / 24;
        }
    }

    score
}

pub fn pawn_majority_bonus(pawn_counts: &PawnFileCounts, color: Color, phase: i32) -> i32 {
    let (own, opp) = match color {
        Color::White => (&pawn_counts.white, &pawn_counts.black),
        Color::Black => (&pawn_counts.black, &pawn_counts.white),
    };
    let own_q: i32 = own[0..4].iter().sum();
    let opp_q: i32 = opp[0..4].iter().sum();
    let own_k: i32 = own[4..8].iter().sum();
    let opp_k: i32 = opp[4..8].iter().sum();

    let mut bonus = 0;
    if own_q > 0 && own_q >= opp_q + 2 {
        bonus += taper_general(PAWN_MAJORITY_BONUS_MG, PAWN_MAJORITY_BONUS_EG, phase);
    }
    if own_k > 0 && own_k >= opp_k + 2 {
        bonus += taper_general(PAWN_MAJORITY_BONUS_MG, PAWN_MAJORITY_BONUS_EG, phase);
    }
    bonus
}
