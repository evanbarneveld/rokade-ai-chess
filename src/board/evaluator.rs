use crate::board::Board;
use crate::piece::pieces::{Color, PieceType};

// Material scores (centipawns)
const PAWN: i32 = 100;
const KNIGHT: i32 = 320;
const BISHOP: i32 = 330;
const ROOK: i32 = 500;
const QUEEN: i32 = 900;
const KING: i32 = 0; // King material is not counted; PST handles its safety/activity

// Piece-Square Tables (from White's perspective, row 0 = White back rank)
// Values in centipawns; lightweight, generic PSTs
// Flipped vertically so that advancing pawns are rewarded toward promotion
const PST_PAWN: [[i32; 8]; 8] = [
    [  0,   0,   0,   0,   0,   0,   0,   0],  // row 0
    [  5,  10,  10, -20, -20,  10,  10,   5],  // row 1 (start rank)
    [  5,  -5, -10,   0,   0, -10,  -5,   5],
    [  0,   0,   0,  20,  20,   0,   0,   0],
    [  5,   5,  10,  25,  25,  10,   5,   5],
    [ 10,  10,  20,  30,  30,  20,  10,  10],
    [ 50,  50,  50,  50,  50,  50,  50,  50],  // advanced pawns credited
    [  0,  0,  0,  0,  0,  0,  0,  0],         // row 7 (promotion rank)
];

const PST_KNIGHT: [[i32; 8]; 8] = [
    [-50, -40, -30, -30, -30, -30, -40, -50],
    [-40, -20,   0,   0,   0,   0, -20, -40],
    [-30,   0,  10,  15,  15,  10,   0, -30],
    [-30,   5,  15,  20,  20,  15,   5, -30],
    [-30,   0,  15,  20,  20,  15,   0, -30],
    [-30,   5,  10,  15,  15,  10,   5, -30],
    [-40, -20,   0,   5,   5,   0, -20, -40],
    [-50, -40, -30, -30, -30, -30, -40, -50],
];

const PST_BISHOP: [[i32; 8]; 8] = [
    [-20, -10, -10, -10, -10, -10, -10, -20],
    [-10,   5,   0,   0,   0,   0,   5, -10],
    [-10,  10,  10,  10,  10,  10,  10, -10],
    [-10,   0,  10,  10,  10,  10,   0, -10],
    [-10,   5,   5,  10,  10,   5,   5, -10],
    [-10,   0,   5,  10,  10,   5,   0, -10],
    [-10,   0,   0,   0,   0,   0,   0, -10],
    [-20, -10, -10, -10, -10, -10, -10, -20],
];

const PST_ROOK: [[i32; 8]; 8] = [
    [  0,   0,   5,  10,  10,   5,   0,   0],
    [ -5,   0,   0,   0,   0,   0,   0,  -5],
    [ -5,   0,   0,   0,   0,   0,   0,  -5],
    [ -5,   0,   0,   0,   0,   0,   0,  -5],
    [ -5,   0,   0,   0,   0,   0,   0,  -5],
    [ -5,   0,   0,   0,   0,   0,   0,  -5],
    [  5,  10,  10,  10,  10,  10,  10,   5],
    [  0,   0,   0,   0,   0,   0,   0,   0],
];

const PST_QUEEN: [[i32; 8]; 8] = [
    [-20, -10, -10,  -5,  -5, -10, -10, -20],
    [-10,   0,   0,   0,   0,   0,   0, -10],
    [-10,   0,   5,   5,   5,   5,   0, -10],
    [ -5,   0,   5,   5,   5,   5,   0,  -5],
    [  0,   0,   5,   5,   5,   5,   0,  -5],
    [-10,   5,   5,   5,   5,   5,   0, -10],
    [-10,   0,   5,   0,   0,   0,   0, -10],
    [-20, -10, -10,  -5,  -5, -10, -10, -20],
];

const PST_KING_MIDGAME: [[i32; 8]; 8] = [
    [-30, -40, -40, -50, -50, -40, -40, -30],
    [-30, -40, -40, -50, -50, -40, -40, -30],
    [-30, -40, -40, -50, -50, -40, -40, -30],
    [-30, -40, -40, -50, -50, -40, -40, -30],
    [-20, -30, -30, -40, -40, -30, -30, -20],
    [-10, -20, -20, -20, -20, -20, -20, -10],
    [ 20,  20,   0,   0,   0,   0,  20,  20],
    [ 20,  30,  10,   0,   0,  10,  30,  20],
];

// Endgame king PST to encourage centralization and activity in simplified positions
const PST_KING_ENDGAME: [[i32; 8]; 8] = [
    [-10, -10, -10, -10, -10, -10, -10, -10],
    [ -5,   0,   0,   0,   0,   0,   0,  -5],
    [ -5,   0,  10,  15,  15,  10,   0,  -5],
    [ -5,   0,  15,  20,  20,  15,   0,  -5],
    [ -5,   0,  15,  20,  20,  15,   0,  -5],
    [ -5,   0,  10,  15,  15,  10,   0,  -5],
    [ -5,  -5,   0,  10,  10,   0,  -5,  -5],
    [-10, -10, -10, -10, -10, -10, -10, -10],
];

#[inline]
fn mirror_row_for_black(row: usize) -> usize { 7 - row }

#[inline]
fn material_value(piece: PieceType) -> i32 {
    match piece {
        PieceType::Pawn => PAWN,
        PieceType::Knight => KNIGHT,
        PieceType::Bishop => BISHOP,
        PieceType::Rook => ROOK,
        PieceType::Queen => QUEEN,
        PieceType::King => KING,
    }
}

#[inline]
fn pst_value_tapered(piece: PieceType, row: usize, col: usize, color: Color, phase: i32) -> i32 {
    // Map black squares by mirroring rows so PSTs are from White's perspective
    let (r, c) = match color {
        Color::White => (row, col),
        Color::Black => (mirror_row_for_black(row), col),
    };

    // Midgame values
    let mg = match piece {
        PieceType::Pawn => PST_PAWN[r][c],
        PieceType::Knight => PST_KNIGHT[r][c],
        PieceType::Bishop => PST_BISHOP[r][c],
        PieceType::Rook => PST_ROOK[r][c],
        PieceType::Queen => PST_QUEEN[r][c],
        PieceType::King => PST_KING_MIDGAME[r][c],
    };

    // Endgame values (default to MG if no EG table is defined)
    let eg = match piece {
        PieceType::King => PST_KING_ENDGAME[r][c],
        _ => mg,
    };

    // Linear interpolation between midgame and endgame based on phase [0..24]
    (mg * phase + eg * (24 - phase)) / 24
}

// Compute a simple material-based game phase: 24 = full midgame, 0 = pure endgame
fn game_phase(board: &Board) -> i32 {
    // Piece phase weights per piece instance
    const PHASE_KNIGHT: i32 = 1;
    const PHASE_BISHOP: i32 = 1;
    const PHASE_ROOK: i32 = 2;
    const PHASE_QUEEN: i32 = 4;

    let mut phase: i32 = 0;

    // Count pieces for both sides
    for row in 0..8 {
        for col in 0..8 {
            if let Some(piece) = board.get(row, col) {
                phase += match piece.get_type() {
                    PieceType::Knight => PHASE_KNIGHT,
                    PieceType::Bishop => PHASE_BISHOP,
                    PieceType::Rook => PHASE_ROOK,
                    PieceType::Queen => PHASE_QUEEN,
                    _ => 0,
                };
            }
        }
    }

    // Clamp to [0, 24] where 24 is initial (all heavy/minor pieces present)
    if phase < 0 { 0 } else if phase > 24 { 24 } else { phase }
}

// Public evaluation function: positive = better for White; negative = better for Black
pub fn evaluate_position(board: &Board, side_to_move: Color) -> i32 {
    let mut score: i32 = 0;
    let phase = game_phase(board);
    let eg = 24 - phase; // endgame weight [0..24]

    // Precompute whether each side still has any pawns (used for rook-on-7th bonus)
    let mut white_pawns = 0i32;
    let mut black_pawns = 0i32;
    for r in 0..8 { for c in 0..8 { if let Some(p)=board.get(r,c) { if matches!(p.get_type(), PieceType::Pawn) {
        if p.get_color()==Color::White { white_pawns += 1; } else { black_pawns += 1; }
    }}}}

    for row in 0..8 {
        for col in 0..8 {
            if let Some(piece) = board.get(row, col) {
                let pt = piece.get_type();
                let color = piece.get_color();
                let mut val = material_value(pt) + pst_value_tapered(pt, row, col, color, phase);

                // Encourage center pawn development in the opening/early middlegame,
                // discourage premature rook-pawn pushes (e.g., h2-h4) as first plans.
                if pt == PieceType::Pawn {
                    // File bonuses from a..h: a/h negative, c/f small positive, d/e strong positive
                    const FILE_BONUS: [i32; 8] = [-30, -10, 10, 25, 25, 10, -10, -30];
                    let file_bonus = (FILE_BONUS[col] * phase) / 24; // taper to 0 by endgame
                    val += file_bonus;

                    // Mild penalty for advanced rook pawns in opening (beyond third rank from own side)
                    if phase > 12 {
                        let is_rook_file = col == 0 || col == 7;
                        if is_rook_file {
                            let advancement_from_home: i32 = match color {
                                Color::White => row as i32,        // white home rank = 0
                                Color::Black => (7 - row) as i32,   // mirror for black
                            };
                            if advancement_from_home >= 3 {
                                val -= (15 * phase) / 24; // up to -15cp in full opening
                            }
                        }
                    }

                    // Lightweight pawn-structure terms to improve opening choices
                    if is_doubled_pawn(board, row, col, color) { val -= 12; }
                    if is_isolated_pawn(board, col, color) { val -= 14; }
                    if is_passed_pawn(board, row, col, color) {
                        let advance = match color { Color::White => row as i32, Color::Black => (7 - row) as i32 };
                        // Base passer bonus grows with advancement and endgame weight (slightly steeper)
                        val += ((8 + 4 * advance) * (8 + eg)) / 24; // ~+12..+50cp

                        // Additional endgame-scaled passer heuristics
                        if eg > 0 {
                            // Clear path to promotion bonus
                            if has_clear_promotion_path(board, row, col, color) {
                                val += (10 * eg) / 24; // up to +10cp
                            }

                            // King proximity and blocking king
                            if let (Some((fk_r,fk_c)), Some((ek_r,ek_c))) = (find_king(board, color), find_king(board, opponent(color))) {
                                let pawn_sq = (row as i32, col as i32);
                                let fk_d = chebyshev_dist((fk_r as i32, fk_c as i32), pawn_sq);
                                let ek_d = chebyshev_dist((ek_r as i32, ek_c as i32), pawn_sq);

                                // Friendly king close to passer
                                let prox = (12 - 2 * fk_d).max(0); // up to ~+12 when adjacent
                                val += (prox * eg) / 48;

                                // Enemy king in front of the pawn and closer than friendly king
                                if is_king_in_front_of_pawn((ek_r,ek_c), row, col, color) && ek_d + 0 <= fk_d { // strictly closer or equal
                                    let block_pen = (14 - 2 * (ek_d as i32)).max(0);
                                    val -= (block_pen * eg) / 48; // up to -14 scaled
                                }
                            }
                        }
                    }
                }
                
                // Rook-specific endgame features
                if pt == PieceType::Rook && eg > 0 {
                    // Rook on opponent's 7th rank if opponent still has pawns
                    let on_7th = match color {
                        Color::White => row == 6 && black_pawns > 0,
                        Color::Black => row == 1 && white_pawns > 0,
                    };
                    if on_7th { val += (30 * eg) / 24; } // up to +30cp

                    // Rook behind own passed pawn (same file, in the direction of promotion, clear line)
                    if let Some((pp_r, _)) = find_passed_pawn_on_file(board, col, color) {
                        let behind = match color { Color::White => row < pp_r, Color::Black => row > pp_r };
                        if behind && file_clear_between(board, row, pp_r, col) {
                            // Scale by pawn advancement toward promotion
                            let adv = match color { Color::White => pp_r as i32, Color::Black => (7 - pp_r) as i32 };
                            let bonus = 12 + 2 * adv; // ~+12..+26
                            val += (bonus * eg) / 24;
                        }
                    }

                    // Cut-off king heuristic: rook cutting the enemy king along file or rank with empty squares between
                    if let Some((ek_r, ek_c)) = find_king(board, opponent(color)) {
                        // Same file cut-off
                        if col == ek_c {
                            if file_clear_between(board, row, ek_r, col) {
                                let dist = (row as i32 - ek_r as i32).abs() as i32;
                                if dist >= 2 { val += (10 * eg) / 24; }
                            }
                        }
                        // Same rank cut-off
                        if row == ek_r {
                            if rank_clear_between(board, col, ek_c, row) {
                                let dist = (col as i32 - ek_c as i32).abs() as i32;
                                if dist >= 2 { val += (10 * eg) / 24; }
                            }
                        }
                    }
                }

                // Early-opening discouragement: rook tucked behind two own pawns on back rank (Rb1/Rg1 patterns)
                if pt == PieceType::Rook && phase > 0 {
                    // Only consider exact back rank squares b1/g1 for White and b8/g8 for Black
                    let is_back_rank = match color { Color::White => row == 0, Color::Black => row == 7 };
                    if is_back_rank && (col == 1 || col == 6) {
                        // Check if both adjacent home pawns in front remain on their initial squares
                        let both_pawns_blocking = match (color, col) {
                            (Color::White, 1) => {
                                matches!(board.get(1,0).filter(|p| p.get_color()==Color::White && matches!(p.get_type(), PieceType::Pawn)), Some(_)) &&
                                matches!(board.get(1,1).filter(|p| p.get_color()==Color::White && matches!(p.get_type(), PieceType::Pawn)), Some(_))
                            }
                            (Color::White, 6) => {
                                matches!(board.get(1,6).filter(|p| p.get_color()==Color::White && matches!(p.get_type(), PieceType::Pawn)), Some(_)) &&
                                matches!(board.get(1,7).filter(|p| p.get_color()==Color::White && matches!(p.get_type(), PieceType::Pawn)), Some(_))
                            }
                            (Color::Black, 1) => {
                                matches!(board.get(6,0).filter(|p| p.get_color()==Color::Black && matches!(p.get_type(), PieceType::Pawn)), Some(_)) &&
                                matches!(board.get(6,1).filter(|p| p.get_color()==Color::Black && matches!(p.get_type(), PieceType::Pawn)), Some(_))
                            }
                            (Color::Black, 6) => {
                                matches!(board.get(6,6).filter(|p| p.get_color()==Color::Black && matches!(p.get_type(), PieceType::Pawn)), Some(_)) &&
                                matches!(board.get(6,7).filter(|p| p.get_color()==Color::Black && matches!(p.get_type(), PieceType::Pawn)), Some(_))
                            }
                            _ => false,
                        };
                        if both_pawns_blocking {
                            // Up to -18cp in full opening, taper to 0 by endgame
                            val -= (18 * phase) / 24;
                        }
                    }
                }

                // Extra endgame pawn aggressiveness and structure nuances
                if pt == PieceType::Pawn && eg > 0 {
                    // Stronger advancement slope for passed pawns in deep endgames
                    if is_passed_pawn(board, row, col, color) {
                        // Add a small extra advancement boost when close to promotion
                        let adv = match color { Color::White => row as i32, Color::Black => (7 - row) as i32 };
                        let close_bonus = (adv.saturating_sub(4) * 6).max(0); // up to ~+18 when on 7th
                        val += (close_bonus * eg) / 24;

                        // Penalize blocked passers (piece directly ahead)
                        let next_r_opt = match color { Color::White => if row < 7 { Some(row + 1) } else { None },
                                                       Color::Black => if row > 0 { Some(row - 1) } else { None } };
                        if let Some(nr) = next_r_opt { if board.get(nr, col).is_some() { val -= (14 * eg) / 24; } }

                        // Free-push incentive: next square empty and not obviously unsafe
                        if let Some(nr) = next_r_opt {
                            if board.get(nr, col).is_none() {
                                // Light safety: discourage if enemy king is immediately in front, otherwise small bonus
                                let mut safe_bonus = 0;
                                // If enemy king is in front on adjacent file within 1 step of target, avoid bonus
                                if let Some((ek_r, ek_c)) = find_king(board, opponent(color)) {
                                    let dr = (ek_r as i32 - nr as i32).abs();
                                    let dc = (ek_c as i32 - col as i32).abs();
                                    if !(dr <= 1 && dc <= 1 && ((color==Color::White && ek_r >= nr) || (color==Color::Black && ek_r <= nr))) {
                                        safe_bonus = 10; // nominal +10, tapered by eg
                                    }
                                } else { safe_bonus = 10; }
                                val += (safe_bonus * eg) / 24;
                            }
                        }

                        // Protected passer/connected passer nudges
                        let mut support = 0;
                        // Protected by own pawn diagonally behind
                        let behind_r_opt = match color { Color::White => row.checked_sub(1), Color::Black => if row<7 { Some(row+1) } else { None } };
                        if let Some(br) = behind_r_opt {
                            for dc in [-1i32, 1] {
                                let nc = (col as i32 + dc) as usize; if dc==-1 && col==0 { continue; }
                                if dc==1 && col==7 { continue; }
                                if let Some(p)=board.get(br, nc) { if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn) { support += 1; } }
                            }
                        }
                        // Connected adjacent pawn on same rank
                        for dc in [-1i32, 1] { let nc = (col as i32 + dc) as usize; if dc==-1 && col==0 { continue; }
                            if dc==1 && col==7 { continue; }
                            if let Some(p)=board.get(row, nc) { if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn) { support += 1; } }
                        }
                        if support > 0 { val += (8 * support as i32 * eg) / 24; }
                    }
                }
                match color {
                    Color::White => score += val,
                    Color::Black => score -= val,
                }
            }
        }
    }

    // Basic attacked/defended (hanging piece) penalties
    let (att_w, att_b) = build_attack_maps(board);
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c) {
                let color = p.get_color();
                let attacked_by_opp = match color {
                    Color::White => att_b[r][c],
                    Color::Black => att_w[r][c],
                };
                let defended_by_own = match color {
                    Color::White => att_w[r][c],
                    Color::Black => att_b[r][c],
                };
                if attacked_by_opp && !defended_by_own {
                    let base_pen = match p.get_type() {
                        PieceType::Pawn => 15,
                        PieceType::Knight | PieceType::Bishop => 30,
                        PieceType::Rook => 45,
                        PieceType::Queen => 60,
                        PieceType::King => 0,
                    };
                    let pen = base_pen * phase / 24; // emphasize middlegame
                    match color { Color::White => score -= pen, Color::Black => score += pen }
                }
            }
        }
    }

    // True tempo: small bonus to the actual side to move, tapered by phase
    let tempo_bonus = 8; // in centipawns
    let tempo = (tempo_bonus * phase) / 24;
    match side_to_move { Color::White => score += tempo, Color::Black => score -= tempo }

    // Add a basic mobility term (pseudo-legal, lightweight), phase-weighted for middlegame
    let (mob_w, mob_b) = mobility_activity(board);
    // Scale: bishops/rooks/queen drive this mostly; the helper already applies type weights.
    // Here we weight by phase to emphasize middlegame activity.
    score += mob_w * phase / 24;
    score -= mob_b * phase / 24;

    // Global light features to bias toward sound openings
    // Bishop pair (middlegame‑weighted)
    let (w_bishops, b_bishops) = count_bishops(board);
    if w_bishops >= 2 { score += (36 * phase) / 24; }
    if b_bishops >= 2 { score -= (36 * phase) / 24; }

    // Rooks on open/semi-open files (middlegame‑weighted)
    score += rook_file_activity(board, Color::White) * phase / 24;
    score -= rook_file_activity(board, Color::Black) * phase / 24;

    // King safety (opening‑weighted) and endgame king activity
    score += king_safety(board, Color::White) * phase / 24;
    score -= king_safety(board, Color::Black) * phase / 24;
    score += king_activity_endgame(board, Color::White) * (24 - phase) / 24;
    score -= king_activity_endgame(board, Color::Black) * (24 - phase) / 24;

    // Development nudges in opening
    if phase > 12 {
        score += development_penalty_on_backrank(board, Color::White) * (phase - 12) / 12;
        score -= development_penalty_on_backrank(board, Color::Black) * (phase - 12) / 12;
    }

    score
}

// ---- Helpers ----

#[inline]
fn is_doubled_pawn(board: &Board, row: usize, col: usize, color: Color) -> bool {
    for r in 0..8 {
        if r==row { continue; }
        if let Some(p)=board.get(r,col) {
            if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn) { return true; }
        }
    }
    false
}

#[inline]
fn is_isolated_pawn(board: &Board, col: usize, color: Color) -> bool {
    for dc in [-1i32, 1] {
        let nc = col as i32 + dc; if nc < 0 || nc > 7 { continue; }
        for r in 0..8 { if let Some(p)=board.get(r, nc as usize) { if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn) { return false; } } }
    }
    true
}

#[inline]
fn is_passed_pawn(board: &Board, row: usize, col: usize, color: Color) -> bool {
    let dir: i32 = if color==Color::White { 1 } else { -1 };
    let mut r = row as i32 + dir;
    while r>=0 && r<8 {
        for dc in [-1i32,0,1] { let nc = col as i32 + dc; if nc<0 || nc>=8 { continue; }
            if let Some(p)=board.get(r as usize, nc as usize) { if p.get_color()!=color && matches!(p.get_type(), PieceType::Pawn) { return false; } }
        }
        r += dir;
    }
    true
}

fn count_bishops(board: &Board) -> (i32,i32) {
    let mut w=0; let mut b=0; for r in 0..8 { for c in 0..8 {
        if let Some(p)=board.get(r,c) { if matches!(p.get_type(), PieceType::Bishop) { if p.get_color()==Color::White { w+=1; } else { b+=1; } } }
    }} (w,b)
}

fn rook_file_activity(board: &Board, color: Color) -> i32 {
    let mut bonus = 0;
    for r in 0..8 { for c in 0..8 {
        if let Some(p)=board.get(r,c) { if p.get_color()==color && matches!(p.get_type(), PieceType::Rook) {
            let mut wp=0; let mut bp=0; for rr in 0..8 { if let Some(pp)=board.get(rr,c) { if matches!(pp.get_type(), PieceType::Pawn) { if pp.get_color()==Color::White { wp+=1; } else { bp+=1; } } } }
            let open = wp==0 && bp==0;
            let semi = match color { Color::White => wp==0 && bp>0, Color::Black => bp==0 && wp>0 };
            if open { bonus += 14; } else if semi { bonus += 8; }
        }}
    }}
    bonus
}

fn find_king(board: &Board, color: Color) -> Option<(usize,usize)> {
    for r in 0..8 { for c in 0..8 { if let Some(p)=board.get(r,c) { if matches!(p.get_type(), PieceType::King) && p.get_color()==color { return Some((r,c)); } } } }
    None
}

fn king_safety(board: &Board, color: Color) -> i32 {
    if let Some((_kr, kf)) = find_king(board, color) {
        // Pawn shield directly in front of home rank (one rank forward from home)
        let front_rank: i32 = if matches!(color, Color::White) { 1 } else { 6 };
        let mut shield = 0;
        for df in -1..=1 {
            let f = kf as i32 + df; if f<0 || f>7 { continue; }
            if let Some(p)=board.get(front_rank as usize, f as usize) { if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn) { shield += 1; } }
        }
        let mut pen = 0;
        if shield==0 { pen += 30; } else if shield==1 { pen += 18; } else if shield==2 { pen += 8; }
        // Half-open king file penalty
        let mut own=0; let mut opp=0; for r in 0..8 { if let Some(p)=board.get(r, kf) { if matches!(p.get_type(), PieceType::Pawn) { if p.get_color()==color { own+=1; } else { opp+=1; } } } }
        if own==0 && opp>0 { pen += 14; }
        return -pen;
    }
    0
}

fn king_activity_endgame(board: &Board, color: Color) -> i32 {
    if let Some((r,c)) = find_king(board, color) {
        let centers = [(3,3),(3,4),(4,3),(4,4)];
        let mut best = 99; for (cr,cc) in centers { let dr = (r as i32 - cr as i32).abs(); let dc = (c as i32 - cc as i32).abs(); let d = dr+dc; if d < best { best = d; } }
        return 12 - 3 * best; // up to about +12
    }
    0
}

fn development_penalty_on_backrank(board: &Board, color: Color) -> i32 {
    let minors: &[(usize,usize)] = if matches!(color, Color::White) { &[(0,1),(0,6),(0,2),(0,5)] } else { &[(7,1),(7,6),(7,2),(7,5)] };
    let mut pen = 0; for &(r,c) in minors.iter() { if let Some(p)=board.get(r,c) { match p.get_type() { PieceType::Knight | PieceType::Bishop => pen += 6, _ => {} } } }
    -pen
}

// ---- New endgame helpers ----

#[inline]
fn opponent(color: Color) -> Color { if matches!(color, Color::White) { Color::Black } else { Color::White } }

#[inline]
fn chebyshev_dist(a: (i32,i32), b: (i32,i32)) -> i32 { (a.0 - b.0).abs().max((a.1 - b.1).abs()) }

// Check if all squares from the pawn to the promotion rank are empty (excluding current square)
fn has_clear_promotion_path(board: &Board, row: usize, col: usize, color: Color) -> bool {
    if !matches!(board.get(row,col).map(|p| p.get_type()), Some(PieceType::Pawn)) { return false; }
    let (start, end, step): (i32, i32, i32) = match color {
        Color::White => (row as i32 + 1, 7, 1),
        Color::Black => (row as i32 - 1, 0, -1),
    };
    let mut r = start; while (step>0 && r<=end) || (step<0 && r>=end) {
        if board.get(r as usize, col).is_some() { return false; }
        r += step;
    }
    true
}

// True if enemy king stands on a square in front of the pawn along its file or adjacent files ahead
fn is_king_in_front_of_pawn(king: (usize,usize), pawn_r: usize, pawn_c: usize, pawn_color: Color) -> bool {
    let (kr,kc) = king; let pr = pawn_r as i32; let pc = pawn_c as i32;
    match pawn_color {
        Color::White => {
            let kr_i = kr as i32; let kc_i = kc as i32;
            kr_i > pr && (kc_i - pc).abs() <= 1
        }
        Color::Black => {
            let kr_i = kr as i32; let kc_i = kc as i32;
            kr_i < pr && (kc_i - pc).abs() <= 1
        }
    }
}

// Find a passed pawn on a given file for the color, preferring the most advanced toward promotion
fn find_passed_pawn_on_file(board: &Board, file: usize, color: Color) -> Option<(usize,usize)> {
    let mut best: Option<(usize,usize)> = None;
    for r in 0..8 {
        if let Some(p)=board.get(r, file) { if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn) {
            if is_passed_pawn(board, r, file, color) {
                best = Some(match (best, color) {
                    (None, _) => (r,file),
                    (Some((br,_)), Color::White) => if r > br { (r,file) } else { (br,file) },
                    (Some((br,_)), Color::Black) => if r < br { (r,file) } else { (br,file) },
                });
            }
        } }
    }
    best
}

// Check that squares strictly between r1 and r2 on the same file are empty
fn file_clear_between(board: &Board, r1: usize, r2: usize, file: usize) -> bool {
    if r1==r2 { return true; }
    let (lo, hi) = if r1 < r2 { (r1+1, r2-1) } else { (r2+1, r1-1) };
    if hi < lo { return true; }
    for r in lo..=hi { if board.get(r, file).is_some() { return false; } }
    true
}

// Check that squares strictly between c1 and c2 on the same rank are empty
fn rank_clear_between(board: &Board, c1: usize, c2: usize, rank: usize) -> bool {
    if c1==c2 { return true; }
    let (lo, hi) = if c1 < c2 { (c1+1, c2-1) } else { (c2+1, c1-1) };
    if hi < lo { return true; }
    for c in lo..=hi { if board.get(rank, c).is_some() { return false; } }
    true
}

// ---- Lightweight mobility and attack/defense helpers ----

// Return a pseudo-legal mobility activity score for White and Black.
// Type weights roughly: N=1, B=2, R=2, Q=1, K=0, P=0 (we keep K/P unweighted to avoid noise)
fn mobility_activity(board: &Board) -> (i32, i32) {
    let mut white: i32 = 0;
    let mut black: i32 = 0;
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c) {
                let color = p.get_color();
                let add = match p.get_type() {
                    PieceType::Knight => count_knight_targets(board, r, c, color) as i32 * 1,
                    PieceType::Bishop => count_slider_targets(board, r, c, color, &[(1,1),(1,-1),(-1,1),(-1,-1)]) as i32 * 2,
                    PieceType::Rook   => count_slider_targets(board, r, c, color, &[(1,0),(-1,0),(0,1),(0,-1)]) as i32 * 2,
                    PieceType::Queen  => count_slider_targets(board, r, c, color, &[(1,1),(1,-1),(-1,1),(-1,-1),(1,0),(-1,0),(0,1),(0,-1)]) as i32 * 1,
                    _ => 0,
                };
                if matches!(color, Color::White) { white += add; } else { black += add; }
            }
        }
    }
    (white, black)
}

#[inline]
fn count_knight_targets(board: &Board, r: usize, c: usize, color: Color) -> usize {
    const K: [(i32,i32);8] = [(2,1),(1,2),(-1,2),(-2,1),(-2,-1),(-1,-2),(1,-2),(2,-1)];
    let mut n = 0usize;
    for (dr,dc) in K { let nr = r as i32 + dr; let nc = c as i32 + dc; if nr>=0&&nr<8&&nc>=0&&nc<8 {
        match board.get(nr as usize, nc as usize) { None => n+=1, Some(tp) if tp.get_color()!=color => n+=1, _ => {} }
    }}
    n
}

#[inline]
fn count_slider_targets(board: &Board, r: usize, c: usize, color: Color, dirs: &[(i32,i32)]) -> usize {
    let mut n = 0usize;
    for (dr,dc) in dirs.iter() {
        let mut nr = r as i32 + dr; let mut nc = c as i32 + dc;
        while nr>=0 && nr<8 && nc>=0 && nc<8 {
            if let Some(tp) = board.get(nr as usize, nc as usize) {
                if tp.get_color()!=color { n+=1; }
                break;
            } else { n+=1; }
            nr += dr; nc += dc;
        }
    }
    n
}

// Attack maps (pseudo-legal): squares attacked by White and by Black
fn build_attack_maps(board: &Board) -> ([[bool;8];8], [[bool;8];8]) {
    let mut w = [[false;8];8];
    let mut b = [[false;8];8];
    for r in 0..8 { for c in 0..8 {
        if let Some(p)=board.get(r,c) {
            let color = p.get_color();
            match p.get_type() {
                PieceType::Knight => add_knight_attacks(board, r, c, color, &mut w, &mut b),
                PieceType::Bishop => add_slider_attacks(board, r, c, color, &[(1,1),(1,-1),(-1,1),(-1,-1)], &mut w, &mut b),
                PieceType::Rook   => add_slider_attacks(board, r, c, color, &[(1,0),(-1,0),(0,1),(0,-1)], &mut w, &mut b),
                PieceType::Queen  => add_slider_attacks(board, r, c, color, &[(1,1),(1,-1),(-1,1),(-1,-1),(1,0),(-1,0),(0,1),(0,-1)], &mut w, &mut b),
                PieceType::King   => add_king_attacks(r, c, color, &mut w, &mut b),
                PieceType::Pawn   => add_pawn_attacks(r, c, color, &mut w, &mut b),
            }
        }
    }}
    (w, b)
}

#[inline]
fn add_knight_attacks(board: &Board, r: usize, c: usize, color: Color, w: &mut [[bool;8];8], b: &mut [[bool;8];8]) {
    const K: [(i32,i32);8] = [(2,1),(1,2),(-1,2),(-2,1),(-2,-1),(-1,-2),(1,-2),(2,-1)];
    for (dr,dc) in K { let nr=r as i32+dr; let nc=c as i32+dc; if nr>=0&&nr<8&&nc>=0&&nc<8 {
        match color { Color::White => w[nr as usize][nc as usize]=true, Color::Black => b[nr as usize][nc as usize]=true }
    }}
}

#[inline]
fn add_slider_attacks(board: &Board, r: usize, c: usize, color: Color, dirs: &[(i32,i32)], w: &mut [[bool;8];8], b: &mut [[bool;8];8]) {
    for (dr,dc) in dirs.iter() {
        let mut nr=r as i32+dr; let mut nc=c as i32+dc;
        while nr>=0&&nr<8&&nc>=0&&nc<8 {
            match color { Color::White => w[nr as usize][nc as usize]=true, Color::Black => b[nr as usize][nc as usize]=true }
            if board.get(nr as usize, nc as usize).is_some() { break; }
            nr += dr; nc += dc;
        }
    }
}

#[inline]
fn add_king_attacks(r: usize, c: usize, color: Color, w: &mut [[bool;8];8], b: &mut [[bool;8];8]) {
    for dr in -1..=1 { for dc in -1..=1 { if dr==0&&dc==0 { continue; } let nr=r as i32+dr; let nc=c as i32+dc; if nr>=0&&nr<8&&nc>=0&&nc<8 {
        match color { Color::White => w[nr as usize][nc as usize]=true, Color::Black => b[nr as usize][nc as usize]=true }
    }}}
}

#[inline]
fn add_pawn_attacks(r: usize, c: usize, color: Color, w: &mut [[bool;8];8], b: &mut [[bool;8];8]) {
    match color {
        Color::White => {
            if r<7 {
                if c>0 { w[r+1][c-1]=true; }
                if c<7 { w[r+1][c+1]=true; }
            }
        }
        Color::Black => {
            if r>0 {
                if c>0 { b[r-1][c-1]=true; }
                if c<7 { b[r-1][c+1]=true; }
            }
        }
    }
}
