use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::history::history::History;
use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{capture_value_cp, opposite_color, piece_value_cp, Color, Piece, PieceType};
use crate::search::alphabeta::alphabeta;
use crate::search::advanced_search::find_all_valid_moves;
use crate::search::see::{see_dest_estimate, SEE_PENALTY_MAX_CP, SEE_PENALTY_MIN_CP};
use crate::search::tt::{decode_move, TranspositionTable};
use crate::search::zobrist::compute_zobrist;
use crate::state::fen::writer::game_state_to_fen_string;
use crate::state::game_state::GameState;


const ROOT_CAPTURE_BONUS_DIV: i32 = 10; // add captured piece value / this divisor

const ENDGAME_SIDEADV_THRESHOLD_CP: i32 = 150; // only apply adjustments if side advantage above this
const ENDGAME_HMC_THRESHOLD: u32 = 80; // start scaling after this half-move clock
const ENDGAME_SCALE_MAX: i32 = 21; // max scaling steps used in formula below
const ENDGAME_CAPTURE_SCALE_BONUS_CP: i32 = 15; // per-scale bonus if capture or pawn move
const ENDGAME_NONCAP_SCALE_PENALTY_CP: i32 = 8; // per-scale penalty if quiet move


const REP_STACK_CAPACITY: usize = 128; // repetition detection stack capacity hint

// / build_pv_for_root constructs the principal variation (PV) line starting from a given root move
/// by following best moves stored in the transposition table (TT), alternating sides, and validating
/// every step’s legality. It returns a list of move pairs (from, to) that represents the best‑known line
/// from the root according to the TT.
#[inline]
pub fn build_pv_for_root(
    board: &Board,
    root_side: Color,
    from: (usize, usize),
    to: (usize, usize),
    tt: &TranspositionTable,
    max_len: usize,
) -> Vec<((usize, usize), (usize, usize))> {
    let mut pv: Vec<((usize, usize), (usize, usize))> = Vec::with_capacity(max_len.max(1));
    pv.push((from, to));

    // Work on a temporary board following the PV using TT best moves
    let mut tmp = board.clone();
    let _undo = tmp.make_move_simple(from, to);
    let mut side = opposite_color(root_side);

    for _ in 1..max_len {
        let key = compute_zobrist(&tmp, side);
        let Some(entry) = tt.probe(key) else {
            break;
        };
        let (bf, bt) = (entry.best_from, entry.best_to);
        let ((nfr, nfc), (ntr, ntc)) = decode_move(bf, bt);
        let next = ((nfr, nfc), (ntr, ntc));
        // Validate legality in current position to avoid garbage PV
        let gs = GameState::from_board_and_side(tmp.clone(), side);
        let legals_pairs: Vec<((usize, usize), (usize, usize))> = find_all_valid_moves(&gs)
            .iter()
            .map(|(f, t, _)| (*f, *t))
            .collect();
        if !legals_pairs.contains(&next) {
            break;
        }
        pv.push(next);
        let _u = tmp.make_move_simple((nfr, nfc), (ntr, ntc));
        side = opposite_color(side);
    }
    pv
}

pub fn hard_root_filter(_board: &Board, _active_color: Color, v: &mut Vec<((usize, usize), (usize, usize))>, filtered: &mut Vec<((usize, usize), (usize, usize))>) {
    // Conservative hard filter at root: discard any quiet move that immediately hangs
    // on the destination square according to a simple SEE probe. This avoids blatant
    // one-ply blunders.
    // Additionally, discard clearly losing captures at root (destination SEE strongly negative).
    // IMPORTANT: Never discard moves that give check — tactical checks/sacrifices must be allowed.
    for &(from, to) in v.iter() {
        let mut post = _board.clone();
        let moved = match _board.get(from.0, from.1) { Some(p) => p, None => { filtered.push((from, to)); continue; } };
        let is_capture = _board.get(to.0, to.1).is_some();

        // Simulate the move once
        post.set(from.0, from.1, None);
        post.set(to.0, to.1, Some(moved));

        // If the move gives check, keep it regardless of SEE — don't filter out checking sacs.
        let opponent = opposite_color(_active_color);
        if post.is_side_in_check(opponent) {
            filtered.push((from, to));
            continue;
        }

        // Quiet move self-hang filter
        if !is_capture {
            let see = see_dest_estimate(&post, _active_color, to, 0);
            if see < 0 { continue; }
        } else {
            // Losing capture gate: estimate with captured value context
            let cap_val = _board
                .get(to.0, to.1)
                .map(|p| piece_value_cp(p.get_type()))
                .unwrap_or(0);
            let see = see_dest_estimate(&post, _active_color, to, cap_val);
            // If SEE is strongly negative (worse than the standard minimum), drop it
            if see < -SEE_PENALTY_MIN_CP { continue; }
        }
        filtered.push((from, to));
    }
}

pub fn get_root_moves(game_state: &GameState, history: &History, board: &Board, active_color: Color, moves: &Vec<((usize, usize), (usize, usize))>, v: &mut Vec<((usize, usize), (usize, usize))>) {
    for &(from, to) in moves {
        let is_capture = board.get(to.0, to.1).is_some();
        let mut gs = *game_state; // GameState is Copy
        let mut promote: Option<Piece> = None;
        if let Some(p) = gs.board().get(from.0, from.1) {
            if p.get_type() == PieceType::Pawn {
                if (active_color == Color::White && to.0 == 7)
                    || (active_color == Color::Black && to.0 == 0)
                {
                    promote = Some(Piece::new(PieceType::Queen, active_color));
                }
            }
        }
        let makes_threefold = if PieceMover::move_piece(&mut gs, from, to, is_capture, promote)
        {
            gs.switch_player_turn();
            let fen = game_state_to_fen_string(gs);
            let truncated = fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ");
            history.fen_repetition_count(&truncated) >= 2
        } else {
            false
        };
        if !makes_threefold {
            v.push((from, to));
        }
    }
}

// Root-level move bonus strictly limited to tiny tie-breakers that are not covered by the static evaluator.
// Positive favors White; negative favors Black (we add for side to move).
// Single source of truth policy: avoid duplicating evaluator logic (pawn-file preferences etc.).
pub fn root_move_bonus(board: &Board, from: (usize, usize), to: (usize, usize), side: Color) -> i32 {
    let mut bonus: i32 = 0;

    // Identify piece and basic metadata
    let piece = match board.get(from.0, from.1) {
        Some(p) => p,
        None => return 0,
    };
    let pt = piece.get_type();

    let (_fr, _fc) = from;
    let (tr, tc) = to;

    // Keep only very small development/centralization nudges not modeled explicitly by eval:
    // Knights to c3/f3 (White) or c6/f6 (Black)
    if pt == PieceType::Knight {
        match side {
            Color::White => {
                if (tr, tc) == (2, 2) || (tr, tc) == (2, 5) {
                    bonus += 5;
                }
            }
            Color::Black => {
                if (tr, tc) == (5, 2) || (tr, tc) == (5, 5) {
                    bonus += 5;
                }
            }
        }
    }

    // Bishops to c4/f4 for White; c5/f5 for Black (tiny nudge)
    if pt == PieceType::Bishop {
        match side {
            Color::White => {
                if (tr, tc) == (3, 2) || (tr, tc) == (3, 5) {
                    bonus += 3;
                }
            }
            Color::Black => {
                if (tr, tc) == (4, 2) || (tr, tc) == (4, 5) {
                    bonus += 3;
                }
            }
        }
    }

    // Apply sign for side to move (we always add for the maximizing side at root)
    match side {
        Color::White => bonus,
        Color::Black => -bonus,
    }
}
#[inline]
pub fn adjust_root_score(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    base_hmc: u32,
    is_capture: bool,
    moved_is_pawn: bool,
    score_raw: i32,
    _strength_ps: i32,
) -> i32 {
    // Base root bonus
    let mut adjusted = score_raw + root_move_bonus(base_board, from, to, side);

    // Detect if the move gives check to the opponent; checking sacs should not be punished by SEE.
    let mut post_probe = base_board.clone();
    let moved_probe = base_board.get(from.0, from.1);
    if let Some(mp) = moved_probe { post_probe.set(to.0, to.1, Some(mp)); }
    post_probe.set(from.0, from.1, None);
    let opp = opposite_color(side);
    let gives_check = post_probe.is_side_in_check(opp);

    // SEE-based root penalty (uniform across pieces) — skip for checking moves to allow tactical sacs
    if !gives_check {
        let mut post = base_board.clone();
        let moved_piece = base_board.get(from.0, from.1);
        let captured = base_board.get(to.0, to.1);
        if let Some(mp) = moved_piece {
            post.set(from.0, from.1, None);
            post.set(to.0, to.1, Some(mp));
            let cap_val = captured.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
            let see = see_dest_estimate(&post, side, to, cap_val);
            if see < 0 {
                let pen = (-see).clamp(SEE_PENALTY_MIN_CP, SEE_PENALTY_MAX_CP);
                adjusted -= pen;
                // Extra discouragement for quiet pawn pushes that hang the pawn immediately
                if !is_capture && moved_is_pawn {
                    adjusted -= SEE_PENALTY_MIN_CP; // add minimum SEE penalty again
                }
            }
        }
    }

    // Small capture bonus at root
    if let Some(captured) = base_board.get(to.0, to.1) {
        let cap_val = capture_value_cp(captured.get_type());
        adjusted += cap_val / ROOT_CAPTURE_BONUS_DIV;
    }

    // Endgame/50-move rule pressure scaling
    let side_adv = if side == Color::White { score_raw } else { -score_raw };
    if side_adv > ENDGAME_SIDEADV_THRESHOLD_CP && base_hmc >= ENDGAME_HMC_THRESHOLD {
        let scale = (base_hmc as i32 - (ENDGAME_HMC_THRESHOLD as i32 - 1)).min(ENDGAME_SCALE_MAX);
        if is_capture || moved_is_pawn {
            adjusted += ENDGAME_CAPTURE_SCALE_BONUS_CP * scale;
        } else {
            adjusted -= ENDGAME_NONCAP_SCALE_PENALTY_CP * scale;
        }
    }

    /*
    // Playing-strength noise (suppressed in deterministic mode)
    let sigma = if crate::search::is_deterministic() { 0 } else { strength_noise_sigma(strength_ps as usize) };
    if sigma > 0 {
        let n: i32 = rng().random_range(-sigma..=sigma);
        adjusted += n;
    }
    */

    // Extra root-level king safety: strongly discourage moving the king into attacked squares
    let moved_is_king = base_board
        .get(from.0, from.1)
        .map(|p| p.get_type() == PieceType::King)
        .unwrap_or(false);
    if moved_is_king {
        let mut postk = base_board.clone();
        let mvp = base_board.get(from.0, from.1);
        postk.set(from.0, from.1, None);
        if let Some(mp) = mvp { postk.set(to.0, to.1, Some(mp)); }
        // If destination is attacked by opponent, apply a hefty penalty (walking into fire)
        if is_square_attacked_by_opponent(&mut postk, to, side) {
            adjusted -= SEE_PENALTY_MAX_CP.max(300);
        }
        // Discourage king captures at root; typically unsafe in middlegame
        if is_capture {
            adjusted -= 600;
        }
    }

    // Root-level self-hanging penalty: if this move leaves any of our pieces
    // immediately losable (opponent can capture with negative SEE for us),
    // apply a bounded penalty. This complements the destination SEE check.
    if !gives_check {
        let mut post = base_board.clone();
        // simulate the move
        let moved_piece = base_board.get(from.0, from.1);
        post.set(from.0, from.1, None);
        if let Some(mp) = moved_piece { post.set(to.0, to.1, Some(mp)); }

        // Scan our pieces; if any square is attacked and SEE < 0 for us, penalize
        let mut total_penalty: i32 = 0;
        for r in 0..8 {
            for c in 0..8 {
                if let Some(p) = post.get(r, c) {
                    if p.get_color() != side { continue; }
                    // Quick filter: only consider squares attacked by opponent
                    if !is_square_attacked_by_opponent(&mut post,(r, c), side) { continue; }
                    // No prior capture gain on these squares in a generic self-hang scan
                    let see = see_dest_estimate(&post, side, (r, c), 0);
                    if see < 0 {
                        // Conservative: half the SEE loss, clamped to familiar bounds
                        let pen = (-see).clamp(SEE_PENALTY_MIN_CP, SEE_PENALTY_MAX_CP) / 2;
                        total_penalty += pen;
                    }
                }
            }
        }
        // Cap aggregate penalty so a cluster of minor hangs doesn't explode the score
        let agg_cap: i32 = SEE_PENALTY_MAX_CP * 2; // up to 2x max per move
        if total_penalty > 0 { adjusted -= total_penalty.min(agg_cap); }
    } else {
        // Stronger tie-break for checking moves at root to favor forcing tactics
        adjusted += 60;

        // Opponent mobility after a checking move: fewer replies -> larger bonus.
        // Compute opponent legal moves in the checked position and scale bonus inversely.
        let opp_state = crate::state::game_state::GameState::from_board_and_side(post_probe.clone(), opp);
        let legals = crate::search::advanced_search::find_all_valid_moves(&opp_state);
        let replies = legals.len() as i32;
        // Scale: up to +120 when replies are 0-2, diminishing thereafter.
        let mobility_bonus = match replies {
            0 => 120, // mate next: huge bonus
            1..=2 => 100,
            3..=5 => 60,
            6..=8 => 30,
            9..=12 => 10,
            _ => 0,
        };
        adjusted += mobility_bonus;
    }

    adjusted
}

#[inline]
pub fn evaluate_after_root_move(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    depth_now: usize,
    a: i32,
    b: i32,
    tt: &mut TranspositionTable,
    base_hmc: u32,
) -> (i32, bool, bool) {
    let mut tmp = base_board.clone();
    let u = tmp.make_move_simple(from, to);
    let moved_is_pawn = base_board
        .get(from.0, from.1)
        .map(|p| p.get_type() == PieceType::Pawn)
        .unwrap_or(false);
    let is_capture = base_board.get(to.0, to.1).is_some();
    let child_hmc: u32 = if is_capture || moved_is_pawn { 0 } else { base_hmc.saturating_add(1) };
    // Root selective extension: extend by 1 ply for checking moves (helps find forcing lines like mates)
    let gives_check = tmp.is_side_in_check(opposite_color(side));
    let score_raw = if depth_now <= 1 {
        // At shallow depth, avoid plain static eval to prevent horizon blunders.
        // Use quiescence search to account for immediate captures/tactics.
        crate::search::qsearch::qsearch(
            &mut tmp,
            opposite_color(side),
            crate::search::advanced_search::MIN_EVAL_VALUE + 1,
            crate::search::advanced_search::MAX_EVAL_VALUE - 1,
            child_hmc,
            &mut Vec::new(),
        )
    } else {
        let mut rep_stack: Vec<u64> = Vec::with_capacity(REP_STACK_CAPACITY);
        let ext = if gives_check { 2 } else { 0 };
        alphabeta(
            &mut tmp,
            opposite_color(side),
            depth_now - 1 + ext,
            a,
            b,
            1,
            tt,
            child_hmc,
            &mut rep_stack,
        )
    };
    tmp.unmake_move_simple(u);
    (score_raw, is_capture, moved_is_pawn)
}

#[inline]
pub fn adjusted_root_eval_for_move(
    base_board: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize, usize),
    base_hmc: u32,
    score_raw: i32,
    is_capture: bool,
    moved_is_pawn: bool,
    ps: i32,
) -> i32 {
    adjust_root_score(
        base_board, side, from, to, base_hmc, is_capture, moved_is_pawn, score_raw, ps,
    )
}



