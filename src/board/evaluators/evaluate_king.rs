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
        let centers: [(i32, i32); 4] = [(3,3),(3,4),(4,3),(4,4)];
        let mut best = 99; for (cr,cc) in centers { let dr = (r as i32 - cr).abs(); let dc = (c as i32 - cc).abs(); let d = dr+dc; if d < best { best = d; } }
        return 12 - 3 * best;
    }
    0
}

pub fn development_penalty_on_backrank(board: &Board, color: Color) -> i32 {
    let minors: &[(usize,usize)] = if matches!(color, Color::White) { &[(0,1),(0,6),(0,2),(0,5)] } else { &[(7,1),(7,6),(7,2),(7,5)] };
    let mut pen = 0; for &(r,c) in minors.iter() { if let Some(p)=board.get(r,c) { match p.get_type() { PieceType::Knight | PieceType::Bishop => pen += 14, _ => {} } } }
    let phase = game_phase(board);
    -((pen * phase) / 24)
}
