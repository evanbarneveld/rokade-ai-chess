//! Staged move picker for efficient move ordering.
//!
//! Instead of generating and sorting all moves upfront, this picks moves in stages:
//! 1. TT move (if available)
//! 2. Good captures sorted by SEE/MVV-LVA
//! 3. Killer moves
//! 4. Quiet moves sorted by history
//! 5. Bad captures
//!
//! This avoids the overhead of scoring/sorting moves that are never searched
//! due to early beta cutoffs.

use crate::board::Board;
use crate::piece::pieces::{Color, Piece, PieceType, piece_value_cp};
use crate::search::evaluation::heuristics::SearchHeuristics;
use crate::search::management::see::see_dest_estimate;
use crate::search::state::tt::decode_move;

/// Move with its ordering score
#[derive(Clone, Copy)]
struct ScoredMove {
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
    score: i32,
}

/// Stages of move picking
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    TtMove,
    GenerateCaptures,
    GoodCaptures,
    Killers,
    GenerateQuiets,
    Quiets,
    BadCaptures,
    Done,
}

/// Staged move picker that returns moves in order of likely quality
pub struct MovePicker {
    stage: Stage,
    tt_move: Option<((usize, usize), (usize, usize), Option<char>)>,
    good_captures: Vec<ScoredMove>,
    bad_captures: Vec<ScoredMove>,
    killers: [Option<((usize, usize), (usize, usize))>; 2],
    quiets: Vec<ScoredMove>,
    all_moves: Vec<((usize, usize), (usize, usize), Option<char>)>,
    capture_idx: usize,
    killer_idx: usize,
    quiet_idx: usize,
    bad_capture_idx: usize,
    // Context for scoring
    to_move: Color,
    half_move_clock: u32,
    prev_move: Option<((usize, usize), (usize, usize))>,
    // Track which moves we've already returned
    searched: Vec<((usize, usize), (usize, usize))>,
}

impl MovePicker {
    /// Create a new move picker with all generated moves
    pub fn new(
        moves: Vec<((usize, usize), (usize, usize), Option<char>)>,
        tt_move_hint: Option<(u8, u8)>,
        killers: [Option<((usize, usize), (usize, usize))>; 2],
        to_move: Color,
        half_move_clock: u32,
        prev_move: Option<((usize, usize), (usize, usize))>,
    ) -> Self {
        // Check if TT move is in the move list
        let tt_move = tt_move_hint.and_then(|(bf, bt)| {
            let bm = decode_move(bf, bt);
            moves.iter()
                .find(|(f, t, _)| (*f, *t) == bm)
                .copied()
        });

        Self {
            stage: Stage::TtMove,
            tt_move,
            good_captures: Vec::with_capacity(16),
            bad_captures: Vec::with_capacity(8),
            killers,
            quiets: Vec::with_capacity(32),
            all_moves: moves,
            capture_idx: 0,
            killer_idx: 0,
            quiet_idx: 0,
            bad_capture_idx: 0,
            to_move,
            half_move_clock,
            prev_move,
            searched: Vec::with_capacity(8),
        }
    }

    /// Get the next move to search
    pub fn next(
        &mut self,
        board: &Board,
        ep_target: Option<(usize, usize)>,
        heuristics: &SearchHeuristics,
    ) -> Option<((usize, usize), (usize, usize), Option<char>)> {
        loop {
            match self.stage {
                Stage::TtMove => {
                    self.stage = Stage::GenerateCaptures;
                    if let Some(mv) = self.tt_move {
                        self.searched.push((mv.0, mv.1));
                        return Some(mv);
                    }
                }

                Stage::GenerateCaptures => {
                    self.generate_captures(board, ep_target);
                    self.stage = Stage::GoodCaptures;
                }

                Stage::GoodCaptures => {
                    if self.capture_idx < self.good_captures.len() {
                        let mv = self.good_captures[self.capture_idx];
                        self.capture_idx += 1;
                        if !self.already_searched(mv.from, mv.to) {
                            self.searched.push((mv.from, mv.to));
                            return Some((mv.from, mv.to, mv.promo));
                        }
                    } else {
                        self.stage = Stage::Killers;
                    }
                }

                Stage::Killers => {
                    while self.killer_idx < 2 {
                        if let Some((kf, kt)) = self.killers[self.killer_idx] {
                            self.killer_idx += 1;
                            if !self.already_searched(kf, kt) {
                                // Verify killer is a legal move in current position
                                if let Some(mv) = self.all_moves.iter()
                                    .find(|(f, t, _)| (*f, *t) == (kf, kt))
                                    .copied()
                                {
                                    // Make sure it's a quiet move (not capture)
                                    if board.get(kt.0, kt.1).is_none() {
                                        self.searched.push((kf, kt));
                                        return Some(mv);
                                    }
                                }
                            }
                        } else {
                            self.killer_idx += 1;
                        }
                    }
                    self.stage = Stage::GenerateQuiets;
                }

                Stage::GenerateQuiets => {
                    self.generate_quiets(board, ep_target, heuristics);
                    self.stage = Stage::Quiets;
                }

                Stage::Quiets => {
                    if self.quiet_idx < self.quiets.len() {
                        let mv = self.quiets[self.quiet_idx];
                        self.quiet_idx += 1;
                        if !self.already_searched(mv.from, mv.to) {
                            self.searched.push((mv.from, mv.to));
                            return Some((mv.from, mv.to, mv.promo));
                        }
                    } else {
                        self.stage = Stage::BadCaptures;
                    }
                }

                Stage::BadCaptures => {
                    if self.bad_capture_idx < self.bad_captures.len() {
                        let mv = self.bad_captures[self.bad_capture_idx];
                        self.bad_capture_idx += 1;
                        if !self.already_searched(mv.from, mv.to) {
                            self.searched.push((mv.from, mv.to));
                            return Some((mv.from, mv.to, mv.promo));
                        }
                    } else {
                        self.stage = Stage::Done;
                    }
                }

                Stage::Done => {
                    return None;
                }
            }
        }
    }

    #[inline]
    fn already_searched(&self, from: (usize, usize), to: (usize, usize)) -> bool {
        self.searched.iter().any(|&(f, t)| f == from && t == to)
    }

    /// Generate and score captures, separating good from bad
    fn generate_captures(&mut self, board: &Board, ep_target: Option<(usize, usize)>) {
        for &(from, to, promo) in &self.all_moves {
            let is_ep = ep_target == Some(to)
                && board.get(from.0, from.1).map(|p| p.get_type()) == Some(PieceType::Pawn)
                && board.get(to.0, to.1).is_none();

            let is_capture = board.get(to.0, to.1).is_some() || is_ep;

            if is_capture || promo.is_some() {
                let score = self.score_capture(board, from, to, promo, is_ep);

                // For promotions, check if the move gives check.
                // Checking promotions should always be considered "good" because:
                // 1. The check forces a response
                // 2. SEE may incorrectly think the piece can be captured when it can't
                //    (e.g., king can't capture due to discovered check from other pieces)
                let is_checking_promo = if promo.is_some() {
                    self.promotion_gives_check(board, from, to, promo)
                } else {
                    false
                };

                let scored = ScoredMove { from, to, promo, score };

                #[cfg(feature = "debug-search")] {
                    if from == (6, 0) && to == (7, 0) && promo == Some('q') {
                        eprintln!("[MOVEPICKER] a8=Q score={} gives_check={} -> {}",
                                  score, is_checking_promo,
                                  if score >= 0 || is_checking_promo { "GOOD" } else { "BAD" });
                    }
                }

                // SEE >= 0 means good capture, < 0 means bad
                // Exception: checking promotions are always considered good
                if score >= 0 || is_checking_promo {
                    self.good_captures.push(scored);
                } else {
                    self.bad_captures.push(scored);
                }
            }
        }

        // Sort good captures by score (descending)
        self.good_captures.sort_by(|a, b| b.score.cmp(&a.score));
        // Bad captures sorted too (less negative first)
        self.bad_captures.sort_by(|a, b| b.score.cmp(&a.score));
    }

    /// Score a capture using MVV-LVA and SEE
    fn score_capture(
        &self,
        board: &Board,
        from: (usize, usize),
        to: (usize, usize),
        promo: Option<char>,
        is_ep: bool,
    ) -> i32 {
        let moved = board.get(from.0, from.1);
        let cap_sq = if is_ep { Some((from.0, to.1)) } else { None };
        let captured = if let Some(sq) = cap_sq {
            board.get(sq.0, sq.1)
        } else {
            board.get(to.0, to.1)
        };

        // MVV-LVA base score
        let cap_val = captured.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
        let attacker_val = moved.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
        let mvv_lva = cap_val * 10 - attacker_val;

        // Promotion bonus
        let promo_bonus = match promo {
            Some('q') | Some('Q') => 900,
            Some('r') | Some('R') => 500,
            Some('b') | Some('B') => 330,
            Some('n') | Some('N') => 320,
            _ => 0,
        };

        // SEE estimate
        let mut post = *board;
        post.set(from.0, from.1, None);
        if let Some(sq) = cap_sq {
            post.set(sq.0, sq.1, None);
        }

        let mut moved_piece = moved.unwrap_or(Piece::new(PieceType::Pawn, self.to_move));
        if let Some(pc) = promo {
            let pt = match pc {
                'q' | 'Q' => PieceType::Queen,
                'r' | 'R' => PieceType::Rook,
                'b' | 'B' => PieceType::Bishop,
                'n' | 'N' => PieceType::Knight,
                _ => moved_piece.get_type(),
            };
            moved_piece = Piece::new(pt, moved_piece.get_color());
        }
        post.set(to.0, to.1, Some(moved_piece));
        if moved_piece.get_type() == PieceType::King {
            post.set_king_location(moved_piece.get_color(), to);
        }

        let see = see_dest_estimate(&post, self.to_move, to, cap_val);

        // Combine: SEE dominates but add MVV-LVA for tie-breaking
        see * 100 + mvv_lva + promo_bonus
    }

    /// Generate and score quiet moves
    fn generate_quiets(
        &mut self,
        board: &Board,
        ep_target: Option<(usize, usize)>,
        heuristics: &SearchHeuristics,
    ) {
        for &(from, to, promo) in &self.all_moves {
            let is_ep = ep_target == Some(to)
                && board.get(from.0, from.1).map(|p| p.get_type()) == Some(PieceType::Pawn)
                && board.get(to.0, to.1).is_none();

            let is_capture = board.get(to.0, to.1).is_some() || is_ep;

            // Skip captures and promotions (already handled)
            if is_capture || promo.is_some() {
                continue;
            }

            let score = self.score_quiet(from, to, heuristics);
            self.quiets.push(ScoredMove { from, to, promo, score });
        }

        // Sort quiets by score (descending)
        self.quiets.sort_by(|a, b| b.score.cmp(&a.score));
    }

    /// Check if a promotion gives check
    fn promotion_gives_check(
        &self,
        board: &Board,
        from: (usize, usize),
        to: (usize, usize),
        promo: Option<char>,
    ) -> bool {
        let promo_char = match promo {
            Some(c) => c,
            None => return false,
        };

        // Build post-position
        let mut post = *board;
        post.set(from.0, from.1, None);

        let pt = match promo_char {
            'q' | 'Q' => PieceType::Queen,
            'r' | 'R' => PieceType::Rook,
            'b' | 'B' => PieceType::Bishop,
            'n' | 'N' => PieceType::Knight,
            _ => return false,
        };
        let promoted = Piece::new(pt, self.to_move);
        post.set(to.0, to.1, Some(promoted));

        // Check if opponent king is in check after promotion
        let opp = match self.to_move {
            Color::White => Color::Black,
            Color::Black => Color::White,
        };
        post.is_side_in_check(opp)
    }

    /// Score a quiet move using history and other heuristics
    fn score_quiet(
        &self,
        from: (usize, usize),
        to: (usize, usize),
        heuristics: &SearchHeuristics,
    ) -> i32 {
        let mut score = 0;

        // Counter move bonus
        if let Some((prev_from, prev_to)) = self.prev_move {
            if heuristics.is_counter_move(self.to_move, prev_from, prev_to, from, to) {
                score += 200_000;
            }
            let cont = heuristics.continuation_score(self.to_move, prev_to, from, to);
            score += (cont / 16).clamp(-150_000, 150_000);
        }

        // History score
        let hist = heuristics.history_score(self.to_move, from, to);
        score += (hist / 32).clamp(-200_000, 200_000);

        // Near 50-move rule bonus for pawn moves
        if self.half_move_clock >= 80 {
            score += 50_000;
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::game_state::GameState;
    use crate::search::core::advanced_search::find_all_valid_moves;

    #[test]
    fn test_move_picker_returns_all_moves() {
        let gs = GameState::default();
        let moves = find_all_valid_moves(&mut gs.clone());
        let heuristics = SearchHeuristics::new(128);

        let mut picker = MovePicker::new(
            moves.clone(),
            None,
            [None, None],
            Color::White,
            0,
            None,
        );

        let mut picked = Vec::new();
        while let Some(mv) = picker.next(gs.board(), gs.en_passant_target(), &heuristics) {
            picked.push(mv);
        }

        assert_eq!(picked.len(), moves.len(), "Should return all moves");
    }

    #[test]
    fn test_tt_move_first() {
        let gs = GameState::default();
        let moves = find_all_valid_moves(&mut gs.clone());
        let heuristics = SearchHeuristics::new(128);

        // Use e2e4 as TT move
        let tt_hint = Some((12, 28)); // e2=12, e4=28 in 0-63 encoding

        let mut picker = MovePicker::new(
            moves,
            tt_hint,
            [None, None],
            Color::White,
            0,
            None,
        );

        let first = picker.next(gs.board(), gs.en_passant_target(), &heuristics);
        assert!(first.is_some());
        let (from, to, _) = first.unwrap();
        // e2 = (1, 4), e4 = (3, 4)
        assert_eq!(from, (1, 4), "TT move should be first");
        assert_eq!(to, (3, 4), "TT move should be first");
    }
}
