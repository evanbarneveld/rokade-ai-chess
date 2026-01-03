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

        // If the move gives check, generally keep it — but explicitly drop suicidal queen checks
        // where the destination SEE is clearly losing for the mover (e.g., hanging the queen).
        let opponent = opposite_color(_active_color);
        if post.is_side_in_check(opponent) {
            let moved_is_queen = matches!(moved.get_type(), PieceType::Queen);
            if moved_is_queen {
                // For a checking queen move, still gate with SEE to avoid blatant queen drops.
                // Use captured value context (0 for quiet) because the loss happens after the check.
                let see_q = see_dest_estimate(&post, _active_color, to, 0);
                if see_q < -SEE_PENALTY_MIN_CP { continue; }
            }
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
    let moved_is_queen = moved_probe
        .map(|p| p.get_type() == PieceType::Queen)
        .unwrap_or(false);

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
    } else {
        // Special-case: even for checking moves, avoid suicidal queen checks that drop the queen by force.
        // Apply a bounded SEE penalty to queen moves if destination SEE is negative.
        if moved_is_queen {
            let mut post = base_board.clone();
            if let Some(mp) = base_board.get(from.0, from.1) {
                post.set(from.0, from.1, None);
                post.set(to.0, to.1, Some(mp));
                let cap_val = base_board
                    .get(to.0, to.1)
                    .map(|p| piece_value_cp(p.get_type()))
                    .unwrap_or(0);
                let see = see_dest_estimate(&post, side, to, cap_val);
                if see < 0 {
                    // For queens, a losing checking move is usually catastrophic; scale penalty strongly.
                    // Scale with SEE magnitude and clamp to a large bound so it can outweigh check bonuses.
                    let q_pen = ((-see) * 6).clamp(600, 6000);
                    adjusted -= q_pen;
                }
            }
        }
    }

    // Threat-resolution: prefer evacuating attacked minors (esp. if attacked by a pawn),
    // and penalize ignoring a clear evacuation.
    if !gives_check {
        // Collect attacked own pieces in the base position
        let mut base_clone = base_board.clone();
        let mut threatened: Vec<(usize, usize, PieceType, bool)> = Vec::new();
        for r in 0..8 { for c in 0..8 {
            if let Some(p) = base_board.get(r,c) {
                if p.get_color() != side { continue; }
                if !is_square_attacked_by_opponent(&mut base_clone, (r,c), side) { continue; }
                // mark if attacked by pawn specifically
                let opp = opposite_color(side);
                let pawn_attacks = if opp == Color::White {
                    (r>0 && c>0 && matches!(base_board.get(r-1,c-1), Some(px) if px.get_color()==opp && px.get_type()==PieceType::Pawn)) ||
                    (r>0 && c+1<8 && matches!(base_board.get(r-1,c+1), Some(px) if px.get_color()==opp && px.get_type()==PieceType::Pawn))
                } else {
                    (r+1<8 && c>0 && matches!(base_board.get(r+1,c-1), Some(px) if px.get_color()==opp && px.get_type()==PieceType::Pawn)) ||
                    (r+1<8 && c+1<8 && matches!(base_board.get(r+1,c+1), Some(px) if px.get_color()==opp && px.get_type()==PieceType::Pawn))
                };
                threatened.push((r,c,p.get_type(), pawn_attacks));
            }
        }}

        if !threatened.is_empty() {
            // Post position after candidate move
            let mut post = base_board.clone();
            if let Some(mp) = base_board.get(from.0, from.1) {
                post.set(from.0, from.1, None);
                post.set(to.0, to.1, Some(mp));
            }
            // side multiplier: positive for White, negative for Black, so that
            // bonuses improve the score for the maximizer and penalties worsen it
            let side_mul: i32 = if side == Color::White { 1 } else { -1 };

            for (tr,tc,pt,by_pawn) in threatened {
                // Knight-specific: precompute safe escape squares in base
                let mut knight_safe: Vec<(usize,usize)> = Vec::new();
                if pt == PieceType::Knight && by_pawn {
                    let deltas: [(isize,isize);8] = [(2,1),(2,-1),(-2,1),(-2,-1),(1,2),(1,-2),(-1,2),(-1,-2)];
                    for (dr,dc) in deltas {
                        let nr = tr as isize + dr; let nc = tc as isize + dc;
                        if nr<0 || nr>=8 || nc<0 || nc>=8 { continue; }
                        let nr=nr as usize; let nc=nc as usize;
                        if let Some(occ)=base_board.get(nr,nc) { if occ.get_color()==side { continue; } }
                        let mut sim = base_board.clone();
                        if let Some(px)=sim.get(tr,tc) { sim.set(tr,tc,None); sim.set(nr,nc,Some(px)); }
                        let mut tmp = sim.clone();
                        let attacked = is_square_attacked_by_opponent(&mut tmp, (nr,nc), side);
                        let see1 = see_dest_estimate(&sim, side, (nr,nc), 0);
                        if !attacked || see1 >= 0 { knight_safe.push((nr,nc)); }
                    }
                }

                // Determine if still attacked after the candidate move
                let mut tmp2 = post.clone();
                let still_attacked = if (tr,tc) == from {
                    // piece moved -> check destination
                    let mut tmpmv = post.clone();
                    is_square_attacked_by_opponent(&mut tmpmv, to, side)
                } else {
                    if post.get(tr,tc).is_none() { false } else { is_square_attacked_by_opponent(&mut tmp2, (tr,tc), side) }
                };

                if (tr,tc) == from {
                    // We moved the threatened piece
                    // Reward safe evacuation
                    let mut evac_bonus = 0;
                    let see_new = see_dest_estimate(&post, side, to, 0);
                    // General safe-evacuation reward (non-pawn threats): keep modest
                    if !still_attacked || see_new >= 0 { evac_bonus += 400; }
                    if pt == PieceType::Knight {
                        // centralization bonus and preference for d5
                        let (nr,nc) = to;
                        let centers=[(3,3),(3,4),(4,3),(4,4)];
                        let mut cb=0; for &(cr,cc) in &centers { let dr = if nr>cr {nr-cr}else{cr-nr}; let dc=if nc>cc{nc-cc}else{cc-nc}; let dist=(dr+dc) as i32; cb=cb.max(60-10*dist); }
                        if to==(3,3) { cb += 80; }
                        if !knight_safe.is_empty() && knight_safe.iter().any(|&sq| sq==to) { cb += 80; }
                        evac_bonus += cb.max(0);
                    }
                    // Absolute priority clamp: if the knight was pawn-threatened and has any safe squares,
                    // give a large extra bonus specifically when we moved it to any of those safe squares.
                    if pt == PieceType::Knight && by_pawn && !knight_safe.is_empty() {
                        if knight_safe.iter().any(|&sq| sq==to) {
                            evac_bonus += 6000;
                        }
                    }
                    adjusted += side_mul * evac_bonus;
                } else {
                    // We did not move the threatened piece
                    // If a safe evacuation exists for a pawn-threatened knight, penalize ignoring it
                    if pt == PieceType::Knight && by_pawn && !knight_safe.is_empty() {
                        adjusted -= side_mul * 8000;
                    }
                    // General penalty for leaving an attacked minor as-is
                    if still_attacked {
                        let pen = match pt { PieceType::Knight|PieceType::Bishop => 200, PieceType::Rook=>120, PieceType::Queen=>80, PieceType::Pawn=>40, PieceType::King=>400 };
                        let val = if by_pawn { pen+400 } else { pen };
                        adjusted -= side_mul * val;
                    }
                }

                // Enforce absolute priority: if a pawn-threatened knight has safe evacuation squares,
                // demote all non-evacuation root moves below evacuation candidates by a large margin.
                if pt == PieceType::Knight && by_pawn && !knight_safe.is_empty() {
                    if (tr,tc) != from {
                        adjusted -= side_mul * 12000;
                    } else if knight_safe.iter().any(|&sq| sq==to) {
                        adjusted += side_mul * 8000;
                    }
                }
            }
        }
    }

    // Direct per-move heuristic: if any of our knights are attacked by an enemy pawn in the base
    // position, prefer moving that knight to a safe square and penalize any non-evacuation move.
    if !gives_check {
        // Locate pawn-threatened knights in base position
        let mut attacked_knights: Vec<(usize,usize)> = Vec::new();
        let opp = opposite_color(side);
        for r in 0..8 { for c in 0..8 {
            if let Some(p)=base_board.get(r,c) {
                if p.get_color()==side && p.get_type()==PieceType::Knight {
                    let pawn_threat = if opp==Color::White {
                        (r>0 && c>0 && matches!(base_board.get(r-1,c-1), Some(px) if px.get_color()==opp && px.get_type()==PieceType::Pawn)) ||
                        (r>0 && c+1<8 && matches!(base_board.get(r-1,c+1), Some(px) if px.get_color()==opp && px.get_type()==PieceType::Pawn))
                    } else {
                        (r+1<8 && c>0 && matches!(base_board.get(r+1,c-1), Some(px) if px.get_color()==opp && px.get_type()==PieceType::Pawn)) ||
                        (r+1<8 && c+1<8 && matches!(base_board.get(r+1,c+1), Some(px) if px.get_color()==opp && px.get_type()==PieceType::Pawn))
                    };
                    if pawn_threat { attacked_knights.push((r,c)); }
                }
            }
        }}
        if !attacked_knights.is_empty() {
            let side_mul: i32 = if side==Color::White { 1 } else { -1 };
            // If this move does NOT move any attacked knight, demote strongly
            if !attacked_knights.contains(&from) {
                adjusted -= side_mul * 10000;
            } else {
                // We are moving an attacked knight; check if destination is safe in base terms
                let (fr,fc) = from; let (tr,tc)=to;
                // Ensure the piece is a knight
                if let Some(p)=base_board.get(fr,fc) { if p.get_type()==PieceType::Knight {
                    // Simulate knight move on base
                    let mut sim = base_board.clone();
                    sim.set(fr,fc,None);
                    sim.set(tr,tc,Some(p));
                    let mut tmp = sim.clone();
                    let dest_attacked = is_square_attacked_by_opponent(&mut tmp, (tr,tc), side);
                    let see1 = see_dest_estimate(&sim, side, (tr,tc), 0);
                    if !dest_attacked || see1 >= 0 {
                        // Strong evacuation reward
                        let mut evac = 7000;
                        // Central/outpost preference
                        let centers=[(3,3),(3,4),(4,3),(4,4)];
                        for &(cr,cc) in &centers {
                            let dr = if tr>cr {tr-cr}else{cr-tr}; let dc = if tc>cc {tc-cc}else{cc-tc};
                            let dist=(dr+dc) as i32;
                            evac += (200 - 40*dist).max(0);
                        }
                        if (tr,tc)==(3,3) { evac += 800; } // d5 extra
                        adjusted += side_mul * evac;
                    } else {
                        // discourage moving into attacked/losing squares
                        adjusted -= side_mul * 3000;
                    }
                }}
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
        // Stronger tie-break for checking moves at root to favor forcing tactics.
        // Use side-aware sign so it helps both White (maximizer) and Black (minimizer).
        let side_mul: i32 = if side == Color::White { 1 } else { -1 };
        // Base check tie-break (side-aware): strong to let forcing moves compete with evacuations
        adjusted += side_mul * 400;

        // Opponent mobility after a checking move: fewer replies -> larger bonus.
        // Compute opponent legal moves in the checked position and scale bonus inversely.
        let opp_state = crate::state::game_state::GameState::from_board_and_side(post_probe.clone(), opp);
        let legals = crate::search::advanced_search::find_all_valid_moves(&opp_state);
        let replies = legals.len() as i32;
        // Scale: larger magnitudes so decisive checks can outrank passive evacuations at root.
        let mobility_bonus = match replies {
            0 => 4000, // mate next: huge bonus
            1..=2 => 3000,
            3..=5 => 2000,
            6..=8 => 1200,
            9..=12 => 600,
            _ => 200,
        };
        adjusted += side_mul * mobility_bonus;
    }

    // Queen king-side pressure heuristic: reward creating direct threats on f2/h2 or f7/h7
    // This captures common motifs like ...Qh4 aiming at f2/h2 (for Black) and Qh5/Qh4 ideas for White.
    if let Some(mp) = base_board.get(from.0, from.1) {
        if mp.get_type() == PieceType::Queen {
            let mut post = base_board.clone();
            post.set(from.0, from.1, None);
            post.set(to.0, to.1, Some(mp));

            let side_mul: i32 = if side == Color::White { 1 } else { -1 };
            let targets: &[(usize,usize)] = if side == Color::White { &[(1,5),(1,7)] } else { &[(6,5),(6,7)] }; // f7/h7 or f2/h2
            let mut hit_count = 0;
            for &sq in targets {
                let mut tmp = post.clone();
                // Pass opponent as active_color so the function checks attacks by our side
                let active_for_query = opposite_color(side);
                if is_square_attacked_by_opponent(&mut tmp, sq, active_for_query) {
                    hit_count += 1;
                }
            }
            if hit_count > 0 {
                let bonus = match hit_count { 1 => 350, _ => 700 };
                adjusted += side_mul * bonus;
            }
        }
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



