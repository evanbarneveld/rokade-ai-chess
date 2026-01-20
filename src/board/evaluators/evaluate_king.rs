use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};
use crate::board::evaluator::{opponent, is_piece};

pub fn is_king_in_front_of_pawn(king: (usize, usize), pawn_r: usize, pawn_c: usize, pawn_color: Color) -> bool {
    let (kr, kc) = king;
    if kc != pawn_c { return false; }
    match pawn_color {
        Color::White => kr > pawn_r,
        Color::Black => kr < pawn_r,
    }
}

pub fn king_safety(board: &Board, color: Color, phase: i32, king_pos: Option<(usize, usize)>) -> i32 {
    let mut score = 0;
    if let Some((r, c)) = king_pos {
        let mut shield = 0;
        let shield_row = match color { Color::White => 1, Color::Black => 6 };
        for dc in [-1, 0, 1] {
            let nc = c as i32 + dc;
            if (0..=7).contains(&nc) && is_piece(board, shield_row, nc as usize, color, PieceType::Pawn) { shield += 1; }
        }
        score += (shield * 12 * phase) / 24;

        let enemy = opponent(color);
        let mut attacker_count = 0;
        let mut attack_weight = 0;

        // Count attackers in king zone (3x3 around king and extended 5x5)
        for dr in -2..=2 {
            for dc in -2..=2 {
                let nr = r as i32 + dr;
                let nc = c as i32 + dc;
                if (0..=7).contains(&nr)
                    && (0..=7).contains(&nc)
                    && let Some(p) = board.get(nr as usize, nc as usize)
                    && p.get_color() == enemy
                {
                    let (weight, count) = match p.get_type() {
                        PieceType::Queen => (4, 2),  // Queen counts as 2 attackers
                        PieceType::Rook => (3, 1),
                        PieceType::Knight | PieceType::Bishop => (2, 1),
                        _ => (0, 0),
                    };
                    attack_weight += weight;
                    attacker_count += count;
                }
            }
        }

        // Exponential scaling: danger grows faster with more attackers
        // Base danger from attack weights, multiplied by attacker scaling factor
        // 1 attacker: 1x, 2 attackers: 1.5x, 3 attackers: 2.25x, 4+ attackers: 3x+
        let scaling_factor = match attacker_count {
            0 => 0,
            1 => 10,
            2 => 15,
            3 => 23,
            4 => 32,
            _ => 40 + (attacker_count - 5) * 8,
        };

        let danger = (attack_weight * scaling_factor) / 10;
        score -= (danger * phase) / 24;
    }
    score
}

pub fn king_activity_endgame(king_pos: Option<(usize, usize)>) -> i32 {
    if let Some((r, c)) = king_pos {
        let centers: [(i32, i32); 4] = [(3,3),(3,4),(4,3),(4,4)];
        let mut best = 99; for (cr,cc) in centers { let dr = (r as i32 - cr).abs(); let dc = (c as i32 - cc).abs(); let d = dr+dc; if d < best { best = d; } }
        return 12 - 3 * best;
    }
    0
}

pub fn development_penalty_on_backrank(board: &Board, color: Color, phase: i32) -> i32 {
    let minors: &[(usize,usize)] = if matches!(color, Color::White) { &[(0,1),(0,6),(0,2),(0,5)] } else { &[(7,1),(7,6),(7,2),(7,5)] };
    let mut pen = 0; for &(r,c) in minors.iter() { if let Some(p)=board.get(r,c) { match p.get_type() { PieceType::Knight | PieceType::Bishop => pen += 14, _ => {} } } }
    -((pen * phase) / 24)
}

/// Evaluate king shelter patterns for castled king positions.
/// Detects specific weaknesses in pawn structure around the king.
pub fn evaluate_king_shelter_patterns(board: &Board, color: Color, phase: i32, king_pos: Option<(usize, usize)>) -> i32 {
    let (king_row, king_col) = match king_pos {
        Some(pos) => pos,
        None => return 0,
    };

    // King should be on back rank or one rank forward, and on kingside or queenside
    let on_back_ranks = match color {
        Color::White => king_row <= 1,
        Color::Black => king_row >= 6,
    };

    if !on_back_ranks {
        return 0;
    }

    let kingside_castled = king_col >= 5;
    let queenside_castled = king_col <= 2;

    if !kingside_castled && !queenside_castled {
        return 0; // King in center, shelter patterns less relevant
    }

    let mut penalty = 0;

    // Pawn shelter rows
    let (pawn_row_1, pawn_row_2) = match color {
        Color::White => (1, 2),
        Color::Black => (6, 5),
    };

    if kingside_castled {
        // Evaluate f, g, h pawns for kingside castled position
        // f-pawn (file 5)
        let f_pawn_home = is_piece(board, pawn_row_1, 5, color, PieceType::Pawn);
        let f_pawn_pushed = is_piece(board, pawn_row_2, 5, color, PieceType::Pawn);
        let f_pawn_missing = !f_pawn_home && !f_pawn_pushed;

        // g-pawn (file 6)
        let g_pawn_home = is_piece(board, pawn_row_1, 6, color, PieceType::Pawn);
        let g_pawn_pushed = is_piece(board, pawn_row_2, 6, color, PieceType::Pawn);
        let g_pawn_missing = !g_pawn_home && !g_pawn_pushed;

        // h-pawn (file 7)
        let h_pawn_home = is_piece(board, pawn_row_1, 7, color, PieceType::Pawn);
        let h_pawn_pushed = is_piece(board, pawn_row_2, 7, color, PieceType::Pawn);
        let h_pawn_missing = !h_pawn_home && !h_pawn_pushed;

        // Missing f-pawn is very dangerous (opens diagonal and file)
        if f_pawn_missing {
            penalty += 25;
        } else if f_pawn_pushed {
            penalty += 8; // Pushed f-pawn weakens e1-h4 diagonal
        }

        // g-pawn evaluation
        if g_pawn_missing {
            penalty += 20;
        } else if g_pawn_pushed {
            // Fianchetto (g3/g6) - check if bishop is there
            let fianchetto_square = (pawn_row_2, 6);
            let has_fianchetto_bishop = is_piece(board, fianchetto_square.0, fianchetto_square.1, color, PieceType::Bishop);
            if !has_fianchetto_bishop {
                penalty += 15; // Fianchetto hole without bishop
            }
        }

        // h-pawn evaluation
        if h_pawn_missing {
            penalty += 15;
        } else if h_pawn_pushed {
            penalty += 5; // Slight weakness, but also provides luft
        }

    } else if queenside_castled {
        // Evaluate a, b, c pawns for queenside castled position
        // a-pawn (file 0)
        let a_pawn_home = is_piece(board, pawn_row_1, 0, color, PieceType::Pawn);
        let a_pawn_pushed = is_piece(board, pawn_row_2, 0, color, PieceType::Pawn);
        let a_pawn_missing = !a_pawn_home && !a_pawn_pushed;

        // b-pawn (file 1)
        let b_pawn_home = is_piece(board, pawn_row_1, 1, color, PieceType::Pawn);
        let b_pawn_pushed = is_piece(board, pawn_row_2, 1, color, PieceType::Pawn);
        let b_pawn_missing = !b_pawn_home && !b_pawn_pushed;

        // c-pawn (file 2)
        let c_pawn_home = is_piece(board, pawn_row_1, 2, color, PieceType::Pawn);
        let c_pawn_pushed = is_piece(board, pawn_row_2, 2, color, PieceType::Pawn);
        let c_pawn_missing = !c_pawn_home && !c_pawn_pushed;

        // c-pawn is most important for queenside castled king
        if c_pawn_missing {
            penalty += 25;
        } else if c_pawn_pushed {
            penalty += 8;
        }

        // b-pawn
        if b_pawn_missing {
            penalty += 20;
        } else if b_pawn_pushed {
            // Check for queenside fianchetto
            let fianchetto_square = (pawn_row_2, 1);
            let has_fianchetto_bishop = is_piece(board, fianchetto_square.0, fianchetto_square.1, color, PieceType::Bishop);
            if !has_fianchetto_bishop {
                penalty += 15;
            }
        }

        // a-pawn
        if a_pawn_missing {
            penalty += 12;
        } else if a_pawn_pushed {
            penalty += 4;
        }
    }

    -(penalty * phase) / 24
}
