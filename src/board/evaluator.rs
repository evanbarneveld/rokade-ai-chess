use crate::board::attack_maps::build_attack_maps;
use crate::board::evaluation_helpers::{
    apply_color_score, chebyshev_dist, find_king,
    get_piece_type, is_color, is_piece, material_value, square_attacked_by_enemy_pawn,
};
use crate::board::Board;
use crate::board::pst::*;
use crate::piece::pieces::{piece_value_cp, Color, PieceType};

pub use crate::board::evaluation_helpers::{FileClearance, PawnFileCounts};
pub(crate) use crate::board::evaluation_helpers::taper_general;

pub const MIN_EVAL_VALUE: i32 = i32::MIN + 100_000;
pub const MAX_EVAL_VALUE: i32 = i32::MAX - 100_000;

pub const MATE_VALUE: i32 = 30_000;

const THREAT_MIN_GAIN: i32 = 150;
const THREAT_BASE_BONUS: i32 = 6;
const THREAT_VALUE_DIV: i32 = 20;
const THREAT_MAX_BONUS: i32 = 40;

// Phase weights for game phase calculation (total = 24 at start)
const PHASE_KNIGHT: i32 = 1;
const PHASE_BISHOP: i32 = 1;
const PHASE_ROOK: i32 = 2;
const PHASE_QUEEN: i32 = 4;

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
    score += ctx.evaluate_threats();
    score += ctx.evaluate_minor_piece_imbalance();

    apply_drawish_tweaks(&ctx.stats, score)
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
    blocked_pawns: i32,      // Pawns blocked by enemy pawn directly ahead
    white_knights: i32,
    black_knights: i32,
    white_bishops: i32,
    black_bishops: i32,
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
        let mut blocked_pawns = 0;
        let mut white_knights = 0;
        let mut black_knights = 0;
        let mut white_bishops = 0;
        let mut black_bishops = 0;

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
                                // Check if blocked by enemy pawn
                                if r < 7 {
                                    if let Some(blocker) = board.get(r + 1, c) {
                                        if blocker.get_type() == PieceType::Pawn && blocker.get_color() == Color::Black {
                                            blocked_pawns += 1;
                                        }
                                    }
                                }
                            } else {
                                black_pawns += 1;
                                // Check if blocked by enemy pawn
                                if r > 0 {
                                    if let Some(blocker) = board.get(r - 1, c) {
                                        if blocker.get_type() == PieceType::Pawn && blocker.get_color() == Color::White {
                                            blocked_pawns += 1;
                                        }
                                    }
                                }
                            }
                            drawish.pawns += 1;
                        }
                        PieceType::Bishop => {
                            if color == Color::White {
                                drawish.white_bishops += 1;
                                white_bishops += 1;
                                if (r + c) % 2 == 1 {
                                    white_bishop_on_dark = true;
                                }
                            } else {
                                drawish.black_bishops += 1;
                                black_bishops += 1;
                                if (r + c) % 2 == 1 {
                                    black_bishop_on_dark = true;
                                }
                            }
                            drawish.minors += 1;
                        }
                        PieceType::Knight => {
                            if color == Color::White {
                                white_knights += 1;
                            } else {
                                black_knights += 1;
                            }
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
            blocked_pawns,
            white_knights,
            black_knights,
            white_bishops,
            black_bishops,
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

    /// Calculate position openness: 0 = very closed, 100 = very open.
    /// Based on total pawns and blocked pawns.
    fn openness(&self) -> i32 {
        let total_pawns = self.stats.white_pawns + self.stats.black_pawns;
        let blocked = self.stats.blocked_pawns;

        // Fewer pawns = more open
        // More blocked pawns = more closed
        // Start with 100, subtract for pawns and blocked pawns
        let pawn_factor = (16 - total_pawns) * 4;  // 0-64 range
        let blocked_factor = -blocked * 8;          // Penalty for blocked pawns

        (50 + pawn_factor + blocked_factor).clamp(0, 100)
    }

    /// Evaluate knight vs bishop imbalance based on position openness.
    /// Returns score adjustment in White's perspective.
    fn evaluate_minor_piece_imbalance(&self) -> i32 {
        let openness = self.openness();
        let phase = self.phase();

        // Adjustment per piece: positive = good in this position type
        // Open positions (openness > 60): bishops +, knights -
        // Closed positions (openness < 40): knights +, bishops -
        // Neutral (40-60): no adjustment

        let adjustment_per_piece = if openness > 60 {
            // Open position: bishops better
            (openness - 60) / 4  // Max +10 per piece
        } else if openness < 40 {
            // Closed position: knights better (we'll apply opposite sign)
            (40 - openness) / 4  // Max +10 per piece
        } else {
            0
        };

        if adjustment_per_piece == 0 {
            return 0;
        }

        let mut score = 0;

        if openness > 60 {
            // Bishops get bonus, knights get penalty
            score += adjustment_per_piece * self.stats.white_bishops;
            score -= adjustment_per_piece * self.stats.white_knights;
            score -= adjustment_per_piece * self.stats.black_bishops;
            score += adjustment_per_piece * self.stats.black_knights;
        } else {
            // Knights get bonus, bishops get penalty
            score += adjustment_per_piece * self.stats.white_knights;
            score -= adjustment_per_piece * self.stats.white_bishops;
            score -= adjustment_per_piece * self.stats.black_knights;
            score += adjustment_per_piece * self.stats.black_bishops;
        }

        // Scale by phase (more important in middlegame)
        (score * phase) / 24
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
                self.board, row, col, color, self.phase()
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
        // Mobility is now handled by individual piece evaluators (evaluate_knights, etc.)
        // which compute safe mobility (squares not attacked by enemy pawns).
        // This avoids duplicate computation and provides more accurate evaluation.
        0
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

        // Pawn majority bonus
        let w_majority = crate::board::evaluators::evaluate_pawns::pawn_majority_bonus(
            &self.pawn_counts, Color::White, self.phase()
        );
        let b_majority = crate::board::evaluators::evaluate_pawns::pawn_majority_bonus(
            &self.pawn_counts, Color::Black, self.phase()
        );
        score += w_majority - b_majority;

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
            let alignment_bonus = crate::board::evaluators::evaluate_rooks::rook_queen_alignment_bonus(
                self.board, color, &self.pawn_counts
            );
            let queen_bonus = crate::board::evaluators::evaluate_queens::queen_on_semi_open_file_bonus(
                self.board, color, &self.pawn_counts
            );

            let rook_queen_score = (rook_act + doubled_bonus + king_file_bonus + alignment_bonus + queen_bonus)
                * self.phase() / 24;
            score += apply_color_score(rook_queen_score, color);
        }

        // King safety and activity
        for &color in &[Color::White, Color::Black] {
            let king_pos = match color {
                Color::White => self.king_w,
                Color::Black => self.king_b,
            };
            let safety = crate::board::evaluators::evaluate_king::king_safety(
                self.board, color, self.phase(), king_pos, &self.pawn_counts
            );
            let ring_pressure = crate::board::evaluators::evaluate_king::king_ring_pressure(
                self.board, color, self.phase(), king_pos, &self.att_w, &self.att_b
            );
            let activity = crate::board::evaluators::evaluate_king::king_activity_endgame(king_pos);
            let shelter = crate::board::evaluators::evaluate_king::evaluate_king_shelter_patterns(
                self.board, color, self.phase(), king_pos
            );

            score += apply_color_score((safety * self.phase()) / 24, color);
            score += apply_color_score(ring_pressure, color);
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

    fn evaluate_threats(&self) -> i32 {
        let mut score = 0;
        for r in 0..8 {
            for c in 0..8 {
                if let Some(p) = self.board.get(r, c) {
                    let bonus = self.threat_bonus_for_piece(r, c, p.get_color(), p.get_type());
                    score += apply_color_score(bonus, p.get_color());
                }
            }
        }
        score
    }

    fn threat_bonus_for_piece(&self, r: usize, c: usize, color: Color, pt: PieceType) -> i32 {
        if pt == PieceType::King {
            return 0;
        }
        if !self.is_attacker_safe(r, c, color) {
            return 0;
        }

        let attacker_value = piece_value_cp(pt);
        let mut best = 0i32;

        match pt {
            PieceType::Pawn => {
                let dir = if color == Color::White { 1 } else { -1 };
                for dc in [-1i32, 1] {
                    let tr = r as i32 + dir;
                    let tc = c as i32 + dc;
                    if let Some(bonus) = self.threat_bonus_from_target(color, attacker_value, tr, tc) {
                        best = best.max(bonus);
                    }
                }
            }
            PieceType::Knight => {
                for (dr, dc) in [(-2, -1), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)] {
                    let tr = r as i32 + dr;
                    let tc = c as i32 + dc;
                    if let Some(bonus) = self.threat_bonus_from_target(color, attacker_value, tr, tc) {
                        best = best.max(bonus);
                    }
                }
            }
            PieceType::Bishop => {
                best = best.max(self.threat_bonus_from_slider(color, attacker_value, r, c, &[(1, 1), (1, -1), (-1, 1), (-1, -1)]));
            }
            PieceType::Rook => {
                best = best.max(self.threat_bonus_from_slider(color, attacker_value, r, c, &[(1, 0), (-1, 0), (0, 1), (0, -1)]));
            }
            PieceType::Queen => {
                best = best.max(self.threat_bonus_from_slider(color, attacker_value, r, c, &[
                    (1, 1), (1, -1), (-1, 1), (-1, -1),
                    (1, 0), (-1, 0), (0, 1), (0, -1),
                ]));
            }
            PieceType::King => {}
        }

        (best * self.phase()) / 24
    }

    fn threat_bonus_from_slider(
        &self,
        color: Color,
        attacker_value: i32,
        r: usize,
        c: usize,
        dirs: &[(i32, i32)],
    ) -> i32 {
        let mut best = 0i32;
        for (dr, dc) in dirs {
            let mut tr = r as i32 + dr;
            let mut tc = c as i32 + dc;
            while (0..8).contains(&tr) && (0..8).contains(&tc) {
                if let Some(bonus) = self.threat_bonus_from_target(color, attacker_value, tr, tc) {
                    best = best.max(bonus);
                }
                if self.board.get(tr as usize, tc as usize).is_some() {
                    break;
                }
                tr += dr;
                tc += dc;
            }
        }
        best
    }

    fn threat_bonus_from_target(
        &self,
        color: Color,
        attacker_value: i32,
        tr: i32,
        tc: i32,
    ) -> Option<i32> {
        if !(0..8).contains(&tr) || !(0..8).contains(&tc) {
            return None;
        }
        let tr = tr as usize;
        let tc = tc as usize;
        let target = self.board.get(tr, tc)?;
        if target.get_color() == color {
            return None;
        }
        if target.get_type() == PieceType::King {
            return None;
        }

        let defended_by_enemy = match color {
            Color::White => self.att_b[tr][tc],
            Color::Black => self.att_w[tr][tc],
        };
        if !defended_by_enemy {
            return None;
        }

        let target_value = piece_value_cp(target.get_type());
        let gain = target_value - attacker_value;
        if gain < THREAT_MIN_GAIN {
            return None;
        }

        let bonus = (THREAT_BASE_BONUS + gain / THREAT_VALUE_DIV).min(THREAT_MAX_BONUS);
        Some(bonus)
    }

    fn is_attacker_safe(&self, r: usize, c: usize, color: Color) -> bool {
        let (attacked_by_opp, defended_by_own) = match color {
            Color::White => (self.att_b[r][c], self.att_w[r][c]),
            Color::Black => (self.att_w[r][c], self.att_b[r][c]),
        };
        defended_by_own || !attacked_by_opp
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
