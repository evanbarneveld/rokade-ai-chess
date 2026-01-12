use crate::board::Board;
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

#[inline]
fn is_piece(board: &Board, r: usize, c: usize, color: Color, pt: PieceType) -> bool {
    matches!(board.get(r, c), Some(p) if p.get_color() == color && p.get_type() == pt)
}

#[inline]
fn is_color(board: &Board, r: usize, c: usize, color: Color) -> bool {
    matches!(board.get(r, c), Some(p) if p.get_color() == color)
}

#[inline]
fn get_piece_type(board: &Board, r: usize, c: usize) -> Option<PieceType> {
    board.get(r, c).map(|p| p.get_type())
}

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

// Compute a simple material-based game phase: 24 = full midgame, 0 = pure endgame
struct PawnFileCounts {
    white: [i32; 8],
    black: [i32; 8],
}

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
        let pawn_counts = pawn_file_counts(board);
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

    #[inline]
    fn taper(&self, mg: i32, eg: i32) -> i32 {
        tapered_eval(mg, eg, self.phase)
    }

    fn evaluate_piece(&self, pt: PieceType, row: usize, col: usize, color: Color) -> i32 {
        let mut val = material_value(pt) + pst_value_tapered(pt, row, col, color, self.phase);

        match pt {
            PieceType::Pawn => val += self.evaluate_pawn(row, col, color),
            PieceType::Knight => val += self.evaluate_knight(row, col, color),
            PieceType::Bishop => val += self.evaluate_bishop(row, col, color),
            PieceType::Rook => val += self.evaluate_rook(row, col, color),
            PieceType::Queen => val += self.evaluate_queen(row, col, color),
            _ => {}
        }

        val
    }

    fn evaluate_pawn(&self, row: usize, col: usize, color: Color) -> i32 {
        let mut val = 0;
        // File bonuses from a..h: a/h negative, c/f small positive, d/e strong positive
        const FILE_BONUS: [i32; 8] = [-30, -10, 10, 25, 25, 10, -10, -30];
        val += (FILE_BONUS[col] * self.phase) / 24;

        // Mild penalty for advanced rook pawns in opening
        if self.phase > 12 {
            if col == 0 || col == 7 {
                let advancement = match color {
                    Color::White => row as i32,
                    Color::Black => (7 - row) as i32,
                };
                if advancement >= 3 {
                    val -= (15 * self.phase) / 24;
                }
            }
        }

        if is_doubled_pawn(self.board, row, col, color) { val -= 12; }
        if is_isolated_pawn(self.board, col, color) { val -= 14; }
        if is_backward_pawn(self.board, row, col, color) {
            val -= self.taper(22, 8);
        }
        if is_passed_pawn(self.board, row, col, color) {
            val += evaluate_passed_pawn(self.board, row, col, color, self.phase, self.king_w, self.king_b, &self.att_w, &self.att_b);
        }
        val
    }

    fn evaluate_knight(&self, row: usize, col: usize, color: Color) -> i32 {
        let mut val = 0;
        if self.phase > 0 {
            let dev_bonus = match color {
                Color::White => match (row, col) {
                    (2, 2) | (2, 5) => 6,
                    (1, 3) | (1, 4) => 4,
                    _ => 0,
                },
                Color::Black => match (row, col) {
                    (5, 2) | (5, 5) => 6,
                    (6, 3) | (6, 4) => 4,
                    _ => 0,
                },
            };
            val += (dev_bonus * self.phase) / 24;
        }
        if is_knight_outpost(self.board, row, col, color) {
            val += self.taper(22, 8);
        }
        val
    }

    fn evaluate_bishop(&self, row: usize, col: usize, color: Color) -> i32 {
        let mut val = 0;
        if self.phase > 0 {
            let home = match color {
                Color::White => row == 0 && (col == 2 || col == 5),
                Color::Black => row == 7 && (col == 2 || col == 5),
            };
            if !home { val += (8 * self.phase) / 24; }
        }
        val
    }

    fn evaluate_rook(&self, row: usize, col: usize, color: Color) -> i32 {
        let mut val = 0;
        if self.eg > 0 {
            // Rook on 7th
            let on_7th = match color {
                Color::White => row == 6 && self.black_pawns > 0,
                Color::Black => row == 1 && self.white_pawns > 0,
            };
            if on_7th { val += (30 * self.eg) / 24; }

            // Rook behind passed pawn
            if let Some((pp_r, _)) = find_passed_pawn_on_file(self.board, col, color) {
                let behind = match color { Color::White => row < pp_r, Color::Black => row > pp_r };
                if behind && file_clear_between(self.board, row, pp_r, col) {
                    let adv = match color { Color::White => pp_r as i32, Color::Black => (7 - pp_r) as i32 };
                    val += ((12 + 2 * adv) * self.eg) / 24;
                }
            }

            // Cut-off king
            if let Some((ek_r, ek_c)) = find_king(self.board, opponent(color)) {
                if col == ek_c && file_clear_between(self.board, row, ek_r, col) {
                    if (row as i32 - ek_r as i32).abs() >= 2 { val += (10 * self.eg) / 24; }
                }
                if row == ek_r && rank_clear_between(self.board, col, ek_c, row) {
                    if (col as i32 - ek_c as i32).abs() >= 2 { val += (10 * self.eg) / 24; }
                }
            }
        }
        if self.phase > 0 {
            let (is_back_rank, start_row) = match color { Color::White => (row==0, 1usize), Color::Black => (row==7, 6usize) };
            if is_back_rank {
                let left_block = col > 0 && is_piece(self.board, start_row, col-1, color, PieceType::Pawn);
                let right_block = col < 7 && is_piece(self.board, start_row, col+1, color, PieceType::Pawn);
                if left_block && right_block { val -= (16 * self.phase) / 24; }
            }
        }
        val
    }

    fn evaluate_queen(&self, row: usize, col: usize, color: Color) -> i32 {
        let mut val = 0;
        if self.phase > 0 {
            if col == 0 || col == 7 {
                let deep = match color { Color::White => row >= 3, Color::Black => row <= 4 };
                if deep { val -= (12 * self.phase) / 24; }
            }
            let (back_r, k1c, k2c) = match color { Color::White => (0, 1, 6), Color::Black => (7, 1, 6) };
            let both_knights_back = is_piece(self.board, back_r, k1c, color, PieceType::Knight)
                && is_piece(self.board, back_r, k2c, color, PieceType::Knight);
            if both_knights_back {
                let shallow = match color { Color::White => row <= 2, Color::Black => row >= 5 };
                if shallow { val -= (14 * self.phase) / 24; }
            }
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
                if is_hole_square_limited(self.board, r, c, Color::White, self.phase) {
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
                if is_hole_square_limited(self.board, r, c, Color::Black, self.phase) {
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
                    || friendly_pawn_adjacent_behind_limited(self.board, 4, c, Color::White, self.phase);
                if safe { score += (SPACE_PAWN5_CP * self.phase) / 24; }
            }
            // Black pawn on 5th rank (r==3)
            if is_piece(self.board, 3, c, Color::Black, PieceType::Pawn) {
                let safe = !square_attacked_by_enemy_pawn(self.board, 3, c, Color::White)
                    || friendly_pawn_adjacent_behind_limited(self.board, 3, c, Color::Black, self.phase);
                if safe { score -= (SPACE_PAWN5_CP * self.phase) / 24; }
            }
        }
        score
    }

    fn evaluate_global_features(&self) -> i32 {
        let mut score = 0;

        // Bishop pair
        let (w_bishops, b_bishops) = count_bishops(self.board);
        if w_bishops >= 2 { score += self.taper(36, 24); }
        if b_bishops >= 2 { score -= self.taper(36, 24); }

        // Rook/Queen activity and coordination
        let w_rook_act = rook_file_activity(self.board, Color::White, &self.pawn_counts);
        let b_rook_act = rook_file_activity(self.board, Color::Black, &self.pawn_counts);
        score += (w_rook_act * self.phase) / 24;
        score -= (b_rook_act * self.phase) / 24;

        score += (doubled_rooks_bonus(self.board, Color::White, &self.pawn_counts) * self.phase) / 24;
        score -= (doubled_rooks_bonus(self.board, Color::Black, &self.pawn_counts) * self.phase) / 24;

        score += (rook_on_enemy_king_file_bonus(self.board, Color::White) * self.phase) / 24;
        score -= (rook_on_enemy_king_file_bonus(self.board, Color::Black) * self.phase) / 24;

        score += (queen_on_semi_open_file_bonus(self.board, Color::White, &self.pawn_counts) * self.phase) / 24;
        score -= (queen_on_semi_open_file_bonus(self.board, Color::Black, &self.pawn_counts) * self.phase) / 24;

        // King safety and activity
        score += (king_safety(self.board, Color::White) * self.phase) / 24;
        score -= (king_safety(self.board, Color::Black) * self.phase) / 24;

        score += (king_activity_endgame(self.board, Color::White) * self.eg) / 24;
        score -= (king_activity_endgame(self.board, Color::Black) * self.eg) / 24;

        // Development penalty
        if self.phase > 12 {
            score += development_penalty_on_backrank(self.board, Color::White) * (self.phase - 12) / 12;
            score -= development_penalty_on_backrank(self.board, Color::Black) * (self.phase - 12) / 12;
        }

        // Early queen penalty
        score -= (early_queen_penalty(self.board, Color::White, &self.pawn_counts) * self.phase) / 24;
        score += (early_queen_penalty(self.board, Color::Black, &self.pawn_counts) * self.phase) / 24;

        score
    }
}

#[inline]
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

#[inline]
fn has_enemy_pawn_ahead_same_file(board: &Board, row: usize, col: usize, color: Color) -> bool {
    let dir: i32 = if color==Color::White { 1 } else { -1 };
    let mut r = row as i32 + dir;
    while r>=0 && r<8 {
        if let Some(p)=board.get(r as usize, col) {
            if p.get_color()!=color && matches!(p.get_type(), PieceType::Pawn) { return true; }
        }
        r += dir;
    }
    false
}

#[inline]
fn friendly_pawn_adjacent_behind_limited(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> bool {
    // Consider only the nearest pawn on adjacent files within a distance cap depending on phase
    let cap = if phase >= 12 { 2 } else { 4 };
    for dc in [-1i32, 1] {
        let nc_i = col as i32 + dc; if nc_i < 0 || nc_i > 7 { continue; }
        let nc = nc_i as usize;
        let mut best_dist: Option<i32> = None;
        for r in 0..8 {
            if let Some(p) = board.get(r, nc) { if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn) {
                let d = match color { Color::White => row as i32 - r as i32, Color::Black => r as i32 - row as i32 };
                if d >= 0 { if best_dist.map_or(true, |bd| d < bd) { best_dist = Some(d); } }
            }}
        }
        if let Some(d) = best_dist { if d <= cap { return true; } }
    }
    false
}

#[inline]
fn square_attacked_by_enemy_pawn(board: &Board, r: usize, c: usize, enemy: Color) -> bool {
    match enemy {
        Color::White => {
            // White pawns attack up: from (r-1,c-1) and (r-1,c+1) to (r,c)
            if r>0 {
                if c>0 { if let Some(p)=board.get(r-1, c-1) { if p.get_color()==Color::White && matches!(p.get_type(), PieceType::Pawn) { return true; } } }
                if c<7 { if let Some(p)=board.get(r-1, c+1) { if p.get_color()==Color::White && matches!(p.get_type(), PieceType::Pawn) { return true; } } }
            }
            false
        }
        Color::Black => {
            // Black pawns attack down: from (r+1,c-1) and (r+1,c+1) to (r,c)
            if r<7 {
                if c>0 { if let Some(p)=board.get(r+1, c-1) { if p.get_color()==Color::Black && matches!(p.get_type(), PieceType::Pawn) { return true; } } }
                if c<7 { if let Some(p)=board.get(r+1, c+1) { if p.get_color()==Color::Black && matches!(p.get_type(), PieceType::Pawn) { return true; } } }
            }
            false
        }
    }
}

// Conservative backward pawn detection:
// - not passed
// - enemy pawn ahead on the same file
// - front square is blocked by enemy piece OR controlled by enemy pawn
// - no friendly pawn on adjacent files behind that can support
#[inline]
fn is_backward_pawn(board: &Board, row: usize, col: usize, color: Color) -> bool {
    if !matches!(board.get(row,col).map(|p| p.get_type()), Some(PieceType::Pawn)) { return false; }
    if is_passed_pawn(board, row, col, color) { return false; }
    if !has_enemy_pawn_ahead_same_file(board, row, col, color) { return false; }
    let dir: i32 = if color==Color::White { 1 } else { -1 };
    let fr_i = row as i32 + dir; if fr_i < 0 || fr_i > 7 { return false; }
    let fr = fr_i as usize;
    let front_blocked_by_enemy = match board.get(fr, col) { Some(p) if p.get_color()!=color => true, _ => false };
    let front_controlled_by_enemy_pawn = square_attacked_by_enemy_pawn(board, fr, col, opponent(color));
    if !(front_blocked_by_enemy || front_controlled_by_enemy_pawn) { return false; }
    // Use a conservative phase cap (MG≤2, EG≤4). Without phase here, approximate with board phase.
    let phase = game_phase(board);
    if friendly_pawn_adjacent_behind_limited(board, row, col, color, phase) { return false; }
    true
}

// Knight outpost detection: protected by own pawn and cannot be chased by an enemy pawn (no enemy pawn can attack the square now or from behind on adjacent files)
fn is_knight_outpost(board: &Board, row: usize, col: usize, color: Color) -> bool {
    if !matches!(board.get(row,col).map(|p| p.get_type()), Some(PieceType::Knight)) { return false; }
    // Must be protected by own pawn
    let own_pawn_protects = match color {
        Color::White => {
            (row>0 && col>0 && matches!(board.get(row-1,col-1), Some(p) if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn))) ||
            (row>0 && col<7 && matches!(board.get(row-1,col+1), Some(p) if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn)))
        }
        Color::Black => {
            (row<7 && col>0 && matches!(board.get(row+1,col-1), Some(p) if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn))) ||
            (row<7 && col<7 && matches!(board.get(row+1,col+1), Some(p) if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn)))
        }
    };
    if !own_pawn_protects { return false; }
    // Not chasable by enemy pawn: (a) not currently attacked by enemy pawn; (b) no enemy pawn on adjacent files behind relative to enemy advance
    let enemy = opponent(color);
    if square_attacked_by_enemy_pawn(board, row, col, enemy) { return false; }
    for dc in [-1i32, 1] {
        let nc_i = col as i32 + dc; if nc_i < 0 || nc_i > 7 { continue; }
        let nc = nc_i as usize;
        match enemy {
            Color::White => {
                // any white pawn behind the square (lower row index) on an adjacent file could advance to attack later
                for r in 0..row { if let Some(p)=board.get(r,nc) { if p.get_color()==Color::White && matches!(p.get_type(), PieceType::Pawn) { return false; } } }
            }
            Color::Black => {
                for r in (row+1)..8 { if let Some(p)=board.get(r,nc) { if p.get_color()==Color::Black && matches!(p.get_type(), PieceType::Pawn) { return false; } } }
            }
        }
    }
    true
}

fn count_bishops(board: &Board) -> (i32, i32) {
    let mut w = 0;
    let mut b = 0;
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r, c) {
                if matches!(p.get_type(), PieceType::Bishop) {
                    if p.get_color() == Color::White {
                        w += 1;
                    } else {
                        b += 1;
                    }
                }
            }
        }
    }
    (w, b)
}

#[inline]
fn pawn_file_counts(board: &Board) -> PawnFileCounts {
    let mut w = [0i32;8];
    let mut b = [0i32;8];
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = board.get(r,c) {
                if matches!(p.get_type(), PieceType::Pawn) {
                    if p.get_color()==Color::White { w[c] += 1; } else { b[c] += 1; }
                }
            }
        }
    }
    PawnFileCounts { white: w, black: b }
}

fn rook_file_activity(board: &Board, color: Color, counts: &PawnFileCounts) -> i32 {
    let mut bonus = 0;
    for r in 0..8 { for c in 0..8 {
        if let Some(p)=board.get(r,c) { if p.get_color()==color && matches!(p.get_type(), PieceType::Rook) {
            let wp = counts.white[c];
            let bp = counts.black[c];
            let open = wp==0 && bp==0;
            let semi = match color { Color::White => wp==0 && bp>0, Color::Black => bp==0 && wp>0 };
            if open { bonus += 14; } else if semi { bonus += 8; }
        }}
    }}
    bonus
}

#[inline]
// file_pawn_counts removed in favor of cached PawnFileCounts

// Bonus for doubled rooks on the same file; slightly higher on (semi-)open files.
fn doubled_rooks_bonus(board: &Board, color: Color, counts: &PawnFileCounts) -> i32 {
    let mut bonus = 0;
    for file in 0..8 {
        let mut count = 0;
        for r in 0..8 {
            if let Some(p)=board.get(r,file) {
                if p.get_color()==color && matches!(p.get_type(), PieceType::Rook) { count += 1; }
            }
        }
        if count >= 2 {
            // Base for doubled rooks
            let mut b = 10;
            let wp = counts.white[file];
            let bp = counts.black[file];
            let open = wp==0 && bp==0;
            let semi = match color { Color::White => wp==0 && bp>0, Color::Black => bp==0 && wp>0 };
            if open { b += 6; } else if semi { b += 3; }
            bonus += b;
        }
    }
    bonus
}

// Bonus if a rook is on the same file as the enemy king with empty squares between
// (classic pressure motif). Counts each qualifying rook.
fn rook_on_enemy_king_file_bonus(board: &Board, color: Color) -> i32 {
    let mut bonus = 0;
    if let Some((ek_r, ek_f)) = find_king(board, opponent(color)) {
        for r in 0..8 { if let Some(p)=board.get(r, ek_f) {
            if p.get_color()==color && matches!(p.get_type(), PieceType::Rook) {
                if file_clear_between(board, r, ek_r, ek_f) {
                    bonus += 10;
                }
            }
        }}
    }
    bonus
}

// Middlegame bonus for queen on a semi-open file (encourages useful central/semi-open placement
// without overcommitting to early queen activity).
fn queen_on_semi_open_file_bonus(board: &Board, color: Color, counts: &PawnFileCounts) -> i32 {
    // Should be negligible in the opening and only start to matter later.
    // Use an inverse phase weight so that it grows toward endgame.
    let phase = game_phase(board);
    let later_scale = (24 - phase).clamp(0, 24); // 0 in pure opening, 24 in pure endgame
    let mut bonus = 0;
    for r in 0..8 { for c in 0..8 {
        if let Some(p)=board.get(r,c) {
            if p.get_color()==color && matches!(p.get_type(), PieceType::Queen) {
                let wp = counts.white[c];
                let bp = counts.black[c];
                let semi = match color { Color::White => wp==0 && bp>0, Color::Black => bp==0 && wp>0 };
                if semi { bonus += (6 * later_scale) / 24; }
            }
        }
    }}
    bonus
}

// Penalize early queen development if minor pieces are still on the back rank.
fn early_queen_penalty(board: &Board, color: Color, counts: &PawnFileCounts) -> i32 {
    // Identify queen home square and back rank
    let (home_r, home_c) = match color { Color::White => (0usize, 3usize), Color::Black => (7usize, 3usize) };
    // If the queen is still at home, no penalty
    if matches!(board.get(home_r, home_c), Some(p) if p.get_color()==color && matches!(p.get_type(), PieceType::Queen)) {
        return 0;
    }
    // Count undeveloped minors on the back rank (knights/bishops still on back rank squares)
    let mut undeveloped = 0;
    let back_r = home_r;
    for c in 0..8 {
        if let Some(p)=board.get(back_r, c) {
            if p.get_color()==color {
                match p.get_type() {
                    PieceType::Knight | PieceType::Bishop => { undeveloped += 1; }
                    _ => {}
                }
            }
        }
    }
    if undeveloped == 0 { return 0; }
    // Base penalty scales with how undeveloped the position is (stronger to curb early queen sorties)
    let base = if undeveloped >= 3 { 48 } else if undeveloped == 2 { 36 } else { 24 };
    // Slightly increase if queen has advanced beyond the back rank (always true here),
    // and not shielded by pawns in front (crude heuristic: open a file at a queen file)
    // Find queen square
    let mut extra = 0;
    let mut queen_pos: Option<(usize,usize)> = None;
    'outer: for r in 0..8 { for c in 0..8 {
        if let Some(p)=board.get(r,c) { if p.get_color()==color && matches!(p.get_type(), PieceType::Queen) {
            queen_pos = Some((r,c));
            let wp = counts.white[c];
            let bp = counts.black[c];
            let open = wp==0 && bp==0;
            if open { extra += 8; }
            break 'outer;
        }}
    }}
    // Additional penalty if queen is advanced deeply into the board early
    if let Some((qr, _qc)) = queen_pos {
        let advanced = match color { Color::White => qr >= 3, Color::Black => qr <= 4 };
        if advanced { extra += 8; }
    }
    // Scale by opening phase (strongest in opening, zero in EG)
    base + extra
}

#[inline]
fn find_king(board: &Board, color: Color) -> Option<(usize,usize)> {
    for r in 0..8 { for c in 0..8 { if let Some(p)=board.get(r,c) { if matches!(p.get_type(), PieceType::King) && p.get_color()==color { return Some((r,c)); } } } }
    None
}

fn king_safety(board: &Board, color: Color) -> i32 {
    if let Some((kr, kf)) = find_king(board, color) {
        // 1) Pawn shelter quality around the king using actual king rank: immediate front rank and the next rank at half weight.
        let (front1, front2_opt) = match color {
            Color::White => {
                let f1 = if kr < 7 { Some(kr + 1) } else { None };
                let f2 = if kr < 6 { Some(kr + 2) } else { None };
                (f1, f2)
            }
            Color::Black => {
                let f1 = if kr > 0 { Some(kr - 1) } else { None };
                let f2 = if kr > 1 { Some(kr - 2) } else { None };
                (f1, f2)
            }
        };
        let mut shield2x = 0; // double-weight units to allow half weights as integers
        for df in -1..=1 {
            let f = kf as i32 + df; if f<0 || f>7 { continue; }
            if let Some(r1) = front1 { if let Some(p)=board.get(r1, f as usize) { if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn) { shield2x += 2; } } }
            if let Some(r2) = front2_opt { if let Some(p)=board.get(r2, f as usize) { if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn) { shield2x += 1; } } }
        }
        // Convert to buckets roughly comparable with the previous scale
        let mut pen = 0;
        if shield2x <= 1 { pen += 30; } else if shield2x <= 3 { pen += 18; } else if shield2x <= 5 { pen += 8; }

        // Half-open king file penalty
        let mut own=0; let mut opp=0; for r in 0..8 { if let Some(p)=board.get(r, kf) { if matches!(p.get_type(), PieceType::Pawn) { if p.get_color()==color { own+=1; } else { opp+=1; } } } }
        if own==0 && opp>0 { pen += 14; }

        // 2) King-ring attacker count (3x3 around king)
        let (att_w, att_b) = build_attack_maps(board);
        let enemy = opponent(color);
        let mut danger = 0;
        // piece presence map around king to weigh attackers by proximity rays
        for dr in -1..=1 { for dc in -1..=1 { if dr==0 && dc==0 { continue; }
            let nr = kr as i32 + dr; let nc = kf as i32 + dc; if nr<0||nr>=8||nc<0||nc>=8 { continue; }
            let r = nr as usize; let c = nc as usize;
            let attacked = match enemy { Color::White => att_w[r][c], Color::Black => att_b[r][c] };
            if attacked { danger += 3; } // base per controlled square around king
        }}

        // Add directional openness penalties: open/half-open adjacent files next to king
        for df in [-1i32,0,1] {
            let file = kf as i32 + df; if file < 0 || file > 7 { continue; }
            let mut ownp=0; let mut oppp=0; for r in 0..8 { if let Some(p)=board.get(r, file as usize) { if matches!(p.get_type(), PieceType::Pawn) { if p.get_color()==color { ownp+=1; } else { oppp+=1; } } } }
            if ownp==0 && oppp==0 { danger += 6; } // open file near king
            else if ownp==0 && oppp>0 { danger += 10; } // half-open against king
        }

        // Add light attacker weighting by nearby enemy heavy/minor pieces (chebyshev <=3), with a modest cap
        let mut extra_danger = 0;
        for r in 0..8 { for c in 0..8 { if let Some(p)=board.get(r,c) {
            if p.get_color()==enemy {
                let d = chebyshev_dist((kr as i32, kf as i32), (r as i32, c as i32));
                if d <= 3 {
                    let w = match p.get_type() { PieceType::Knight|PieceType::Bishop|PieceType::Rook => 2, PieceType::Queen => 3, _ => 0 };
                    extra_danger += w;
                }
            }
        }}}
        if extra_danger > 12 { extra_danger = 12; }
        danger += extra_danger;

        // 3) Castling status: stronger in opening, fades later
        let phase = game_phase(board);
        let castled_raw = if (color==Color::White && kr==0 && (kf==6 || kf==2)) || (color==Color::Black && kr==7 && (kf==6 || kf==2)) { 16 } else { 0 };
        let castled_bonus = (castled_raw * phase) / 24;

        // Combine: safety score is a negative penalty plus bonus for being castled
        return castled_bonus - (pen + danger);
    }
    0
}

#[inline]
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
    let mut pen = 0; for &(r,c) in minors.iter() { if let Some(p)=board.get(r,c) { match p.get_type() { PieceType::Knight | PieceType::Bishop => pen += 14, _ => {} } } }
    let phase = game_phase(board);
    -((pen * phase) / 24)
}

// ---- New endgame helpers ----

#[inline]
fn opponent(color: Color) -> Color { if matches!(color, Color::White) { Color::Black } else { Color::White } }

#[inline]
fn chebyshev_dist(a: (i32,i32), b: (i32,i32)) -> i32 { (a.0 - b.0).abs().max((a.1 - b.1).abs()) }

// Centralized passed pawn evaluation (single source of truth)
// Tapers naturally with phase and caps per-pawn contribution to keep stability.
fn evaluate_passed_pawn(
    board: &Board,
    row: usize,
    col: usize,
    color: Color,
    phase: i32,
    king_w: Option<(usize, usize)>,
    king_b: Option<(usize, usize)>,
    att_w: &[[bool;8];8],
    att_b: &[[bool;8];8],
) -> i32 {
    let eg = 24 - phase;

    // Advancement from home side (0..7)
    let adv: i32 = match color { Color::White => row as i32, Color::Black => (7 - row) as i32 };

    // Base value: slightly weighted more in endgame, modest in middlegame
    // mg_base: 4 + 3*adv; eg_base: 6 + 4*adv
    let mg_base = 4 + 3 * adv;
    let eg_base = 6 + 4 * adv;
    let mut score = (mg_base * phase + eg_base * eg) / 24;

    // Clear path toward promotion (no blockers on same file ahead of the pawn)
    if has_clear_promotion_path(board, row, col, color) {
        // Endgame-weighted
        score += (8 * eg) / 24;
    }

    // Close-to-promotion extra (adv >= 5 roughly 6th/7th rank)
    let close_bonus = (adv.saturating_sub(4) * 6).max(0);
    score += (close_bonus * eg) / 24;

    // Immediate block in front reduces value (more in EG)
    let next_r_opt = match color {
        Color::White => if row < 7 { Some(row + 1) } else { None },
        Color::Black => if row > 0 { Some(row - 1) } else { None },
    };
    if let Some(nr) = next_r_opt {
        if board.get(nr, col).is_some() {
            score -= (14 * eg) / 24;
        } else {
            // Free-push incentive: next square empty and not obviously unsafe (enemy king right in front)
            let enemy_king = match color { Color::White => king_b, Color::Black => king_w };
            let mut safe_bonus = 10;
            if let Some((ek_r, ek_c)) = enemy_king {
                let dr = (ek_r as i32 - nr as i32).abs();
                let dc = (ek_c as i32 - col as i32).abs();
                let in_front = (color == Color::White && ek_r >= nr) || (color == Color::Black && ek_r <= nr);
                if dr <= 1 && dc <= 1 && in_front { safe_bonus = 0; }
            }
            // Reduce/zero bonus if the push square is attacked by enemy pieces or enemy pawns
            let enemy = opponent(color);
            let attacked_general = match enemy { Color::White => att_w[nr][col], Color::Black => att_b[nr][col] };
            if attacked_general || square_attacked_by_enemy_pawn(board, nr, col, enemy) {
                // If unsafe, make the bonus very small
                safe_bonus = safe_bonus.min(2);
            }
            score += (safe_bonus * eg) / 24;
        }
    }

    // Support: protected by a pawn diagonally behind and/or connected pawns on same rank adjacent
    let mut support = 0;
    let behind_r_opt = match color { Color::White => row.checked_sub(1), Color::Black => if row < 7 { Some(row + 1) } else { None } };
    if let Some(br) = behind_r_opt {
        for dc in [-1i32, 1] {
            let nc_i = col as i32 + dc; if nc_i < 0 || nc_i > 7 { continue; }
            let nc = nc_i as usize;
            if let Some(p) = board.get(br, nc) { if p.get_color() == color && matches!(p.get_type(), PieceType::Pawn) { support += 1; } }
        }
    }
    for dc in [-1i32, 1] {
        let nc_i = col as i32 + dc; if nc_i < 0 || nc_i > 7 { continue; }
        let nc = nc_i as usize;
        if let Some(p) = board.get(row, nc) { if p.get_color() == color && matches!(p.get_type(), PieceType::Pawn) { support += 1; } }
    }
    if support > 0 { score += (8 * support as i32 * eg) / 24; }

    // King proximity/blocking
    if let (Some((fk_r, fk_c)), Some((ek_r, ek_c))) = (
        match color { Color::White => king_w, Color::Black => king_b },
        match color { Color::White => king_b, Color::Black => king_w },
    ) {
        let pawn_sq = (row as i32, col as i32);
        let fk_d = chebyshev_dist((fk_r as i32, fk_c as i32), pawn_sq);
        let ek_d = chebyshev_dist((ek_r as i32, ek_c as i32), pawn_sq);
        let prox = (12 - 2 * fk_d).max(0);
        score += (prox * eg) / 48;
        if is_king_in_front_of_pawn((ek_r, ek_c), row, col, color) && ek_d <= fk_d {
            let block_pen = (14 - 2 * (ek_d as i32)).max(0);
            score -= (block_pen * eg) / 48;
        }
    }

    // Cap per-passer contribution
    let cap: i32 = 90;
    if score > cap { cap } else { score }
}

// Check if all squares from the pawn to the promotion rank are empty (excluding the current square)
#[inline]
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

// True if the enemy king stands on a square in front of the pawn along its file or adjacent files ahead
#[inline]
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
#[inline]
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
#[inline]
fn file_clear_between(board: &Board, r1: usize, r2: usize, file: usize) -> bool {
    if r1==r2 { return true; }
    let (lo, hi) = if r1 < r2 { (r1+1, r2-1) } else { (r2+1, r1-1) };
    if hi < lo { return true; }
    for r in lo..=hi { if board.get(r, file).is_some() { return false; } }
    true
}

// Check that squares strictly between c1 and c2 on the same rank are empty
#[inline]
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

// ---- Hole (weak square) detection ----
#[inline]
fn is_hole_square(board: &Board, row: usize, col: usize, color: Color) -> bool {
    // A simplified hole: square in a central band not controllable by own pawn now nor by a pawn from behind on adjacent files
    // Quick current control test
    let own = color;
    let cur_ctrl = match own {
        Color::White => {
            if row>0 {
                (col>0 && matches!(board.get(row-1,col-1), Some(p) if p.get_color()==own && matches!(p.get_type(), PieceType::Pawn))) ||
                (col<7 && matches!(board.get(row-1,col+1), Some(p) if p.get_color()==own && matches!(p.get_type(), PieceType::Pawn)))
            } else { false }
        }
        Color::Black => {
            if row<7 {
                (col>0 && matches!(board.get(row+1,col-1), Some(p) if p.get_color()==own && matches!(p.get_type(), PieceType::Pawn))) ||
                (col<7 && matches!(board.get(row+1,col+1), Some(p) if p.get_color()==own && matches!(p.get_type(), PieceType::Pawn)))
            } else { false }
        }
    };
    if cur_ctrl { return false; }
    // Is there any own pawn on adjacent files behind the square that could, in principle, advance to control it?
    for dc in [-1i32, 1] {
        let nc_i = col as i32 + dc; if nc_i < 0 || nc_i > 7 { continue; }
        let nc = nc_i as usize;
        match color {
            Color::White => {
                // any white pawn strictly behind the square on an adjacent file
                for r in 0..row { if let Some(p)=board.get(r, nc) { if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn) { return false; } } }
            }
            Color::Black => {
                for r in (row+1)..8 { if let Some(p)=board.get(r, nc) { if p.get_color()==color && matches!(p.get_type(), PieceType::Pawn) { return false; } } }
            }
        }
    }
    true
}

#[inline]
fn is_hole_square_limited(board: &Board, row: usize, col: usize, color: Color, phase: i32) -> bool {
    // Not a hole if currently controllable by own pawn
    if !is_hole_square(board, row, col, color) { return false; }
    // Distance caps for potential future control by an advancing pawn on adjacent files
    let cap = if phase >= 12 { 2 } else { 4 };
    for dc in [-1i32, 1] {
        let nc_i = col as i32 + dc; if nc_i < 0 || nc_i > 7 { continue; }
        let nc = nc_i as usize;
        match color {
            Color::White => {
                // nearest white pawn strictly behind the target square on file nc
                let mut nearest: Option<usize> = None;
                for r in (0..row).rev() { if let Some(p)=board.get(r,nc) {
                    if p.get_color()==Color::White && matches!(p.get_type(), PieceType::Pawn) { nearest = Some(r); break; }
                }}
                if let Some(pr) = nearest {
                    let dist = row as i32 - pr as i32;
                    if dist <= cap as i32 {
                        // ensure no own pawn on same file between pr and row that blocks its advance
                        let mut blocked = false;
                        for br in (pr+1)..row { if let Some(p)=board.get(br, nc) { if p.get_color()==Color::White && matches!(p.get_type(), PieceType::Pawn) { blocked = true; break; } } }
                        if !blocked { return false; }
                    }
                }
            }
            Color::Black => {
                let mut nearest: Option<usize> = None;
                for r in (row+1)..8 { if let Some(p)=board.get(r,nc) {
                    if p.get_color()==Color::Black && matches!(p.get_type(), PieceType::Pawn) { nearest = Some(r); break; }
                }}
                if let Some(pr) = nearest {
                    let dist = pr as i32 - row as i32;
                    if dist <= cap as i32 {
                        let mut blocked = false;
                        for br in (row+1)..pr { if let Some(p)=board.get(br, nc) { if p.get_color()==Color::Black && matches!(p.get_type(), PieceType::Pawn) { blocked = true; break; } } }
                        if !blocked { return false; }
                    }
                }
            }
        }
    }
    true
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
