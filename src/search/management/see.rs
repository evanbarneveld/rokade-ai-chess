use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
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

/// Find the king square on a board for a given color.
#[inline]
fn find_king_square(board: &Board, color: Color) -> Option<(usize, usize)> {
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c)
                && p.get_color() == color && p.get_type() == PieceType::King {
                    return Some((r, c));
                }
        }
    }
    None
}

/// Check if two squares are adjacent (within 1 step in any direction).
#[inline]
fn squares_adjacent(a: (usize, usize), b: (usize, usize)) -> bool {
    let dr = a.0.abs_diff(b.0);
    let dc = a.1.abs_diff(b.1);
    dr <= 1 && dc <= 1
}

// ============================================================
// SEE PENALTY HEURISTICS
// ============================================================

/// Check if opponent king can safely capture a piece on the destination square.
#[inline]
pub fn king_can_safely_capture(
    post_after: &Board,
    side: Color,
    to: (usize, usize),
    moved_pt: PieceType,
) -> Option<i32> {
    let opp = opposite_color(side);

    // Find opponent king
    let king_sq = find_king_square(post_after, opp)?;

    // King can only capture if adjacent
    if !squares_adjacent(king_sq, to) {
        return None;
    }

    // Ensure destination holds our piece
    let on_to = post_after.get(to.0, to.1)?;
    if on_to.get_color() != side {
        return None;
    }

    // Simulate king capture
    let mut after_kx = *post_after;
    after_kx.set(king_sq.0, king_sq.1, None);
    after_kx.set(to.0, to.1, Some(Piece::new(PieceType::King, opp)));

    // Check if king is safe after capture
    let mut tmp_chk = after_kx;
    let unsafe_for_king = is_square_attacked_by_opponent(&mut tmp_chk, to, opp);

    if unsafe_for_king {
        return None;
    }

    // Apply penalty scaled by piece importance
    let base_pen = match moved_pt {
        PieceType::Queen => 900,
        PieceType::Rook => 500,
        PieceType::Bishop | PieceType::Knight => 300,
        _ => 200,
    };
    let scale = match moved_pt {
        PieceType::Queen | PieceType::Rook => 8,
        PieceType::Bishop | PieceType::Knight => 10,
        _ => 8,
    };
    Some((base_pen * scale).clamp(2400, 8000))
}

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
    let mut delta = 0;
    let captured = base_board.get(to.0, to.1);

    if !gives_check {
        // Non-checking moves: simple SEE penalty
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
        return delta;
    }

    // Checking moves: guard against suicidal checks
    let see = see_after(post_after, side, to, captured);
    let moved_pt = base_board.get(from.0, from.1).map(|p| p.get_type());

    if see < 0 {
        // Scale penalty by piece importance
        let pen = match moved_pt {
            Some(PieceType::Queen) => ((-see) * 6).clamp(600, 6000),
            Some(PieceType::Rook) => ((-see) * 4).clamp(3600, 6000),
            Some(PieceType::Bishop) | Some(PieceType::Knight) => ((-see) * 4).clamp(600, 3600),
            Some(PieceType::Pawn) => ((-see) * 2).clamp(200, 2000),
            _ => ((-see) * 3).clamp(300, 3000),
        };
        delta += apply_for_side(-pen, side);
    }

    // Additional penalty for minors attacked by pawn after check
    if let Some(pt) = moved_pt {
        let pawn_pen = pawn_attacked_minor_penalty(post_after, side, to, pt);
        delta += apply_for_side(-pawn_pen, side);
    }

    // Check if opponent king can safely capture the checking piece
    if let Some(pt) = moved_pt
        && matches!(pt, PieceType::Rook | PieceType::Queen | PieceType::Bishop | PieceType::Knight)
            && let Some(pen) = king_can_safely_capture(post_after, side, to, pt) {
                delta += apply_for_side(-pen, side);
            }

    delta
}

