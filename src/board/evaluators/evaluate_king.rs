use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};
use crate::board::evaluation_helpers::{is_piece, opponent, PawnFileCounts};

pub fn is_king_in_front_of_pawn(king: (usize, usize), pawn_r: usize, pawn_c: usize, pawn_color: Color) -> bool {
    let (kr, kc) = king;
    if kc != pawn_c { return false; }
    match pawn_color {
        Color::White => kr > pawn_r,
        Color::Black => kr < pawn_r,
    }
}

pub fn king_safety(
    board: &Board,
    color: Color,
    phase: i32,
    king_pos: Option<(usize, usize)>,
    pawn_counts: &PawnFileCounts,
) -> i32 {
    let mut score = 0;
    if let Some((r, c)) = king_pos {
        let mut shield = 0;
        let mut shield_gaps = 0;
        let shield_row = match color { Color::White => 1, Color::Black => 6 };
        let shield_row2 = match color { Color::White => 2, Color::Black => 5 };
        for dc in [-1, 0, 1] {
            let nc = c as i32 + dc;
            if (0..=7).contains(&nc) {
                let file = nc as usize;
                let home = is_piece(board, shield_row, file, color, PieceType::Pawn);
                let advanced = is_piece(board, shield_row2, file, color, PieceType::Pawn);
                if home {
                    shield += 2;
                } else if advanced {
                    shield += 1;
                } else {
                    shield_gaps += 1;
                }
            }
        }

        let enemy = opponent(color);
        let enemy_has_queen = has_piece_type(board, enemy, PieceType::Queen);
        let queen_scale = if enemy_has_queen { 100 } else { 60 };
        score += (shield * 6 * phase) / 24;
        if shield_gaps > 0 {
            let gap_penalty = shield_gaps * 14;
            score -= (gap_penalty * phase * queen_scale) / 2400;
        }

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
        score -= (danger * phase * queen_scale) / 2400;

        let file_penalty = king_file_pressure(board, color, (r, c), pawn_counts);
        score -= (file_penalty * phase * queen_scale) / 2400;
    }
    score
}

const KING_RING_ATTACK_WEIGHT: i32 = 4;
const KING_RING_UNSAFE_WEIGHT: i32 = 6;
const KING_RING_CHECK_WEIGHT: i32 = 10;

pub fn king_ring_pressure(
    board: &Board,
    color: Color,
    phase: i32,
    king_pos: Option<(usize, usize)>,
    att_w: &[[bool; 8]; 8],
    att_b: &[[bool; 8]; 8],
) -> i32 {
    let (r, c) = match king_pos {
        Some(pos) => pos,
        None => return 0,
    };

    let enemy = opponent(color);
    let enemy_has_queen = has_piece_type(board, enemy, PieceType::Queen);
    let queen_scale = if enemy_has_queen { 100 } else { 60 };

    let enemy_att = match color {
        Color::White => att_b,
        Color::Black => att_w,
    };

    let mut ring_attacks = 0i32;
    let mut unsafe_attacks = 0i32;

    for dr in -1..=1 {
        for dc in -1..=1 {
            if dr == 0 && dc == 0 {
                continue;
            }
            let nr = r as i32 + dr;
            let nc = c as i32 + dc;
            if !(0..8).contains(&nr) || !(0..8).contains(&nc) {
                continue;
            }
            let nr = nr as usize;
            let nc = nc as usize;
            if enemy_att[nr][nc] {
                ring_attacks += 1;
                if !is_square_attacked_by_color_excluding_king(board, (nr, nc), color) {
                    unsafe_attacks += 1;
                }
            }
        }
    }

    let direct_check = if enemy_att[r][c] { 1 } else { 0 };
    let ring_penalty = ring_attacks * KING_RING_ATTACK_WEIGHT
        + unsafe_attacks * KING_RING_UNSAFE_WEIGHT
        + direct_check * KING_RING_CHECK_WEIGHT;

    let scaled = (ring_penalty * phase * queen_scale) / 2400;
    -scaled
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

fn has_piece_type(board: &Board, color: Color, pt: PieceType) -> bool {
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c)
                && p.get_color() == color
                && p.get_type() == pt
            {
                return true;
            }
        }
    }
    false
}

fn is_square_attacked_by_color_excluding_king(
    board: &Board,
    square: (usize, usize),
    color: Color,
) -> bool {
    let (r, c) = square;

    let pawn_row = match color {
        Color::White => r.checked_sub(1),
        Color::Black => if r < 7 { Some(r + 1) } else { None },
    };
    if let Some(pr) = pawn_row {
        for dc in [-1i32, 1] {
            let pc_i = c as i32 + dc;
            if (0..=7).contains(&pc_i)
                && is_piece(board, pr, pc_i as usize, color, PieceType::Pawn)
            {
                return true;
            }
        }
    }

    for (dr, dc) in [
        (2, 1), (1, 2), (-1, 2), (-2, 1),
        (-2, -1), (-1, -2), (1, -2), (2, -1),
    ] {
        let nr = r as i32 + dr;
        let nc = c as i32 + dc;
        if (0..8).contains(&nr)
            && (0..8).contains(&nc)
            && is_piece(board, nr as usize, nc as usize, color, PieceType::Knight)
        {
            return true;
        }
    }

    for (dr, dc) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
        let mut nr = r as i32 + dr;
        let mut nc = c as i32 + dc;
        while (0..8).contains(&nr) && (0..8).contains(&nc) {
            if let Some(p) = board.get(nr as usize, nc as usize) {
                if p.get_color() == color && matches!(p.get_type(), PieceType::Bishop | PieceType::Queen) {
                    return true;
                }
                break;
            }
            nr += dr;
            nc += dc;
        }
    }

    for (dr, dc) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let mut nr = r as i32 + dr;
        let mut nc = c as i32 + dc;
        while (0..8).contains(&nr) && (0..8).contains(&nc) {
            if let Some(p) = board.get(nr as usize, nc as usize) {
                if p.get_color() == color && matches!(p.get_type(), PieceType::Rook | PieceType::Queen) {
                    return true;
                }
                break;
            }
            nr += dr;
            nc += dc;
        }
    }

    false
}

fn king_file_pressure(
    board: &Board,
    color: Color,
    king_pos: (usize, usize),
    pawn_counts: &PawnFileCounts,
) -> i32 {
    const OPEN_FILE_PENALTY: i32 = 16;
    const SEMI_OPEN_FILE_PENALTY: i32 = 10;
    const FILE_ATTACKER_BONUS: i32 = 8;

    let enemy = opponent(color);
    let (r, c) = king_pos;
    let mut penalty = 0i32;

    for df in [-1i32, 0, 1] {
        let file_i = c as i32 + df;
        if !(0..=7).contains(&file_i) {
            continue;
        }
        let file = file_i as usize;
        let friendly_pawns = match color {
            Color::White => pawn_counts.white[file],
            Color::Black => pawn_counts.black[file],
        };
        if friendly_pawns > 0 {
            continue;
        }
        let enemy_pawns = match color {
            Color::White => pawn_counts.black[file],
            Color::Black => pawn_counts.white[file],
        };
        let mut file_pen = if enemy_pawns == 0 {
            OPEN_FILE_PENALTY
        } else {
            SEMI_OPEN_FILE_PENALTY
        };
        if file == c {
            file_pen += 4;
        }
        if has_rook_or_queen_on_file(board, enemy, file, r) {
            file_pen += FILE_ATTACKER_BONUS;
        }
        penalty += file_pen;
    }

    penalty
}

fn has_rook_or_queen_on_file(
    board: &Board,
    enemy: Color,
    file: usize,
    king_row: usize,
) -> bool {
    for step in [-1i32, 1] {
        let mut r = king_row as i32 + step;
        while (0..8).contains(&r) {
            if let Some(p) = board.get(r as usize, file) {
                return p.get_color() == enemy
                    && matches!(p.get_type(), PieceType::Rook | PieceType::Queen);
            }
            r += step;
        }
    }
    false
}
