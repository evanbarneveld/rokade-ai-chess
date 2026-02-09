use crate::board::Board;
use crate::piece::pieces::{opposite_color, piece_value_cp, Color, Piece, PieceType};

// Root SEE-based gating and penalties
pub const SEE_PENALTY_MIN_CP: i32 = 80; // min penalty for negative SEE (non-queen)
pub const SEE_PENALTY_MAX_CP: i32 = 300; // max penalty for negative SEE (non-queen)

// Quiescence SEE threshold: Allow slightly negative captures to avoid over-pruning
// A small tolerance prevents discarding moves that might lead to tactical complications
pub const QSEE_CAPTURE_TOLERANCE: i32 = -50;

// SEE penalty scaling by piece type
const MINOR_PAWN_ATTACK_PENALTY: i32 = 1200;

// Full Static Exchange Evaluation on destination square.
// Returns net material from the perspective of the side that just moved.
// Positive => safe/profitable, Negative => likely losing material on dest.
// This implements a full exchange sequence simulation with X-ray detection.
#[inline]
pub fn see_dest_estimate(
    board_after: &Board,
    side_just_moved: Color,
    dest: (usize, usize),
    captured_val: i32,
) -> i32 {
    let moved_piece = match board_after.get(dest.0, dest.1) {
        Some(p) => p,
        None => return captured_val,
    };

    // Simulate full exchange sequence
    let mut board = *board_after;
    let mut gain = [0i32; 32]; // Max depth of exchange sequence
    let mut depth = 0;

    gain[0] = captured_val;
    let mut attacker_side = opposite_color(side_just_moved);
    let mut target_piece = moved_piece;

    // Simulate exchanges until no more attackers
    loop {
        // Find the smallest attacker for the current side
        let attacker = find_smallest_attacker(&board, dest, attacker_side);
        if attacker.is_none() {
            break; // No more attackers, depth stays at current value
        }

        depth += 1;
        if depth >= 32 {
            break; // Safety limit
        }

        let (attacker_sq, attacker_piece) = attacker.unwrap();
        let target_val = piece_value_cp(target_piece.get_type());

        // Record the gain for this capture (alternating perspective)
        gain[depth] = -gain[depth - 1] + target_val;

        // Simulate the capture (this handles X-ray attacks naturally)
        board.set(attacker_sq.0, attacker_sq.1, None);
        board.set(dest.0, dest.1, Some(attacker_piece));

        // Update for next iteration
        target_piece = attacker_piece;
        attacker_side = opposite_color(attacker_side);

        // Stop if the king would be captured (invalid exchange)
        if target_piece.get_type() == PieceType::King {
            break;
        }
    }

    // Negamax backwards through the gain list to find the best outcome
    // At each level, the side to move chooses between stopping or continuing
    // Standard negamax formula: gain[d-1] = -max(-gain[d-1], gain[d])
    // Process from highest depth down to 1, updating gain[d-1] using gain[d]
    while depth > 0 {
        gain[depth - 1] = -(-gain[depth - 1]).max(gain[depth]);
        depth -= 1;
    }
    // If depth == 0, no exchanges occurred, gain[0] is returned as-is

    gain[0]
}

/// Find the smallest (least valuable) attacker of a square for a given side.
/// This is used to simulate the most favorable exchange sequence.
/// Returns (square, piece) of the attacker, or None if no attacker exists.
#[inline]
pub fn find_smallest_attacker(
    board: &Board,
    target: (usize, usize),
    attacker_color: Color,
) -> Option<((usize, usize), Piece)> {
    let (tr, tc) = target;
    let mut smallest: Option<((usize, usize), Piece)> = None;
    let mut smallest_val = i32::MAX;

    // Check pawns first (cheapest)
    // White pawns attack from lower rows (row-1), Black pawns attack from higher rows (row+1)
    // To find pawn attackers of target square (r, c):
    // - White pawn attackers: check (r-1, c±1)
    // - Black pawn attackers: check (r+1, c±1)
    let pawn_row_offset = match attacker_color {
        Color::White => -1,  // White pawns attack from one row below (toward higher rows)
        Color::Black => 1,   // Black pawns attack from one row above (toward lower rows)
    };

    for dc in [-1, 1] {
        let r_i32 = tr as i32 + pawn_row_offset;
        let c_i32 = tc as i32 + dc;
        // Check bounds before casting to usize to avoid wraparound
        if (0..8).contains(&r_i32) && (0..8).contains(&c_i32) {
            let r = r_i32 as usize;
            let c = c_i32 as usize;
            if let Some(p) = board.get(r, c)
                && p.get_color() == attacker_color
                && p.get_type() == PieceType::Pawn {
                    let val = piece_value_cp(PieceType::Pawn);
                    if val < smallest_val {
                        smallest_val = val;
                        smallest = Some(((r, c), p));
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
        let r_i32 = tr as i32 + dr;
        let c_i32 = tc as i32 + dc;
        if (0..8).contains(&r_i32) && (0..8).contains(&c_i32) {
            let r = r_i32 as usize;
            let c = c_i32 as usize;
            if let Some(p) = board.get(r, c)
                && p.get_color() == attacker_color
                && p.get_type() == PieceType::Knight {
                    let val = piece_value_cp(PieceType::Knight);
                    if val < smallest_val {
                        smallest_val = val;
                        smallest = Some(((r, c), p));
                    }
                }
        }
    }

    // Check bishops and queens (diagonal rays)
    for (dr, dc) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
        let attacker = scan_ray(board, target, dr, dc, attacker_color, &[PieceType::Bishop, PieceType::Queen]);
        if let Some((sq, p)) = attacker {
            let val = piece_value_cp(p.get_type());
            if val < smallest_val {
                smallest_val = val;
                smallest = Some((sq, p));
            }
        }
    }

    // Check rooks and queens (straight rays)
    for (dr, dc) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let attacker = scan_ray(board, target, dr, dc, attacker_color, &[PieceType::Rook, PieceType::Queen]);
        if let Some((sq, p)) = attacker {
            let val = piece_value_cp(p.get_type());
            if val < smallest_val {
                smallest_val = val;
                smallest = Some((sq, p));
            }
        }
    }

    // Check king (last, as it's highest value for exchange purposes)
    for dr in -1..=1 {
        for dc in -1..=1 {
            if dr == 0 && dc == 0 {
                continue;
            }
            let r_i32 = tr as i32 + dr;
            let c_i32 = tc as i32 + dc;
            if (0..8).contains(&r_i32) && (0..8).contains(&c_i32) {
                let r = r_i32 as usize;
                let c = c_i32 as usize;
                if let Some(p) = board.get(r, c)
                    && p.get_color() == attacker_color
                    && p.get_type() == PieceType::King {
                        // King is most valuable, consider it last
                        let val = 10000; // Use high value so it's only selected if no other attacker
                        if val < smallest_val {
                            smallest_val = val;
                            smallest = Some(((r, c), p));
                        }
                    }
            }
        }
    }

    smallest
}

// Scan along a ray (diagonal or straight) to find an attacker.
// Returns the first piece of the specified types along the ray.
// This naturally handles X-ray attacks as pieces are removed during exchange simulation.
#[inline]
fn scan_ray(
    board: &Board,
    from: (usize, usize),
    dr: i32,
    dc: i32,
    color: Color,
    piece_types: &[PieceType],
) -> Option<((usize, usize), Piece)> {
    let mut r = from.0 as i32 + dr;
    let mut c = from.1 as i32 + dc;

    while (0..8).contains(&r) && (0..8).contains(&c) {
        let (ur, uc) = (r as usize, c as usize);
        if let Some(p) = board.get(ur, uc) {
            if p.get_color() == color && piece_types.contains(&p.get_type()) {
                return Some(((ur, uc), p));
            }
            // Blocked by any piece
            return None;
        }
        r += dr;
        c += dc;
    }
    None
}

// ============================================================
// SEE HELPER FUNCTIONS
// ============================================================

/// Apply sign based on side (positive for White, negative for Black).
#[inline]
fn apply_for_side(v: i32, side: Color) -> i32 {
    if side == Color::White { v } else { -v }
}

/// Compute SEE estimate after simulating a move.
#[inline]
pub fn see_after(board: &Board, side: Color, to: (usize, usize), captured: Option<Piece>) -> i32 {
    let cap = captured.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
    see_dest_estimate(board, side, to, cap)
}

/// Check if a square is attacked by an opponent pawn.
#[inline]
pub fn attacked_by_pawn(board: &Board, sq: (usize, usize), attacker: Color) -> bool {
    let (r, c) = sq;
    // White pawns attack from row-1 (from lower rows toward higher rows)
    // Black pawns attack from row+1 (from higher rows toward lower rows)
    match attacker {
        Color::White => {
            // White pawn at (r-1, c±1) attacks sq at (r, c)
            (r > 0 && c > 0 && matches!(board.get(r-1, c-1), Some(p) if p.get_color() == attacker && p.get_type() == PieceType::Pawn))
            || (r > 0 && c + 1 < 8 && matches!(board.get(r-1, c+1), Some(p) if p.get_color() == attacker && p.get_type() == PieceType::Pawn))
        }
        Color::Black => {
            // Black pawn at (r+1, c±1) attacks sq at (r, c)
            (r + 1 < 8 && c > 0 && matches!(board.get(r+1, c-1), Some(p) if p.get_color() == attacker && p.get_type() == PieceType::Pawn))
            || (r + 1 < 8 && c + 1 < 8 && matches!(board.get(r+1, c+1), Some(p) if p.get_color() == attacker && p.get_type() == PieceType::Pawn))
        }
    }
}

// ============================================================
// SEE PENALTY HEURISTICS
// ============================================================

/// Calculate SEE-based penalty for checking piece attacked by pawn.
#[inline]
pub fn pawn_attacked_minor_penalty(
    post_after: &Board,
    side: Color,
    to: (usize, usize),
    moved_pt: PieceType,
) -> i32 {
    if moved_pt != PieceType::Knight && moved_pt != PieceType::Bishop {
        return 0;
    }
    let opp = opposite_color(side);
    if attacked_by_pawn(post_after, to, opp) {
        MINOR_PAWN_ATTACK_PENALTY
    } else {
        0
    }
}

/// Apply SEE-based penalties for destination square vulnerabilities.
///
/// IMPORTANT: For checking moves, we apply NO SEE penalties. The search has already
/// evaluated the full tactical consequences of the check. Applying SEE penalties to
/// checks can cause the engine to miss brilliant sacrifices where the piece appears
/// to be hanging but the check leads to a forced win (e.g., Qxb7+ Kxb7 a8=Q#).
#[inline]
pub fn apply_destination_see_penalties(
    base_board: &Board,
    post_after: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    is_capture: bool,
    moved_is_pawn: bool,
    gives_check: bool,
    _moved_is_queen: bool,
) -> i32 {
    // For checking moves: trust the search result, don't apply SEE penalties.
    // The opponent must respond to check, so static SEE is unreliable.
    // This allows brilliant sacrifices like Qxb7+! to be evaluated correctly.
    if gives_check {
        return 0;
    }

    let mut delta = 0;
    let captured = base_board.get(to.0, to.1);

    // Non-checking moves: apply SEE penalty
    let see = see_after(post_after, side, to, captured);
    if see < 0 {
        // Scale penalty by piece importance, don't cap too low for queen/rook blunders
        let moved_pt = base_board.get(from.0, from.1).map(|p| p.get_type());
        let pen = match moved_pt {
            Some(PieceType::Queen) => ((-see) * 6).clamp(600, 6000),
            Some(PieceType::Rook) => ((-see) * 4).clamp(SEE_PENALTY_MIN_CP, 3000),
            _ => (-see).clamp(SEE_PENALTY_MIN_CP, SEE_PENALTY_MAX_CP),
        };
        // Penalty makes score worse for the moving side
        // For White: reduce score (negative penalty from White's perspective)
        // For Black: increase score (positive penalty from White's perspective = worse for Black)
        delta += apply_for_side(-pen, side);
        if !is_capture && moved_is_pawn {
            delta += apply_for_side(-SEE_PENALTY_MIN_CP, side);
        }
    }

    delta
}

