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
const PST_PAWN: [[i32; 8]; 8] = [
    [  0,   0,   0,   0,   0,   0,   0,   0],
    [ 50,  50,  50,  50,  50,  50,  50,  50],
    [ 10,  10,  20,  30,  30,  20,  10,  10],
    [  5,   5,  10,  25,  25,  10,   5,   5],
    [  0,   0,   0,  20,  20,   0,   0,   0],
    [  5,  -5, -10,   0,   0, -10,  -5,   5],
    [  5,  10,  10, -20, -20,  10,  10,   5],
    [  0,   0,   0,   0,   0,   0,   0,   0],
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
pub fn evaluate_position(board: &Board) -> i32 {
    let mut score: i32 = 0;
    let phase = game_phase(board);

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
                }
                match color {
                    Color::White => score += val,
                    Color::Black => score -= val,
                }
            }
        }
    }

    // Small tempo bonus to the side to move to break ties among otherwise equal quiet moves.
    // This helps differentiate opening replies where static features are very similar.
    // Positive = better for White, negative = better for Black.
    // We detect the side to move by counting legal moves; cheaper: infer by parity of total moves is not available here.
    // Instead, approximate: if White has strictly more legal moves than Black, assume White to move; vice versa.
    // To keep it deterministic and inexpensive, give a fixed tiny bonus scaled by phase.
    // Note: evaluation here is board-only; if a true side-to-move flag is available in GameState, prefer passing it in.
    let tempo_bonus = 8; // in centipawns
    // Heuristic: in opening/middlegame (phase high), apply full bonus, taper towards endgame.
    let tempo = (tempo_bonus * phase) / 24;
    // Estimate side to move by mobility; if equal, give no bonus.
    let mut white_moves = 0usize;
    let mut black_moves = 0usize;
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c) {
                let color = p.get_color();
                match p.get_type() {
                    PieceType::Knight => {
                        // Knight mobility: up to 8 offsets
                        const K: [(i32,i32);8] = [(2,1),(1,2),(-1,2),(-2,1),(-2,-1),(-1,-2),(1,-2),(2,-1)];
                        for (dr, dc) in K { let nr = r as i32 + dr; let nc = c as i32 + dc; if nr>=0 && nr<8 && nc>=0 && nc<8 {
                            if let Some(tp) = board.get(nr as usize, nc as usize) { if tp.get_color()!=color { if color==Color::White { white_moves+=1; } else { black_moves+=1; } } } else { if color==Color::White { white_moves+=1; } else { black_moves+=1; } }
                        }}
                    }
                    PieceType::Bishop | PieceType::Rook | PieceType::Queen => {
                        // Sliding mobility in basic directions
                        let dirs: &[(i32,i32)] = match p.get_type() {
                            PieceType::Bishop => &[(1,1),(1,-1),(-1,1),(-1,-1)],
                            PieceType::Rook => &[(1,0),(-1,0),(0,1),(0,-1)],
                            _ => &[(1,1),(1,-1),(-1,1),(-1,-1),(1,0),(-1,0),(0,1),(0,-1)],
                        };
                        for (dr, dc) in dirs.iter() {
                            let mut nr = r as i32 + dr; let mut nc = c as i32 + dc;
                            while nr>=0 && nr<8 && nc>=0 && nc<8 {
                                if let Some(tp) = board.get(nr as usize, nc as usize) {
                                    if tp.get_color()!=color { if color==Color::White { white_moves+=1; } else { black_moves+=1; } }
                                    break;
                                } else {
                                    if color==Color::White { white_moves+=1; } else { black_moves+=1; }
                                }
                                nr += dr; nc += dc;
                            }
                        }
                    }
                    PieceType::King => {
                        for dr in -1..=1 { for dc in -1..=1 { if dr==0 && dc==0 { continue; } let nr=r as i32+dr; let nc=c as i32+dc; if nr>=0&&nr<8&&nc>=0&&nc<8 {
                            if let Some(tp)=board.get(nr as usize, nc as usize) { if tp.get_color()!=color { if color==Color::White { white_moves+=1; } else { black_moves+=1; } } } else { if color==Color::White { white_moves+=1; } else { black_moves+=1; } }
                        } }}
                    }
                    PieceType::Pawn => {
                        // Simple pawn mobility: one step forward if empty, captures diagonally if enemy
                        let dir: i32 = if color==Color::White { 1 } else { -1 };
                        let nr = r as i32 + dir;
                        if nr>=0 && nr<8 {
                            // forward
                            if board.get(nr as usize, c).is_none() { if color==Color::White { white_moves+=1; } else { black_moves+=1; } }
                            // captures
                            for dc in [-1,1] { let nc = c as i32 + dc; if nc>=0 && nc<8 {
                                if let Some(tp)=board.get(nr as usize, nc as usize) { if tp.get_color()!=color { if color==Color::White { white_moves+=1; } else { black_moves+=1; } } }
                            }}
                        }
                    }
                }
            }
        }
    }

    if white_moves > black_moves { score += tempo; }
    else if black_moves > white_moves { score -= tempo; }

    score
}
