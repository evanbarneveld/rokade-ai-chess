use crate::board::Board;
pub(crate) use crate::board::pst::{tapered_eval as taper_general};
use crate::board::pst::*;
use crate::piece::pieces::{Color, PieceType};

pub const MIN_EVAL_VALUE: i32 = i32::MIN + 100_000;
pub const MAX_EVAL_VALUE: i32 = i32::MAX - 100_000;

// Material scores (centipawns)
const PAWN: i32 = 100;
const KNIGHT: i32 = 320;
const BISHOP: i32 = 330;
const ROOK: i32 = 500;
const QUEEN: i32 = 900;
const KING: i32 = 0; // King material is not counted; PST handles its safety/activity

pub struct PawnFileCounts {
    pub white: [i32; 8],
    pub black: [i32; 8],
}

// --- Public Functions ---

/// Public evaluation function: positive = better for White; negative = better for Black
pub fn evaluate_position(board: &Board, side_to_move: Color) -> i32 {
    let ctx = EvalContext::new(board);
    let mut score: i32 = 0;

    for row in 0..8 {
        for col in 0..8 {
            if let Some(piece) = board.get(row, col) {
                let val = ctx.evaluate_piece(piece.get_type(), row, col, piece.get_color());
                match piece.get_color() {
                    Color::White => score += val,
                    Color::Black => score -= val,
                }
            }
        }
    }

    score += ctx.evaluate_hanging_pieces();
    
    // Tempo
    let tempo = (12 * ctx.phase) / 24;
    match side_to_move {
        Color::White => score += tempo,
        Color::Black => score -= tempo,
    }

    score += ctx.evaluate_mobility();
    score += ctx.evaluate_holes();
    score += ctx.evaluate_center_control();
    score += ctx.evaluate_space();
    score += ctx.evaluate_global_features();

    score = apply_drawish_tweaks(board, score);

    score
}

#[inline]
pub(crate) fn is_piece(board: &Board, r: usize, c: usize, color: Color, pt: PieceType) -> bool {
    matches!(board.get(r, c), Some(p) if p.get_color() == color && p.get_type() == pt)
}

#[inline]
pub(crate) fn is_color(board: &Board, r: usize, c: usize, color: Color) -> bool {
    matches!(board.get(r, c), Some(p) if p.get_color() == color)
}

#[inline]
pub(crate) fn get_piece_type(board: &Board, r: usize, c: usize) -> Option<PieceType> {
    board.get(r, c).map(|p| p.get_type())
}

#[inline]
pub(crate) fn material_value(piece: PieceType) -> i32 {
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
pub(crate) fn game_phase(board: &Board) -> i32 {
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

#[inline]
pub(crate) fn square_attacked_by_enemy_pawn(board: &Board, r: usize, c: usize, enemy: Color) -> bool {
    match enemy {
        Color::White => {
            // White pawns attack up: from (r-1,c-1) and (r-1,c+1) to (r,c)
            if r > 0 {
                if c > 0 && is_piece(board, r-1, c-1, Color::White, PieceType::Pawn) { return true; }
                if c < 7 && is_piece(board, r-1, c+1, Color::White, PieceType::Pawn) { return true; }
            }
        }
        Color::Black => {
            // Black pawns attack down: from (r+1,c-1) and (r+1,c+1) to (r,c)
            if r < 7 {
                if c > 0 && is_piece(board, r+1, c-1, Color::Black, PieceType::Pawn) { return true; }
                if c < 7 && is_piece(board, r+1, c+1, Color::Black, PieceType::Pawn) { return true; }
            }
        }
    }
    false
}

#[inline]
pub(crate) fn find_king(board: &Board, color: Color) -> Option<(usize, usize)> {
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c) {
                if p.get_color() == color && p.get_type() == PieceType::King { return Some((r, c)); }
            }
        }
    }
    None
}

#[inline]
pub(crate) fn opponent(color: Color) -> Color {
    match color {
        Color::White => Color::Black,
        Color::Black => Color::White,
    }
}

#[inline]
pub(crate) fn chebyshev_dist(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

// --- Private Functions and Types ---

struct EvalContext<'a> {
    board: &'a Board,
    phase: i32,
    eg: i32,
    king_w: Option<(usize, usize)>,
    king_b: Option<(usize, usize)>,
    pawn_counts: PawnFileCounts,
    att_w: [[bool; 8]; 8],
    att_b: [[bool; 8]; 8],
    white_pawns: i32,
    black_pawns: i32,
}

impl<'a> EvalContext<'a> {
    fn new(board: &'a Board) -> Self {
        let phase = game_phase(board);
        let eg = 24 - phase;
        let king_w = find_king(board, Color::White);
        let king_b = find_king(board, Color::Black);
        let pawn_counts = crate::board::evaluate_pawns::pawn_file_counts(board);
        let (att_w, att_b) = build_attack_maps(board);
        
        let mut white_pawns = 0;
        let mut black_pawns = 0;
        for f in 0..8 {
            white_pawns += pawn_counts.white[f];
            black_pawns += pawn_counts.black[f];
        }

        Self {
            board,
            phase,
            eg,
            king_w,
            king_b,
            pawn_counts,
            att_w,
            att_b,
            white_pawns,
            black_pawns,
        }
    }

    fn evaluate_piece(&self, pt: PieceType, row: usize, col: usize, color: Color) -> i32 {
        let mut val = material_value(pt) + pst_value_tapered(pt, row, col, color, self.phase);

        match pt {
            PieceType::Pawn => val += crate::board::evaluate_pawns::evaluate_pawn(self.board, row, col, color, self.phase, self.king_w, self.king_b, &self.att_w, &self.att_b),
            PieceType::Knight => val += crate::board::evaluate_knights::evaluate_knight(self.board, row, col, color, self.phase),
            PieceType::Bishop => val += crate::board::evaluate_bishops::evaluate_bishop(row, col, color, self.phase),
            PieceType::Rook => val += crate::board::evaluate_rooks::evaluate_rook(self.board, row, col, color, self.phase, self.eg, self.white_pawns, self.black_pawns),
            PieceType::Queen => val += crate::board::evaluate_queens::evaluate_queen(self.board, row, col, color, self.phase),
            _ => {}
        }

        val
    }

    fn evaluate_hanging_pieces(&self) -> i32 {
        let mut score = 0;
        for r in 0..8 {
            for c in 0..8 {
                if let Some(p) = self.board.get(r, c) {
                    let color = p.get_color();
                    let (attacked_by_opp, defended_by_own) = match color {
                        Color::White => (self.att_b[r][c], self.att_w[r][c]),
                        Color::Black => (self.att_w[r][c], self.att_b[r][c]),
                    };
                    if attacked_by_opp && !defended_by_own {
                        let base_pen = match p.get_type() {
                            PieceType::Pawn => 15,
                            PieceType::Knight | PieceType::Bishop => 30,
                            PieceType::Rook => 45,
                            PieceType::Queen => 60,
                            PieceType::King => 0,
                        };
                        let pen = (base_pen * self.phase) / 24;
                        match color {
                            Color::White => score -= pen,
                            Color::Black => score += pen,
                        }
                    }
                }
            }
        }
        score
    }

    fn evaluate_mobility(&self) -> i32 {
        let (mob_w, mob_b) = mobility_activity(self.board);
        let mut score = (mob_w * self.phase) / 24;
        score -= (mob_b * self.phase) / 24;

        if self.phase > 12 {
            let damp_w = (mob_w / 20) * (self.phase - 12) / 12;
            let damp_b = (mob_b / 20) * (self.phase - 12) / 12;
            score -= damp_w;
            score += damp_b;
        }
        score
    }

    fn evaluate_holes(&self) -> i32 {
        let mut score = 0;
        let hole_mg_pen: i32 = 10;
        for r in 2..=5 {
            for c in 2..=5 {
                // White holes
                if crate::board::evaluate_pawns::is_hole_square_limited(self.board, r, c, Color::White, self.phase) {
                    let influenced = self.att_b[r][c];
                    let occ_minor = matches!(get_piece_type(self.board, r, c), Some(PieceType::Knight | PieceType::Bishop))
                        && is_color(self.board, r, c, Color::Black);
                    if influenced || occ_minor {
                        let mut pen = hole_mg_pen;
                        if occ_minor { pen += 6; }
                        score -= (pen * self.phase) / 24;
                    }
                }
                // Black holes
                if crate::board::evaluate_pawns::is_hole_square_limited(self.board, r, c, Color::Black, self.phase) {
                    let influenced = self.att_w[r][c];
                    let occ_minor = matches!(get_piece_type(self.board, r, c), Some(PieceType::Knight | PieceType::Bishop))
                        && is_color(self.board, r, c, Color::White);
                    if influenced || occ_minor {
                        let mut pen = hole_mg_pen;
                        if occ_minor { pen += 6; }
                        score += (pen * self.phase) / 24;
                    }
                }
            }
        }
        score
    }

    fn evaluate_center_control(&self) -> i32 {
        let mut score = 0;
        const CENTER_CTRL_CP: i32 = 4;
        const CENTER_OCC_EXTRA_CP: i32 = 3;
        for &(r, c) in &[(3, 3), (3, 4), (4, 3), (4, 4)] {
            if self.att_w[r][c] { score += (CENTER_CTRL_CP * self.phase) / 24; }
            if self.att_b[r][c] { score -= (CENTER_CTRL_CP * self.phase) / 24; }
            if let Some(p) = self.board.get(r, c) {
                if matches!(p.get_type(), PieceType::Pawn | PieceType::Knight | PieceType::Bishop) {
                    let bonus = (CENTER_OCC_EXTRA_CP * self.phase) / 24;
                    match p.get_color() {
                        Color::White => score += bonus,
                        Color::Black => score -= bonus,
                    }
                }
            }
        }
        score
    }

    fn evaluate_space(&self) -> i32 {
        let mut score = 0;
        const SPACE_PAWN5_CP: i32 = 6;
        for c in 0..8 {
            // White pawn on 5th rank (r==4)
            if is_piece(self.board, 4, c, Color::White, PieceType::Pawn) {
                let safe = !square_attacked_by_enemy_pawn(self.board, 4, c, Color::Black)
                    || crate::board::evaluate_pawns::friendly_pawn_adjacent_behind_limited(self.board, 4, c, Color::White, self.phase);
                if safe { score += (SPACE_PAWN5_CP * self.phase) / 24; }
            }
            // Black pawn on 5th rank (r==3)
            if is_piece(self.board, 3, c, Color::Black, PieceType::Pawn) {
                let safe = !square_attacked_by_enemy_pawn(self.board, 3, c, Color::White)
                    || crate::board::evaluate_pawns::friendly_pawn_adjacent_behind_limited(self.board, 3, c, Color::Black, self.phase);
                if safe { score -= (SPACE_PAWN5_CP * self.phase) / 24; }
            }
        }
        score
    }

    fn evaluate_global_features(&self) -> i32 {
        let mut score = 0;

        // Bishop pair
        let (w_bishops, b_bishops) = crate::board::evaluate_bishops::count_bishops(self.board);
        if w_bishops >= 2 { score += self.taper(36, 24); }
        if b_bishops >= 2 { score -= self.taper(36, 24); }

        // Rook/Queen activity and coordination
        let w_rook_act = crate::board::evaluate_rooks::rook_file_activity(self.board, Color::White, &self.pawn_counts);
        let b_rook_act = crate::board::evaluate_rooks::rook_file_activity(self.board, Color::Black, &self.pawn_counts);
        score += (w_rook_act * self.phase) / 24;
        score -= (b_rook_act * self.phase) / 24;

        score += (crate::board::evaluate_rooks::doubled_rooks_bonus(self.board, Color::White, &self.pawn_counts) * self.phase) / 24;
        score -= (crate::board::evaluate_rooks::doubled_rooks_bonus(self.board, Color::Black, &self.pawn_counts) * self.phase) / 24;

        score += (crate::board::evaluate_rooks::rook_on_enemy_king_file_bonus(self.board, Color::White) * self.phase) / 24;
        score -= (crate::board::evaluate_rooks::rook_on_enemy_king_file_bonus(self.board, Color::Black) * self.phase) / 24;

        score += (crate::board::evaluate_queens::queen_on_semi_open_file_bonus(self.board, Color::White, &self.pawn_counts) * self.phase) / 24;
        score -= (crate::board::evaluate_queens::queen_on_semi_open_file_bonus(self.board, Color::Black, &self.pawn_counts) * self.phase) / 24;

        // King safety and activity
        score += (crate::board::evaluate_king::king_safety(self.board, Color::White) * self.phase) / 24;
        score -= (crate::board::evaluate_king::king_safety(self.board, Color::Black) * self.phase) / 24;

        score += (crate::board::evaluate_king::king_activity_endgame(self.board, Color::White) * self.eg) / 24;
        score -= (crate::board::evaluate_king::king_activity_endgame(self.board, Color::Black) * self.eg) / 24;

        // Development penalty
        if self.phase > 12 {
            score += crate::board::evaluate_king::development_penalty_on_backrank(self.board, Color::White) * (self.phase - 12) / 12;
            score -= crate::board::evaluate_king::development_penalty_on_backrank(self.board, Color::Black) * (self.phase - 12) / 12;
        }

        // Early queen penalty
        score -= (crate::board::evaluate_queens::early_queen_penalty(self.board, Color::White, &self.pawn_counts) * self.phase) / 24;
        score += (crate::board::evaluate_queens::early_queen_penalty(self.board, Color::Black, &self.pawn_counts) * self.phase) / 24;

        score
    }

    #[inline]
    fn taper(&self, mg: i32, eg: i32) -> i32 {
        taper_general(self.phase, mg, eg)
    }
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
                    PieceType::Knight => count_knight_targets(board, r, c, color) as i32 * 2,
                    PieceType::Bishop => count_slider_targets(board, r, c, color, &[(1,1),(1,-1),(-1,1),(-1,-1)]) as i32 * 3,
                    PieceType::Rook   => count_slider_targets(board, r, c, color, &[(1,0),(-1,0),(0,1),(0,-1)]) as i32 * 3,
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
fn add_knight_attacks(_board: &Board, r: usize, c: usize, color: Color, w: &mut [[bool;8];8], b: &mut [[bool;8];8]) {
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

// ---- Drawishness helpers ----

fn apply_drawish_tweaks(board: &Board, mut score: i32) -> i32 {
    // Insufficient material: no pawns, no rooks/queens, and total minors <=1 → dead draw
    let mut pawns = 0; let mut rooks = 0; let mut queens = 0; let mut minors = 0;
    let mut w_bish = 0; let mut b_bish = 0; let mut w_dark = false; let mut b_dark = false;
    for r in 0..8 { for c in 0..8 { if let Some(p)=board.get(r,c) {
        match p.get_type() {
            PieceType::Pawn => pawns += 1,
            PieceType::Rook => rooks += 1,
            PieceType::Queen => queens += 1,
            PieceType::Knight => minors += 1,
            PieceType::Bishop => {
                minors += 1;
                if p.get_color()==Color::White { w_bish += 1; w_dark |= ((r+c) % 2)==1; }
                else { b_bish += 1; b_dark |= ((r+c) % 2)==1; }
            }
            _ => {}
        }
    }}}

    if pawns==0 && rooks==0 && queens==0 && minors<=1 { return 0; }

    // Opposite-colored bishops only and no other pieces except kings and bishops (one each)
    let mut others = 0; for r in 0..8 { for c in 0..8 { if let Some(p)=board.get(r,c) {
        match p.get_type() { PieceType::King|PieceType::Bishop => {}, _ => others += 1 }
    }}}
    if others==0 && w_bish==1 && b_bish==1 {
        // Determine colors of bishops: dark if on dark square currently (approximate)
        let w_is_dark = w_dark; let b_is_dark = b_dark;
        if w_is_dark != b_is_dark {
            // Pull score toward zero (¾ factor)
            score = score - score/4;
        }
    }
    score
}
