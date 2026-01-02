use rand::{rng, Rng};
use crate::board::Board;
use crate::board::evaluator::evaluate_position;
use crate::history::history::History;
use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{capture_value_cp, opposite_color, piece_value_cp, Color, Piece, PieceType};
use crate::search::alphabeta::alphabeta;
use crate::search::playing_strength::strength_noise_sigma;
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
    // Previously applied piece-specific root filtering (queen/minor handling) has been removed.
    // With a stronger evaluator, we keep all legal root moves and rely on Search/eval to decide.
    filtered.extend(v.iter().copied());
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
    strength_ps: i32,
) -> i32 {
    // Base root bonus
    let mut adjusted = score_raw + root_move_bonus(base_board, from, to, side);

    // SEE-based root penalty (uniform across pieces)
    {
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

    // Playing-strength noise
    let sigma = strength_noise_sigma(strength_ps as usize);
    if sigma > 0 {
        let n: i32 = rng().random_range(-sigma..=sigma);
        adjusted += n;
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
        alphabeta(
            &mut tmp,
            opposite_color(side),
            depth_now - 1,
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



