use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};
use crate::board::evaluator::{is_piece, square_attacked_by_enemy_pawn, PawnFileCounts, opponent, chebyshev_dist, taper_general};

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

    if is_doubled_pawn(board, row, col, color) { val -= 12; }
    if is_isolated_pawn(board, col, color) { val -= 14; }
    if is_backward_pawn(board, row, col, color) {
        val -= taper_general(phase, 22, 8);
    }
    if is_passed_pawn(board, row, col, color) {
        val += evaluate_passed_pawn(board, row, col, color, phase, king_w, king_b, att_w, att_b);
    }
    val
}
use crate::board::evaluators::evaluate_king::is_king_in_front_of_pawn;

pub fn is_doubled_pawn(board: &Board, row: usize, col: usize, color: Color) -> bool {
    for r in 0..8 {
        if r == row { continue; }
        if is_piece(board, r, col, color, PieceType::Pawn) { return true; }
    }
    false
}

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

pub fn is_backward_pawn(board: &Board, row: usize, col: usize, color: Color) -> bool {
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
    // Use a conservative phase cap (MG≤2, EG≤4). Without phase here, approximate with board phase.
    let phase = crate::board::evaluator::game_phase(board);
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
        if board.get(nr, col).is_some() {
            score -= (14 * eg) / 24;
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
    }

    let cap: i32 = 90;
    if score > cap { cap } else { score }
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
