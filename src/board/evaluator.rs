use crate::board::Board;
use crate::board::attack_maps::build_attack_maps;
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

// Phase weights for game phase calculation (total = 24 at start)
const PHASE_KNIGHT: i32 = 1;
const PHASE_BISHOP: i32 = 1;
const PHASE_ROOK: i32 = 2;
const PHASE_QUEEN: i32 = 4;

pub struct PawnFileCounts {
    pub white: [i32; 8],
    pub black: [i32; 8],
}

pub struct FileClearance {
    /// For each file, stores ranges that are clear of pieces
    /// Used for rook evaluation optimization
    pub files: [Vec<(usize, usize)>; 8],
}

impl FileClearance {
    pub fn new(board: &Board) -> Self {
        let mut files = [const { Vec::new() }; 8];

        for col in 0..8 {
            let mut clear_start = 0;
            for row in 0..8 {
                if board.get(row, col).is_some() {
                    if row > clear_start {
                        files[col].push((clear_start, row));
                    }
                    clear_start = row + 1;
                } else if row == 7 && clear_start <= 7 {
                    files[col].push((clear_start, 8));
                }
            }
            if clear_start < 8 {
                files[col].push((clear_start, 8));
            }
        }

        Self { files }
    }

    #[inline]
    pub fn is_clear_between(&self, r1: usize, r2: usize, file: usize) -> bool {
        let start = r1.min(r2) + 1;
        let end = r1.max(r2);

        if start >= end {
            return true;
        }

        for &(clear_start, clear_end) in &self.files[file] {
            if clear_start <= start && clear_end >= end {
                return true;
            }
        }
        false
    }
}

// ============================================================
// PUBLIC API
// ============================================================

/// Public evaluation function: positive = better for White; negative = better for Black
pub fn evaluate_position(board: &Board, side_to_move: Color) -> i32 {
    let ctx = EvalContext::new(board);
    let mut score = 0;

    // Evaluate all pieces
    for row in 0..8 {
        for col in 0..8 {
            if let Some(piece) = board.get(row, col) {
                let val = ctx.evaluate_piece(piece.get_type(), row, col, piece.get_color());
                score += apply_color_score(val, piece.get_color());
            }
        }
    }

    // Tempo bonus
    let tempo = (12 * ctx.phase()) / 24;
    score += apply_color_score(tempo, side_to_move);

    score += ctx.evaluate_hanging_pieces();
    score += ctx.evaluate_mobility();
    score += ctx.evaluate_holes();
    score += ctx.evaluate_center_control();
    score += ctx.evaluate_space();
    score += ctx.evaluate_global_features();
    score += ctx.evaluate_piece_interactions();

    apply_drawish_tweaks(&ctx.stats, score)
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

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
pub(crate) fn square_attacked_by_enemy_pawn(board: &Board, r: usize, c: usize, enemy: Color) -> bool {
    match enemy {
        Color::White => {
            if r > 0 {
                if c > 0 && is_piece(board, r-1, c-1, Color::White, PieceType::Pawn) { return true; }
                if c < 7 && is_piece(board, r-1, c+1, Color::White, PieceType::Pawn) { return true; }
            }
        }
        Color::Black => {
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
            if let Some(p) = board.get(r, c)
                && p.get_color() == color
                && p.get_type() == PieceType::King
            {
                return Some((r, c));
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

/// Apply score from White's perspective (+) or Black's perspective (-)
#[inline]
fn apply_color_score(score: i32, color: Color) -> i32 {
    match color {
        Color::White => score,
        Color::Black => -score,
    }
}

// ============================================================
// BOARD STATISTICS (Single Pass)
// ============================================================

/// Statistics gathered in a single board scan
struct BoardStats {
    phase: i32,
    white_pawns: i32,
    black_pawns: i32,
    white_bishop_on_dark: bool,
    black_bishop_on_dark: bool,
    drawish_material: MaterialDrawishness,
}

#[derive(Default)]
struct MaterialDrawishness {
    pawns: i32,
    rooks: i32,
    queens: i32,
    minors: i32,
    white_bishops: i32,
    black_bishops: i32,
}

impl BoardStats {
    fn gather(board: &Board) -> Self {
        let mut phase = 0;
        let mut white_pawns = 0;
        let mut black_pawns = 0;
        let mut white_bishop_on_dark = false;
        let mut black_bishop_on_dark = false;
        let mut drawish = MaterialDrawishness::default();

        for r in 0..8 {
            for c in 0..8 {
                if let Some(p) = board.get(r, c) {
                    let pt = p.get_type();
                    let color = p.get_color();

                    // Phase
                    phase += match pt {
                        PieceType::Knight => PHASE_KNIGHT,
                        PieceType::Bishop => PHASE_BISHOP,
                        PieceType::Rook => PHASE_ROOK,
                        PieceType::Queen => PHASE_QUEEN,
                        _ => 0,
                    };

                    // Piece-specific stats
                    match pt {
                        PieceType::Pawn => {
                            if color == Color::White {
                                white_pawns += 1;
                            } else {
                                black_pawns += 1;
                            }
                            drawish.pawns += 1;
                        }
                        PieceType::Bishop => {
                            if color == Color::White {
                                drawish.white_bishops += 1;
                                if (r + c) % 2 == 1 {
                                    white_bishop_on_dark = true;
                                }
                            } else {
                                drawish.black_bishops += 1;
                                if (r + c) % 2 == 1 {
                                    black_bishop_on_dark = true;
                                }
                            }
                            drawish.minors += 1;
                        }
                        PieceType::Knight => {
                            drawish.minors += 1;
                        }
                        PieceType::Rook => {
                            drawish.rooks += 1;
                        }
                        PieceType::Queen => {
                            drawish.queens += 1;
                        }
                        _ => {}
                    }
                }
            }
        }

        Self {
            phase: phase.clamp(0, 24),
            white_pawns,
            black_pawns,
            white_bishop_on_dark,
            black_bishop_on_dark,
            drawish_material: drawish,
        }
    }

    fn is_insufficient_material(&self) -> bool {
        let d = &self.drawish_material;
        d.pawns == 0 && d.rooks == 0 && d.queens == 0 && d.minors <= 1
    }

    fn is_opposite_bishops_only(&self) -> bool {
        let d = &self.drawish_material;
        d.white_bishops == 1
            && d.black_bishops == 1
            && d.pawns == 0
            && d.rooks == 0
            && d.queens == 0
            && d.minors == 2
    }
}

// ============================================================
// EVAL CONTEXT
// ============================================================

struct EvalContext<'a> {
    board: &'a Board,
    stats: BoardStats,
    eg: i32,
    king_w: Option<(usize, usize)>,
    king_b: Option<(usize, usize)>,
    pawn_counts: PawnFileCounts,
    att_w: [[bool; 8]; 8],
    att_b: [[bool; 8]; 8],
    file_clearance: FileClearance,
}

impl<'a> EvalContext<'a> {
    fn new(board: &'a Board) -> Self {
        let stats = BoardStats::gather(board);
        let eg = 24 - stats.phase;
        let king_w = find_king(board, Color::White);
        let king_b = find_king(board, Color::Black);
        let pawn_counts = crate::board::evaluators::evaluate_pawns::pawn_file_counts(board);
        let (att_w, att_b) = build_attack_maps(board);
        let file_clearance = FileClearance::new(board);

        Self {
            board,
            stats,
            eg,
            king_w,
            king_b,
            pawn_counts,
            att_w,
            att_b,
            file_clearance,
        }
    }

    fn phase(&self) -> i32 {
        self.stats.phase
    }

    fn evaluate_piece(&self, pt: PieceType, row: usize, col: usize, color: Color) -> i32 {
        let mut val = material_value(pt) + pst_value_tapered(pt, row, col, color, self.phase());

        match pt {
            PieceType::Pawn => val += crate::board::evaluators::evaluate_pawns::evaluate_pawn(
                self.board, row, col, color, self.phase(), self.king_w, self.king_b, &self.att_w, &self.att_b, &self.pawn_counts
            ),
            PieceType::Knight => val += crate::board::evaluators::evaluate_knights::evaluate_knight(
                self.board, row, col, color, self.phase()
            ),
            PieceType::Bishop => val += crate::board::evaluators::evaluate_bishops::evaluate_bishop(
                row, col, color, self.phase()
            ),
            PieceType::Rook => val += crate::board::evaluators::evaluate_rooks::evaluate_rook(
                self.board, row, col, color, self.phase(), self.eg, self.stats.white_pawns, self.stats.black_pawns, &self.file_clearance
            ),
            PieceType::Queen => val += crate::board::evaluators::evaluate_queens::evaluate_queen(
                self.board, row, col, color, self.phase()
            ),
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
                        let pen = (base_pen * self.phase()) / 24;
                        score += apply_color_score(-pen, color);
                    }
                }
            }
        }
        score
    }

    fn evaluate_mobility(&self) -> i32 {
        // Calculate mobility from existing attack maps instead of recalculating
        let mut mob_w = 0;
        let mut mob_b = 0;

        // Count attacked squares, weighted by piece type
        for r in 0..8 {
            for c in 0..8 {
                if let Some(p) = self.board.get(r, c) {
                    let color = p.get_color();
                    let pt = p.get_type();

                    // Count pseudo-legal moves based on piece type
                    let mobility = match pt {
                        PieceType::Knight => count_knight_targets(self.board, r, c, color) as i32 * 2,
                        PieceType::Bishop => count_slider_targets(self.board, r, c, color, &[(1,1),(1,-1),(-1,1),(-1,-1)]) as i32 * 3,
                        PieceType::Rook => count_slider_targets(self.board, r, c, color, &[(1,0),(-1,0),(0,1),(0,-1)]) as i32 * 3,
                        PieceType::Queen => count_slider_targets(self.board, r, c, color, &[(1,1),(1,-1),(-1,1),(-1,-1),(1,0),(-1,0),(0,1),(0,-1)]) as i32,
                        _ => 0,
                    };

                    if matches!(color, Color::White) {
                        mob_w += mobility;
                    } else {
                        mob_b += mobility;
                    }
                }
            }
        }

        let mut score = (mob_w * self.phase()) / 24;
        score -= (mob_b * self.phase()) / 24;

        if self.phase() > 12 {
            let damp_w = (mob_w / 20) * (self.phase() - 12) / 12;
            let damp_b = (mob_b / 20) * (self.phase() - 12) / 12;
            score -= damp_w;
            score += damp_b;
        }
        score
    }

    fn evaluate_holes(&self) -> i32 {
        let mut score = 0;
        const HOLE_MG_PEN: i32 = 10;

        for r in 2..=5 {
            for c in 2..=5 {
                for &color in &[Color::White, Color::Black] {
                    if crate::board::evaluators::evaluate_pawns::is_hole_square_limited(
                        self.board, r, c, color, self.phase()
                    ) {
                        let (influenced, occupied_by_opp_minor) = match color {
                            Color::White => (
                                self.att_b[r][c],
                                matches!(get_piece_type(self.board, r, c), Some(PieceType::Knight | PieceType::Bishop))
                                    && is_color(self.board, r, c, Color::Black)
                            ),
                            Color::Black => (
                                self.att_w[r][c],
                                matches!(get_piece_type(self.board, r, c), Some(PieceType::Knight | PieceType::Bishop))
                                    && is_color(self.board, r, c, Color::White)
                            ),
                        };

                        if influenced || occupied_by_opp_minor {
                            let mut pen = HOLE_MG_PEN;
                            if occupied_by_opp_minor {
                                pen += 6;
                            }
                            score += apply_color_score(-(pen * self.phase()) / 24, color);
                        }
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
            if self.att_w[r][c] {
                score += (CENTER_CTRL_CP * self.phase()) / 24;
            }
            if self.att_b[r][c] {
                score -= (CENTER_CTRL_CP * self.phase()) / 24;
            }

            if let Some(p) = self.board.get(r, c)
                && matches!(p.get_type(), PieceType::Pawn | PieceType::Knight | PieceType::Bishop)
            {
                let bonus = (CENTER_OCC_EXTRA_CP * self.phase()) / 24;
                score += apply_color_score(bonus, p.get_color());
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
                    || crate::board::evaluators::evaluate_pawns::friendly_pawn_adjacent_behind_limited(
                        self.board, 4, c, Color::White, self.phase()
                    );
                if safe {
                    score += (SPACE_PAWN5_CP * self.phase()) / 24;
                }
            }

            // Black pawn on 5th rank (r==3)
            if is_piece(self.board, 3, c, Color::Black, PieceType::Pawn) {
                let safe = !square_attacked_by_enemy_pawn(self.board, 3, c, Color::White)
                    || crate::board::evaluators::evaluate_pawns::friendly_pawn_adjacent_behind_limited(
                        self.board, 3, c, Color::Black, self.phase()
                    );
                if safe {
                    score -= (SPACE_PAWN5_CP * self.phase()) / 24;
                }
            }
        }
        score
    }

    fn evaluate_global_features(&self) -> i32 {
        let mut score = 0;

        // Pawn islands penalty
        let w_islands = crate::board::evaluators::evaluate_pawns::evaluate_pawn_islands(
            &self.pawn_counts, Color::White, self.phase()
        );
        let b_islands = crate::board::evaluators::evaluate_pawns::evaluate_pawn_islands(
            &self.pawn_counts, Color::Black, self.phase()
        );
        score += w_islands - b_islands;

        // Pawn chains evaluation
        let w_chains = crate::board::evaluators::evaluate_pawns::evaluate_pawn_chains(
            self.board, Color::White, self.phase()
        );
        let b_chains = crate::board::evaluators::evaluate_pawns::evaluate_pawn_chains(
            self.board, Color::Black, self.phase()
        );
        score += w_chains - b_chains;

        // Pawn tension evaluation
        let w_tension = crate::board::evaluators::evaluate_pawns::evaluate_pawn_tension(
            self.board, Color::White, self.phase()
        );
        let b_tension = crate::board::evaluators::evaluate_pawns::evaluate_pawn_tension(
            self.board, Color::Black, self.phase()
        );
        score += w_tension - b_tension;

        // Pawn storm evaluation
        let w_storm = crate::board::evaluators::evaluate_pawns::evaluate_pawn_storm(
            self.board, Color::White, self.phase(), self.king_b, self.king_w
        );
        let b_storm = crate::board::evaluators::evaluate_pawns::evaluate_pawn_storm(
            self.board, Color::Black, self.phase(), self.king_w, self.king_b
        );
        score += w_storm - b_storm;

        // Bishop pair
        if self.stats.drawish_material.white_bishops >= 2 {
            score += self.taper(36, 24);
        }
        if self.stats.drawish_material.black_bishops >= 2 {
            score -= self.taper(36, 24);
        }

        // Rook/Queen activity - evaluate for both colors
        for &color in &[Color::White, Color::Black] {
            let rook_act = crate::board::evaluators::evaluate_rooks::rook_file_activity(
                self.board, color, &self.pawn_counts
            );
            let doubled_bonus = crate::board::evaluators::evaluate_rooks::doubled_rooks_bonus(
                self.board, color, &self.pawn_counts
            );
            let king_file_bonus = crate::board::evaluators::evaluate_rooks::rook_on_enemy_king_file_bonus(
                self.board, color
            );
            let queen_bonus = crate::board::evaluators::evaluate_queens::queen_on_semi_open_file_bonus(
                self.board, color, &self.pawn_counts
            );

            let rook_queen_score = (rook_act + doubled_bonus + king_file_bonus + queen_bonus) * self.phase() / 24;
            score += apply_color_score(rook_queen_score, color);
        }

        // King safety and activity
        for &color in &[Color::White, Color::Black] {
            let king_pos = match color {
                Color::White => self.king_w,
                Color::Black => self.king_b,
            };
            let safety = crate::board::evaluators::evaluate_king::king_safety(
                self.board, color, self.phase(), king_pos
            );
            let activity = crate::board::evaluators::evaluate_king::king_activity_endgame(king_pos);
            let shelter = crate::board::evaluators::evaluate_king::evaluate_king_shelter_patterns(
                self.board, color, self.phase(), king_pos
            );

            score += apply_color_score((safety * self.phase()) / 24, color);
            score += apply_color_score((activity * self.eg) / 24, color);
            score += apply_color_score(shelter, color);

            // Development penalty
            if self.phase() > 12 {
                let dev_pen = crate::board::evaluators::evaluate_king::development_penalty_on_backrank(
                    self.board, color, self.phase()
                );
                score += apply_color_score(dev_pen * (self.phase() - 12) / 12, color);
            }
        }

        // Early queen penalty
        for &color in &[Color::White, Color::Black] {
            let pen = crate::board::evaluators::evaluate_queens::early_queen_penalty(
                self.board, color, &self.pawn_counts
            );
            score += apply_color_score(-(pen * self.phase()) / 24, color);
        }

        score
    }

    /// Evaluate piece interactions: synergy, tropism, and batteries in a single pass.
    fn evaluate_piece_interactions(&self) -> i32 {
        let mut score = 0;

        // Collect piece positions in one pass
        let mut white_queens: Vec<(usize, usize)> = Vec::new();
        let mut white_rooks: Vec<(usize, usize)> = Vec::new();
        let mut white_bishops: Vec<(usize, usize)> = Vec::new();
        let mut black_queens: Vec<(usize, usize)> = Vec::new();
        let mut black_rooks: Vec<(usize, usize)> = Vec::new();
        let mut black_bishops: Vec<(usize, usize)> = Vec::new();

        for r in 0..8 {
            for c in 0..8 {
                if let Some(p) = self.board.get(r, c) {
                    let color = p.get_color();
                    let pt = p.get_type();

                    // Collect pieces for battery detection
                    match (color, pt) {
                        (Color::White, PieceType::Queen) => white_queens.push((r, c)),
                        (Color::White, PieceType::Rook) => white_rooks.push((r, c)),
                        (Color::White, PieceType::Bishop) => white_bishops.push((r, c)),
                        (Color::Black, PieceType::Queen) => black_queens.push((r, c)),
                        (Color::Black, PieceType::Rook) => black_rooks.push((r, c)),
                        (Color::Black, PieceType::Bishop) => black_bishops.push((r, c)),
                        _ => {}
                    }

                    // Skip pawns and kings for synergy and tropism
                    if matches!(pt, PieceType::Pawn | PieceType::King) {
                        continue;
                    }

                    // Synergy: bonus for defended pieces
                    let defended = match color {
                        Color::White => self.att_w[r][c],
                        Color::Black => self.att_b[r][c],
                    };

                    if defended {
                        let synergy_bonus = match pt {
                            PieceType::Knight | PieceType::Bishop => 4,
                            PieceType::Rook => 6,
                            PieceType::Queen => 8,
                            _ => 0,
                        };
                        score += apply_color_score((synergy_bonus * self.phase()) / 24, color);
                    }

                    // Tropism: bonus for proximity to enemy king
                    let enemy_king = match color {
                        Color::White => self.king_b,
                        Color::Black => self.king_w,
                    };

                    if let Some((ek_r, ek_c)) = enemy_king {
                        let dist = chebyshev_dist((r as i32, c as i32), (ek_r as i32, ek_c as i32));
                        let base_bonus = (7 - dist).max(0);

                        let tropism_bonus = match pt {
                            PieceType::Queen => base_bonus * 3,
                            PieceType::Rook => base_bonus * 2,
                            PieceType::Bishop => base_bonus * 2,
                            PieceType::Knight => base_bonus * 3,
                            _ => 0,
                        };

                        score += apply_color_score((tropism_bonus * self.phase()) / 24, color);
                    }
                }
            }
        }

        // Battery detection
        // R+Q batteries for White
        for &(qr, qc) in &white_queens {
            for &(rr, rc) in &white_rooks {
                if self.is_battery_on_line(qr, qc, rr, rc) {
                    score += (12 * self.phase()) / 24;
                }
            }
        }

        // B+Q batteries for White
        for &(qr, qc) in &white_queens {
            for &(br, bc) in &white_bishops {
                if self.is_battery_on_diagonal(qr, qc, br, bc) {
                    score += (10 * self.phase()) / 24;
                }
            }
        }

        // R+Q batteries for Black
        for &(qr, qc) in &black_queens {
            for &(rr, rc) in &black_rooks {
                if self.is_battery_on_line(qr, qc, rr, rc) {
                    score -= (12 * self.phase()) / 24;
                }
            }
        }

        // B+Q batteries for Black
        for &(qr, qc) in &black_queens {
            for &(br, bc) in &black_bishops {
                if self.is_battery_on_diagonal(qr, qc, br, bc) {
                    score -= (10 * self.phase()) / 24;
                }
            }
        }

        score
    }

    /// Check if two pieces form a battery on a file or rank (no pieces between them).
    #[inline]
    fn is_battery_on_line(&self, r1: usize, c1: usize, r2: usize, c2: usize) -> bool {
        if r1 == r2 {
            // Same rank - check if path is clear
            let (min_c, max_c) = if c1 < c2 { (c1, c2) } else { (c2, c1) };
            for c in (min_c + 1)..max_c {
                if self.board.get(r1, c).is_some() {
                    return false;
                }
            }
            true
        } else if c1 == c2 {
            // Same file - use cached clearance
            self.file_clearance.is_clear_between(r1, r2, c1)
        } else {
            false
        }
    }

    /// Check if two pieces form a battery on a diagonal (no pieces between them).
    #[inline]
    fn is_battery_on_diagonal(&self, r1: usize, c1: usize, r2: usize, c2: usize) -> bool {
        let dr = r2 as i32 - r1 as i32;
        let dc = c2 as i32 - c1 as i32;

        // Must be on same diagonal
        if dr.abs() != dc.abs() || dr == 0 {
            return false;
        }

        let step_r = if dr > 0 { 1 } else { -1 };
        let step_c = if dc > 0 { 1 } else { -1 };

        let mut r = r1 as i32 + step_r;
        let mut c = c1 as i32 + step_c;

        while r != r2 as i32 || c != c2 as i32 {
            if self.board.get(r as usize, c as usize).is_some() {
                return false;
            }
            r += step_r;
            c += step_c;
        }

        true
    }

    #[inline]
    fn taper(&self, mg: i32, eg: i32) -> i32 {
        taper_general(mg, eg, self.phase())
    }
}

// ============================================================
// MOBILITY HELPERS
// ============================================================

#[inline]
fn count_knight_targets(board: &Board, r: usize, c: usize, color: Color) -> usize {
    const K: [(i32,i32);8] = [(2,1),(1,2),(-1,2),(-2,1),(-2,-1),(-1,-2),(1,-2),(2,-1)];
    let mut n = 0usize;
    for (dr,dc) in K {
        let nr = r as i32 + dr;
        let nc = c as i32 + dc;
        if (0..8).contains(&nr) && (0..8).contains(&nc) {
            match board.get(nr as usize, nc as usize) {
                None => n += 1,
                Some(tp) if tp.get_color() != color => n += 1,
                _ => {}
            }
        }
    }
    n
}

#[inline]
fn count_slider_targets(board: &Board, r: usize, c: usize, color: Color, dirs: &[(i32,i32)]) -> usize {
    let mut n = 0usize;
    for (dr,dc) in dirs.iter() {
        let mut nr = r as i32 + dr;
        let mut nc = c as i32 + dc;
        while (0..8).contains(&nr) && (0..8).contains(&nc) {
            if let Some(tp) = board.get(nr as usize, nc as usize) {
                if tp.get_color() != color {
                    n += 1;
                }
                break;
            } else {
                n += 1;
            }
            nr += dr;
            nc += dc;
        }
    }
    n
}

// ============================================================
// DRAWISHNESS
// ============================================================

fn apply_drawish_tweaks(stats: &BoardStats, mut score: i32) -> i32 {
    // Insufficient material
    if stats.is_insufficient_material() {
        return 0;
    }

    // Opposite-colored bishops
    if stats.is_opposite_bishops_only() {
        let w_is_dark = stats.white_bishop_on_dark;
        let b_is_dark = stats.black_bishop_on_dark;
        if w_is_dark != b_is_dark {
            // Pull score toward zero (¾ factor)
            score = score - score / 4;
        }
    }

    score
}
