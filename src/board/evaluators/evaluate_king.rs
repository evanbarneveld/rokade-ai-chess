use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};
use crate::board::evaluator::{find_king, game_phase, opponent, is_piece};

pub fn is_king_in_front_of_pawn(king: (usize, usize), pawn_r: usize, pawn_c: usize, pawn_color: Color) -> bool {
    let (kr, kc) = king;
    if kc != pawn_c { return false; }
    match pawn_color {
        Color::White => kr > pawn_r,
        Color::Black => kr < pawn_r,
    }
}

pub fn king_safety(board: &Board, color: Color) -> i32 {
    let mut score = 0;
    let k_pos = find_king(board, color);
    if let Some((r, c)) = k_pos {
        let mut shield = 0;
        let shield_row = match color { Color::White => 1, Color::Black => 6 };
        for dc in [-1, 0, 1] {
            let nc = c as i32 + dc;
            if (0..=7).contains(&nc) && is_piece(board, shield_row, nc as usize, color, PieceType::Pawn) { shield += 1; }
        }
        let phase = game_phase(board);
        score += (shield * 12 * phase) / 24;

        let enemy = opponent(color);
        let mut danger = 0;
        for dr in -2..=2 {
            for dc in -2..=2 {
                let nr = r as i32 + dr;
                let nc = c as i32 + dc;
                if (0..=7).contains(&nr)
                    && (0..=7).contains(&nc)
                    && let Some(p) = board.get(nr as usize, nc as usize)
                    && p.get_color() == enemy
                {
                    danger += match p.get_type() {
                        PieceType::Queen => 15,
                        PieceType::Rook => 10,
                        PieceType::Knight | PieceType::Bishop => 6,
                        _ => 0,
                    };
                }
            }
        }
        score -= (danger * phase) / 48;
    }
    score
}

pub fn king_activity_endgame(board: &Board, color: Color) -> i32 {
    let k_pos = find_king(board, color);
    if let Some((r, c)) = k_pos {
        // Chebyshev distance to center (more accurate than Manhattan)
        let centers: [(i32, i32); 4] = [(3,3),(3,4),(4,3),(4,4)];
        let mut best = 99;
        for (cr, cc) in centers {
            let d = (r as i32 - cr).abs().max((c as i32 - cc).abs());
            if d < best { best = d; }
        }
        // Increased weight: max 40cp for center, -10cp per square away
        return 40 - 10 * best;
    }
    0
}

/// Bonus for pushing the enemy king toward edges/corners in winning positions.
/// Returns a positive bonus when the enemy king is restricted to edges/corners.
pub fn enemy_king_cornering_bonus(board: &Board, winning_color: Color) -> i32 {
    let enemy = opponent(winning_color);
    let enemy_king = find_king(board, enemy);
    let own_king = find_king(board, winning_color);

    if let Some((ek_r, ek_c)) = enemy_king {
        let mut bonus = 0i32;

        // Reward enemy king being on edge (row/col 0 or 7)
        let on_edge_r = ek_r == 0 || ek_r == 7;
        let on_edge_c = ek_c == 0 || ek_c == 7;

        if on_edge_r && on_edge_c {
            // Corner: best position for mating
            bonus += 50;
        } else if on_edge_r || on_edge_c {
            // Edge: good progress
            bonus += 30;
        }

        // Penalize enemy king centralization (Chebyshev distance from center)
        let center_dist = (ek_r as i32 - 3).abs().min((ek_r as i32 - 4).abs())
            .max((ek_c as i32 - 3).abs().min((ek_c as i32 - 4).abs()));
        // center_dist: 0 = center, 3 = corner
        bonus += center_dist * 12;

        // Reward own king being close to enemy king (for mating net)
        if let Some((ok_r, ok_c)) = own_king {
            let king_dist = (ok_r as i32 - ek_r as i32).abs().max((ok_c as i32 - ek_c as i32).abs());
            // Closer is better: dist 1 = +30, dist 2 = +20, etc.
            bonus += (7 - king_dist).max(0) * 5;
        }

        return bonus;
    }
    0
}

pub fn development_penalty_on_backrank(board: &Board, color: Color) -> i32 {
    let minors: &[(usize,usize)] = if matches!(color, Color::White) { &[(0,1),(0,6),(0,2),(0,5)] } else { &[(7,1),(7,6),(7,2),(7,5)] };
    let mut pen = 0; for &(r,c) in minors.iter() { if let Some(p)=board.get(r,c) { match p.get_type() { PieceType::Knight | PieceType::Bishop => pen += 14, _ => {} } } }
    let phase = game_phase(board);
    -((pen * phase) / 24)
}
