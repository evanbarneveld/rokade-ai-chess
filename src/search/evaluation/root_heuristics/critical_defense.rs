//! Critical square defense heuristics.
//!
//! Detects threats to critical squares (f7 for Black, f2 for White) and
//! rewards moves that defend these squares or block attacks on them.

use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::piece::pieces::{opposite_color, Color, PieceType};

use super::utils::apply_for_side;

/// Bonus for defending f7/f2 when under multiple attacks
const CRITICAL_DEFENSE_BONUS: i32 = 400;

/// Additional bonus when blocking an attack on the critical square
const BLOCKING_BONUS: i32 = 200;

/// Detect if a move defends or blocks attacks on critical squares.
/// f7 is critical for Black (weak square in the opening/middlegame).
/// f2 is critical for White.
#[inline]
pub fn critical_square_defense_bonus(
    base_board: &Board,
    post_after: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
) -> i32 {
    // Determine the critical square for the side to move
    let critical_sq = match side {
        Color::Black => (6, 5), // f7 for Black (row 6 = rank 7, col 5 = f-file)
        Color::White => (1, 5), // f2 for White (row 1 = rank 2, col 5 = f-file)
    };

    // Check if the critical square is under attack before our move
    let mut base_clone = *base_board;
    if !is_square_attacked_by_opponent(&mut base_clone, critical_sq, side) {
        return 0; // Critical square is not under attack
    }

    // Count attackers on the critical square before our move
    let attackers_before = count_attackers(base_board, critical_sq, opposite_color(side));
    if attackers_before <= 1 {
        return 0; // Only one attacker - not a critical threat
    }

    let mut bonus = 0;

    // Check if our move adds a defender to the critical square
    let defenders_before = count_defenders_for_piece_type(base_board, critical_sq, side, to);
    let defenders_after = count_defenders_for_piece_type(post_after, critical_sq, side, to);

    if defenders_after > defenders_before {
        // We added a defender to the critical square
        bonus += CRITICAL_DEFENSE_BONUS;
    }

    // Check if our move blocks an attack on the critical square
    // This happens if the move places a piece between an attacker and the critical square
    if blocks_attack_on_critical(base_board, post_after, critical_sq, from, to, side) {
        bonus += BLOCKING_BONUS;
    }

    apply_for_side(bonus, side)
}

/// Count the number of attackers on a square from a given color.
#[inline]
fn count_attackers(board: &Board, sq: (usize, usize), attacker_color: Color) -> i32 {
    let mut count = 0;
    let (r, c) = sq;

    // Check pawns
    let pawn_row_offset = match attacker_color {
        Color::White => -1,
        Color::Black => 1,
    };
    for dc in [-1i32, 1] {
        let pr = r as i32 + pawn_row_offset;
        let pc = c as i32 + dc;
        if pr >= 0 && pr < 8 && pc >= 0 && pc < 8 {
            if let Some(p) = board.get(pr as usize, pc as usize) {
                if p.get_color() == attacker_color && p.get_type() == PieceType::Pawn {
                    count += 1;
                }
            }
        }
    }

    // Check knights
    let knight_moves = [
        (-2, -1), (-2, 1), (-1, -2), (-1, 2),
        (1, -2), (1, 2), (2, -1), (2, 1),
    ];
    for (dr, dc) in knight_moves {
        let nr = r as i32 + dr;
        let nc = c as i32 + dc;
        if nr >= 0 && nr < 8 && nc >= 0 && nc < 8 {
            if let Some(p) = board.get(nr as usize, nc as usize) {
                if p.get_color() == attacker_color && p.get_type() == PieceType::Knight {
                    count += 1;
                }
            }
        }
    }

    // Check bishops/queens on diagonals
    for (dr, dc) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
        if let Some(_attacker) = scan_ray_for_attacker(board, sq, dr, dc, attacker_color, &[PieceType::Bishop, PieceType::Queen]) {
            count += 1;
        }
    }

    // Check rooks/queens on ranks/files
    for (dr, dc) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        if let Some(_attacker) = scan_ray_for_attacker(board, sq, dr, dc, attacker_color, &[PieceType::Rook, PieceType::Queen]) {
            count += 1;
        }
    }

    // Check king
    for dr in -1..=1i32 {
        for dc in -1..=1i32 {
            if dr == 0 && dc == 0 {
                continue;
            }
            let kr = r as i32 + dr;
            let kc = c as i32 + dc;
            if kr >= 0 && kr < 8 && kc >= 0 && kc < 8 {
                if let Some(p) = board.get(kr as usize, kc as usize) {
                    if p.get_color() == attacker_color && p.get_type() == PieceType::King {
                        count += 1;
                    }
                }
            }
        }
    }

    count
}

/// Count defenders of a square, considering that a piece might have moved to/from a location.
#[inline]
fn count_defenders_for_piece_type(board: &Board, sq: (usize, usize), defender_color: Color, moved_to: (usize, usize)) -> i32 {
    let mut count = 0;
    let (r, c) = sq;

    // Check if the piece that just moved defends the square
    if let Some(p) = board.get(moved_to.0, moved_to.1) {
        if p.get_color() == defender_color {
            let pt = p.get_type();
            let (mr, mc) = moved_to;
            let dr = r as i32 - mr as i32;
            let dc = c as i32 - mc as i32;

            let defends = match pt {
                PieceType::Pawn => {
                    // Pawns defend diagonally forward
                    let pawn_dir = match defender_color {
                        Color::White => 1,
                        Color::Black => -1,
                    };
                    dr == pawn_dir && dc.abs() == 1
                }
                PieceType::Knight => {
                    (dr.abs() == 2 && dc.abs() == 1) || (dr.abs() == 1 && dc.abs() == 2)
                }
                PieceType::Bishop => {
                    dr.abs() == dc.abs() && dr != 0 && is_clear_diagonal(board, moved_to, sq)
                }
                PieceType::Rook => {
                    (dr == 0 || dc == 0) && (dr != 0 || dc != 0) && is_clear_line(board, moved_to, sq)
                }
                PieceType::Queen => {
                    (dr.abs() == dc.abs() && dr != 0 && is_clear_diagonal(board, moved_to, sq)) ||
                    ((dr == 0 || dc == 0) && (dr != 0 || dc != 0) && is_clear_line(board, moved_to, sq))
                }
                PieceType::King => {
                    dr.abs() <= 1 && dc.abs() <= 1 && (dr != 0 || dc != 0)
                }
            };

            if defends {
                count += 1;
            }
        }
    }

    count
}

/// Check if a diagonal is clear between two squares.
#[inline]
fn is_clear_diagonal(board: &Board, from: (usize, usize), to: (usize, usize)) -> bool {
    let dr = if to.0 > from.0 { 1i32 } else { -1 };
    let dc = if to.1 > from.1 { 1i32 } else { -1 };
    let mut r = from.0 as i32 + dr;
    let mut c = from.1 as i32 + dc;

    while (r as usize, c as usize) != to {
        if board.get(r as usize, c as usize).is_some() {
            return false;
        }
        r += dr;
        c += dc;
    }
    true
}

/// Check if a line (rank/file) is clear between two squares.
#[inline]
fn is_clear_line(board: &Board, from: (usize, usize), to: (usize, usize)) -> bool {
    let dr = (to.0 as i32 - from.0 as i32).signum();
    let dc = (to.1 as i32 - from.1 as i32).signum();
    let mut r = from.0 as i32 + dr;
    let mut c = from.1 as i32 + dc;

    while (r as usize, c as usize) != to {
        if board.get(r as usize, c as usize).is_some() {
            return false;
        }
        r += dr;
        c += dc;
    }
    true
}

/// Scan a ray to find an attacker of a specific type.
#[inline]
fn scan_ray_for_attacker(
    board: &Board,
    from: (usize, usize),
    dr: i32,
    dc: i32,
    color: Color,
    piece_types: &[PieceType],
) -> Option<(usize, usize)> {
    let mut r = from.0 as i32 + dr;
    let mut c = from.1 as i32 + dc;

    while r >= 0 && r < 8 && c >= 0 && c < 8 {
        let (ur, uc) = (r as usize, c as usize);
        if let Some(p) = board.get(ur, uc) {
            if p.get_color() == color && piece_types.contains(&p.get_type()) {
                return Some((ur, uc));
            }
            return None; // Blocked by a piece
        }
        r += dr;
        c += dc;
    }
    None
}

/// Check if the move blocks an attack on the critical square.
#[inline]
fn blocks_attack_on_critical(
    base_board: &Board,
    post_after: &Board,
    critical_sq: (usize, usize),
    _from: (usize, usize),
    to: (usize, usize),
    side: Color,
) -> bool {
    let opp = opposite_color(side);
    
    // Check if we moved onto the line between an attacker and the critical square
    // Only check diagonal/straight rays (not knights or pawns)

    // Check diagonals
    for (dr, dc) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
        // Is there an opponent bishop/queen attacking the critical square via this diagonal?
        if let Some(attacker_sq) = scan_ray_for_attacker(base_board, critical_sq, dr, dc, opp, &[PieceType::Bishop, PieceType::Queen]) {
            // Check if our piece moved onto this diagonal between attacker and critical
            if is_on_line_between(attacker_sq, critical_sq, to) {
                // Verify the attack is now blocked in post_after
                if scan_ray_for_attacker(post_after, critical_sq, dr, dc, opp, &[PieceType::Bishop, PieceType::Queen]).is_none() {
                    return true;
                }
            }
        }
    }

    // Check ranks/files
    for (dr, dc) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        if let Some(attacker_sq) = scan_ray_for_attacker(base_board, critical_sq, dr, dc, opp, &[PieceType::Rook, PieceType::Queen]) {
            if is_on_line_between(attacker_sq, critical_sq, to) {
                if scan_ray_for_attacker(post_after, critical_sq, dr, dc, opp, &[PieceType::Rook, PieceType::Queen]).is_none() {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if a square is on the line between two other squares.
#[inline]
fn is_on_line_between(a: (usize, usize), b: (usize, usize), point: (usize, usize)) -> bool {
    let (ar, ac) = (a.0 as i32, a.1 as i32);
    let (br, bc) = (b.0 as i32, b.1 as i32);
    let (pr, pc) = (point.0 as i32, point.1 as i32);

    // Check if point is on the line defined by a and b
    let dr = br - ar;
    let dc = bc - ac;

    // Check if point is collinear
    let cross = (pr - ar) * dc - (pc - ac) * dr;
    if cross != 0 {
        return false;
    }

    // Check if point is between a and b
    if dr != 0 {
        let t = (pr - ar) as f32 / dr as f32;
        t > 0.0 && t < 1.0
    } else if dc != 0 {
        let t = (pc - ac) as f32 / dc as f32;
        t > 0.0 && t < 1.0
    } else {
        false
    }
}
