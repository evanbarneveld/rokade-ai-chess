use crate::board::Board;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::history::history::History;
use crate::piece::pieces::{capture_value_cp, opposite_color, piece_value_cp, Color, Piece, PieceType};
use crate::search::alphabeta::alphabeta;
use crate::search::advanced_search::find_all_valid_moves;
use crate::search::see::{see_dest_estimate, SEE_PENALTY_MAX_CP, SEE_PENALTY_MIN_CP};
use crate::search::tt::{decode_move, TranspositionTable};
use crate::search::zobrist::compute_zobrist;
use crate::state::game_state::GameState;


const ROOT_CAPTURE_BONUS_DIV: i32 = 10; // add captured piece value / this divisor

const ENDGAME_SIDEADV_THRESHOLD_CP: i32 = 150; // only apply adjustments if side advantage above this
const ENDGAME_HMC_THRESHOLD: u32 = 80; // start scaling after this half-move clock
const ENDGAME_SCALE_MAX: i32 = 21; // max scaling steps used in the formula below
const ENDGAME_CAPTURE_SCALE_BONUS_CP: i32 = 15; // per-scale bonus if capture or pawn move
const ENDGAME_NONCAP_SCALE_PENALTY_CP: i32 = 8; // per-scale penalty if quiet move

// ---- Root heuristics constants (grouped; extracted from inline literals) ----
const CHECK_TIEBREAK_BASE: i32 = 1;
const KING_CAPTURE_ROOT_PENALTY: i32 = 5;
const KNIGHT_IGNORE_PAWN_THREAT_PENALTY: i32 = 4;
const KNIGHT_NON_EVAC_DEMOTION: i32 = 6;
const KNIGHT_SAFE_EVAC_REWARD: i32 = 3;
const KNIGHT_SAFE_TO_SPECIFIC_REWARD: i32 = 4;
const KNIGHT_CENTER_EXTRA_D5: i32 = 1;
const KNIGHT_CENTER_STEP: i32 = 0;
const CHECK_MOBILITY_BONUS_0: i32 = 5;
const CHECK_MOBILITY_BONUS_1_2: i32 = 2;
const CHECK_MOBILITY_BONUS_3_5: i32 = 1;
const CHECK_MOBILITY_BONUS_6_8: i32 = 0;
const CHECK_MOBILITY_BONUS_9_12: i32 = 0;
const CHECK_MOBILITY_BONUS_OTHER: i32 = 0;

// ---- Small helpers to reduce duplication (all #[inline]) ----
#[inline]
fn apply_for_side(v: i32, side: Color) -> i32 { if side == Color::White { v } else { -v } }

#[inline]
fn simulate_move(board: &Board, from: (usize,usize), to: (usize,usize)) -> (Board, Option<Piece>) {
    let mut b = board.clone();
    let moved = board.get(from.0, from.1);
    b.set(from.0, from.1, None);
    if let Some(p) = moved { b.set(to.0, to.1, Some(p)); }
    (b, moved)
}

#[inline]
fn see_after(board: &Board, side: Color, to: (usize,usize), captured: Option<Piece>) -> i32 {
    let cap = captured.map(|p| piece_value_cp(p.get_type())).unwrap_or(0);
    see_dest_estimate(board, side, to, cap)
}

#[inline]
fn attacked_by_pawn(board: &Board, sq: (usize,usize), attacker: Color) -> bool {
    let (r,c) = sq;
    match attacker {
        Color::White => (r>0 && c>0 && matches!(board.get(r-1,c-1), Some(p) if p.get_color()==attacker && p.get_type()==PieceType::Pawn))
                     || (r>0 && c+1<8 && matches!(board.get(r-1,c+1), Some(p) if p.get_color()==attacker && p.get_type()==PieceType::Pawn)),
        Color::Black => (r+1<8 && c>0 && matches!(board.get(r+1,c-1), Some(p) if p.get_color()==attacker && p.get_type()==PieceType::Pawn))
                     || (r+1<8 && c+1<8 && matches!(board.get(r+1,c+1), Some(p) if p.get_color()==attacker && p.get_type()==PieceType::Pawn)),
    }
}

#[inline]
fn center_score((r,c):(usize,usize)) -> i32 {
    let centers=[(3,3),(3,4),(4,3),(4,4)];
    centers.iter().map(|&(cr,cc)|{
        let dr = if r>cr {r-cr} else {cr-r};
        let dc = if c>cc {c-cc} else {cc-c};
        (60 - 10 * ((dr+dc) as i32)).max(0)
    }).max().unwrap_or(0)
}

#[inline]
fn knight_safe_squares(board: &Board, side: Color, from: (usize,usize)) -> Vec<(usize,usize)> {
    const DELTAS: [(isize,isize);8] = [(2,1),(2,-1),(-2,1),(-2,-1),(1,2),(1,-2),(-1,2),(-1,-2)];
    let (fr,fc)=from;
    let mut v=Vec::new();
    for (dr,dc) in DELTAS {
        let (nr,nc) = (fr as isize+dr, fc as isize+dc);
        if !(0..=7).contains(&nr) || !(0..=7).contains(&nc) { continue; }
        let (nr,nc) = (nr as usize, nc as usize);
        if let Some(occ)=board.get(nr,nc) { if occ.get_color()==side { continue; } }
        let (sim, _) = simulate_move(board, from, (nr,nc));
        let mut tmp = sim.clone();
        if !is_square_attacked_by_opponent(&mut tmp, (nr,nc), side) || see_dest_estimate(&sim, side, (nr,nc), 0) >= 0 {
            v.push((nr,nc));
        }
    }
    v
}

#[inline]
fn check_mobility_bonus_for_side(post_after: &Board, checked_side: Color) -> i32 {
    let opp_state = GameState::from_board_and_side(post_after.clone(), checked_side);
    let replies = find_all_valid_moves(&opp_state).len() as i32;
    match replies {
        0 => CHECK_MOBILITY_BONUS_0,
        1..=2 => CHECK_MOBILITY_BONUS_1_2,
        3..=5 => CHECK_MOBILITY_BONUS_3_5,
        6..=8 => CHECK_MOBILITY_BONUS_6_8,
        9..=12 => CHECK_MOBILITY_BONUS_9_12,
        _ => CHECK_MOBILITY_BONUS_OTHER,
    }
}

// / build_pv_for_root constructs the principal variation (PV) line starting from a given root move
/// by followthe ing best moves stored in the transposition table (TT), alternating sides, and validating
/// every step’s legality. It returns a list of move pairs (from, to) that represents the best‑known line
/// from the root, according to the TT.
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
        // Validate legality in the current position to avoid garbage PV
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

pub fn get_root_moves(_game_state: &GameState, _history: &History, _board: &Board, _active_color: Color, moves: &Vec<((usize, usize), (usize, usize))>, v: &mut Vec<((usize, usize), (usize, usize))>) {
    for &(from, to) in moves {
        v.push((from, to));
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

    // Keep only very small development/centralization nudges aren't modeled explicitly by eval:
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

    // Prepare post-position for general use
    let (mut post_after, moved_probe) = simulate_move(base_board, from, to);
    let opp = opposite_color(side);
    let gives_check = post_after.is_side_in_check(opp);
    let moved_is_queen = moved_probe.map(|p| p.get_type() == PieceType::Queen).unwrap_or(false);

    // 1) Destination SEE penalties and queen suicidal check guard
    adjusted += apply_destination_see_penalties(base_board, &post_after, side, from, to, is_capture, moved_is_pawn, gives_check, moved_is_queen);

    // 2) Threat resolution and evacuation (attacked minors etc.)
    adjusted += threat_resolution_and_evacuation(base_board, &post_after, side, from, to, gives_check);

    // 3) Direct per-move knight evacuation priority from pawn threats
    adjusted += knight_evacuations_priority(base_board, side, from, to, gives_check);

    // 4) Small capture bonus at root
    if let Some(captured) = base_board.get(to.0, to.1) {
        adjusted += capture_value_cp(captured.get_type()) / ROOT_CAPTURE_BONUS_DIV;
    }

    // 5) Endgame / 50-move rule pressure scaling
    adjusted += endgame_50move_scaling(side, score_raw, base_hmc, is_capture, moved_is_pawn);

    // 6) Extra king safety heuristics for king moves
    adjusted += king_safety_root_heuristics(base_board, side, from, to, is_capture);

    // 7) Self-hanging penalty aggregate (if not giving check) OR check tie-break + mobility bonus
    adjusted += self_hang_or_check_mobility(base_board, &post_after, side, from, to, gives_check, opp);

    // 8) Queen kingside pressure motifs (f2/h2 or f7/h7)
    adjusted += queen_kingside_pressure_bonus(base_board, side, from, to);

    adjusted
}

// ---- Helper delegates for adjust_root_score ----

#[inline]
fn apply_destination_see_penalties(
    base_board: &Board,
    post_after: &Board,
    side: Color,
    from: (usize, usize),
    to: (usize,usize),
    is_capture: bool,
    moved_is_pawn: bool,
    gives_check: bool,
    _moved_is_queen: bool,
) -> i32 {
    let mut delta = 0;
    if !gives_check {
        let captured = base_board.get(to.0, to.1);
        let see = see_after(post_after, side, to, captured);
        if see < 0 {
            let pen = (-see).clamp(SEE_PENALTY_MIN_CP, SEE_PENALTY_MAX_CP);
            delta += apply_for_side(-pen, side);
            if !is_capture && moved_is_pawn { delta += apply_for_side(-SEE_PENALTY_MIN_CP, side); }
        }
    } else {
        // Guard suicidal checking moves for major pieces using SEE at the destination.
        // Previously this only applied to the queen; extend to rooks (and lightly to minors)
        let captured = base_board.get(to.0, to.1);
        let see = see_after(post_after, side, to, captured);
        if see < 0 {
            // Determine moved piece type
            let moved_pt = base_board
                .get(from.0, from.1)
                .map(|p| p.get_type());
            // Scale penalty based on piece importance
            let pen = match moved_pt {
                // Keep queen guard as before
                Some(PieceType::Queen) => ((-see) * 6).clamp(600, 6000),
                // Strengthen rook guard: checking moves that hang the rook should be strongly discouraged
                // Set a higher minimum clamp to overcome generic check-mobility bonuses.
                Some(PieceType::Rook) => ((-see) * 4).clamp(3600, 6000),
                // Increase minors penalty so that hanging knight/bishop checks are avoided more reliably
                Some(PieceType::Bishop) | Some(PieceType::Knight) => ((-see) * 4).clamp(600, 3600),
                Some(PieceType::Pawn) => ((-see) * 2).clamp(200, 2000),
                _ => ((-see) * 3).clamp(300, 3000),
            };
            // Temporary debug: trace SEE penalty for checking moves
            //if let Some(mp) = moved_pt { if mp == PieceType::Rook {
            //    println!("[ROOT DEBUG] Rook checking move {:?}->{:?}: SEE={} pen={} (side={:?})", from, to, see, pen, side);
            //}}
            // side-aware penalty (penalize mover regardless of color)
            delta += apply_for_side(-pen, side);
        }

        // Additional specific guard for hanging knight/bishop checks immediately
        // recaptured by a pawn on the destination square (e.g., Na5+ b4xa5).
        // We check for opponent pawn attacks on the destination in the post position.
        if let Some(moved_pt) = base_board.get(from.0, from.1).map(|p| p.get_type()) {
            if moved_pt == PieceType::Knight || moved_pt == PieceType::Bishop {
                let opp = opposite_color(side);
                if attacked_by_pawn(post_after, to, opp) {
                    // Apply a strong penalty to outweigh generic check bonuses and prefer safer alternatives
                    let strong_minor_pen = 1200; // ~ minor piece value with margin
                    delta += apply_for_side(-strong_minor_pen, side);
                }
            }
        }

        // Additional direct guard for checking moves: if the opponent king can immediately
        // capture the checking piece on the destination square and remain safe, apply a
        // very strong penalty regardless of SEE approximation.
        // This specifically addresses patterns like Rd1+, Re1+, Rxb3+ dropping the rook to Kx,
        // and also Bishop/ Knight checks such as Bxf7+ where Kxf7 is safe.
        // Determine moved piece type first.
        if let Some(moved_pt2) = base_board.get(from.0, from.1).map(|p| p.get_type()) {
            // Consider heavy penalty for all pieces that commonly blunder on checking sacrifices.
            // Previously only rook/queen; extend to bishops/knights as well to avoid Bxf7+, Na5+ blunders.
            if matches!(moved_pt2, PieceType::Rook | PieceType::Queen | PieceType::Bishop | PieceType::Knight) {
                let opp = opposite_color(side);
                // Find opponent king square in the post position (after our checking move)
                let mut king_sq: Option<(usize,usize)> = None;
                'seek: for r in 0..8 { for c in 0..8 {
                    if let Some(px) = post_after.get(r,c) {
                        if px.get_color()==opp && px.get_type()==PieceType::King { king_sq = Some((r,c)); break 'seek; }
                    }
                }}
                if let Some((kr,kc)) = king_sq {
                    // King can capture if destination is adjacent and occupied by our moved piece
                    let dr = if kr>to.0 { kr - to.0 } else { to.0 - kr };
                    let dc = if kc>to.1 { kc - to.1 } else { to.1 - kc };
                    let adjacent = dr <= 1 && dc <= 1;
                    if adjacent {
                        // Ensure the destination holds our moved piece in the post position
                        if let Some(on_to) = post_after.get(to.0, to.1) {
                            if on_to.get_color()==side {
                                // Simulate king capture and verify king safety
                                let mut after_kx = post_after.clone();
                                // Remove king from old square
                                after_kx.set(kr, kc, None);
                                // Place king on destination (capturing our piece)
                                after_kx.set(to.0, to.1, Some(Piece::new(PieceType::King, opp)));
                                // Check if king on new square is attacked by our side; if not, it's a safe capture
                                let mut tmp_chk = after_kx.clone();
                                let unsafe_for_king = is_square_attacked_by_opponent(&mut tmp_chk, to, opp);
                                if !unsafe_for_king {
                                    // Apply a strong penalty: at least the piece value plus margin to overcome check bonuses
                                    let base_pen = match moved_pt2 {
                                        PieceType::Queen => 900,
                                        PieceType::Rook => 500,
                                        PieceType::Bishop | PieceType::Knight => 300,
                                        _ => 200,
                                    };
                                    // Scale a bit stronger for minors to ensure avoidance at low depths
                                    let scale = match moved_pt2 {
                                        PieceType::Queen => 8,
                                        PieceType::Rook => 8,
                                        PieceType::Bishop | PieceType::Knight => 10,
                                        _ => 8,
                                    };
                                    let strong_pen = (base_pen * scale).clamp(2400, 8000);
                                    //println!("[ROOT DEBUG] Opp king safe Kx on {:?}; applying strong_pen={} for {:?}->{:?}", to, strong_pen, from, to);
                                    delta += apply_for_side(-strong_pen, side);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    delta
}

#[inline]
fn threat_resolution_and_evacuation(
    base_board: &Board,
    post_after: &Board,
    side: Color,
    from: (usize,usize),
    to: (usize,usize),
    gives_check: bool,
) -> i32 {
    if gives_check { return 0; }
    let mut base_clone = base_board.clone();
    let mut threatened: Vec<(usize, usize, PieceType, bool)> = Vec::new();
    for r in 0..8 { for c in 0..8 {
        if let Some(p) = base_board.get(r,c) {
            if p.get_color() != side { continue; }
            if !is_square_attacked_by_opponent(&mut base_clone, (r,c), side) { continue; }
            let opp = opposite_color(side);
            let pawn_attacks = attacked_by_pawn(base_board, (r,c), opp);
            threatened.push((r,c,p.get_type(), pawn_attacks));
        }
    }}
    if threatened.is_empty() { return 0; }
    let mut delta = 0;
    for (tr,tc,pt,by_pawn) in threatened {
        // Knight-specific: precompute safe escape squares in base
        let knight_safe: Vec<(usize,usize)> = if pt == PieceType::Knight && by_pawn { knight_safe_squares(base_board, side, (tr,tc)) } else { Vec::new() };

        // Determine if still attacked after the candidate move
        let mut tmp2 = post_after.clone();
        let still_attacked = if (tr,tc) == from {
            let mut tmpmv = post_after.clone();
            is_square_attacked_by_opponent(&mut tmpmv, to, side)
        } else {
            if post_after.get(tr,tc).is_none() { false } else { is_square_attacked_by_opponent(&mut tmp2, (tr,tc), side) }
        };

        if (tr,tc) == from {
            // We moved the threatened piece
            let mut evac_bonus = 0;
            let see_new = see_dest_estimate(post_after, side, to, 0);
            if !still_attacked || see_new >= 0 { evac_bonus += 400; }
            if pt == PieceType::Knight {
                let mut cb = center_score(to);
                if to == (3,3) { cb += 80; }
                if !knight_safe.is_empty() && knight_safe.iter().any(|&sq| sq==to) { cb += 80; }
                evac_bonus += cb.max(0);
            }
            if pt == PieceType::Knight && by_pawn && !knight_safe.is_empty() {
                if knight_safe.iter().any(|&sq| sq==to) { evac_bonus += KNIGHT_SAFE_TO_SPECIFIC_REWARD; }
            }
            delta += apply_for_side(evac_bonus, side);
        } else {
            if pt == PieceType::Knight && by_pawn && !knight_safe.is_empty() {
                delta -= apply_for_side(KNIGHT_IGNORE_PAWN_THREAT_PENALTY, side);
            }
            if still_attacked {
                let pen = match pt { PieceType::Knight|PieceType::Bishop => 200, PieceType::Rook=>120, PieceType::Queen=>80, PieceType::Pawn=>40, PieceType::King=>400 };
                let val = if by_pawn { pen+400 } else { pen };
                delta -= apply_for_side(val, side);
            }
        }

        if pt == PieceType::Knight && by_pawn && !knight_safe.is_empty() {
            if (tr,tc) != from {
                delta -= apply_for_side(KNIGHT_NON_EVAC_DEMOTION, side);
            } else if knight_safe.iter().any(|&sq| sq==to) {
                delta += apply_for_side(KNIGHT_SAFE_TO_SPECIFIC_REWARD, side);
            }
        }
    }
    delta
}

#[inline]
fn knight_evacuations_priority(
    base_board: &Board,
    side: Color,
    from: (usize,usize),
    to: (usize,usize),
    gives_check: bool,
) -> i32 {
    if gives_check { return 0; }
    let opp = opposite_color(side);
    let mut attacked_knights: Vec<(usize,usize)> = Vec::new();
    for r in 0..8 { for c in 0..8 {
        if let Some(p)=base_board.get(r,c) {
            if p.get_color()==side && p.get_type()==PieceType::Knight {
                if attacked_by_pawn(base_board, (r,c), opp) { attacked_knights.push((r,c)); }
            }
        }
    }}
    if attacked_knights.is_empty() { return 0; }
    let mut delta = 0;
    if !attacked_knights.contains(&from) {
        delta -= apply_for_side(500, side);
    } else if let Some(p)=base_board.get(from.0, from.1) { if p.get_type()==PieceType::Knight {
        let (fr,fc) = from; let (tr,tc)=to;
        let (sim, _) = simulate_move(base_board, (fr,fc), (tr,tc));
        let mut tmp = sim.clone();
        let dest_attacked = is_square_attacked_by_opponent(&mut tmp, (tr,tc), side);
        let see1 = see_dest_estimate(&sim, side, (tr,tc), 0);
        if !dest_attacked || see1 >= 0 {
            let mut evac = KNIGHT_SAFE_EVAC_REWARD;
            for &(cr,cc) in &[(3,3),(3,4),(4,3),(4,4)] {
                let dr = if tr>cr {tr-cr}else{cr-tr}; let dc = if tc>cc {tc-cc}else{cc-tc};
                let dist=(dr+dc) as i32; evac += (20 - KNIGHT_CENTER_STEP*dist).max(0);
            }
            if (tr,tc)==(3,3) { evac += KNIGHT_CENTER_EXTRA_D5; }
            delta += apply_for_side(evac, side);
        } else {
            delta -= apply_for_side(150, side);
        }
    }}
    delta
}

#[inline]
fn endgame_50move_scaling(side: Color, score_raw: i32, base_hmc: u32, is_capture: bool, moved_is_pawn: bool) -> i32 {
    let side_adv = apply_for_side(score_raw, side);
    if side_adv > ENDGAME_SIDEADV_THRESHOLD_CP && base_hmc >= ENDGAME_HMC_THRESHOLD {
        let scale = (base_hmc as i32 - (ENDGAME_HMC_THRESHOLD as i32 - 1)).min(ENDGAME_SCALE_MAX);
        if is_capture || moved_is_pawn { ENDGAME_CAPTURE_SCALE_BONUS_CP * scale } else { -ENDGAME_NONCAP_SCALE_PENALTY_CP * scale }
    } else { 0 }
}

#[inline]
fn king_safety_root_heuristics(base_board: &Board, side: Color, from: (usize,usize), to: (usize,usize), is_capture: bool) -> i32 {
    let moved_is_king = base_board.get(from.0, from.1).map(|p| p.get_type() == PieceType::King).unwrap_or(false);
    if !moved_is_king { return 0; }
    let (mut postk, _) = simulate_move(base_board, from, to);
    let mut delta = 0;
    if is_square_attacked_by_opponent(&mut postk, to, side) { delta -= 50; }
    if is_capture { delta -= KING_CAPTURE_ROOT_PENALTY; }
    delta
}

#[inline]
fn self_hang_or_check_mobility(
    _base_board: &Board,
    post_after: &Board,
    side: Color,
    _from: (usize, usize),
    _to: (usize, usize),
    gives_check: bool,
    opp: Color,
) -> i32 {
    let mut total_penalty: i32 = 0;
    // Scan our pieces; if any square is attacked and SEE < 0 for us, penalize
    for r in 0..8 {
        for c in 0..8 {
            if let Some(p) = post_after.get(r, c) {
                if p.get_color() != side {
                    continue;
                }
                let mut post_for_query = post_after.clone();
                if !is_square_attacked_by_opponent(&mut post_for_query, (r, c), side) {
                    continue;
                }
                let see = see_dest_estimate(post_after, side, (r, c), 0);
                if see < 0 {
                    let pen = (-see).clamp(SEE_PENALTY_MIN_CP, SEE_PENALTY_MAX_CP) / 2;
                    total_penalty += pen;
                    if p.get_type() == PieceType::Queen {
                        let q_extra = ((-see) * 12).clamp(40, 120);
                        total_penalty += q_extra;
                    }
                }
            }
        }
    }
    let agg_cap: i32 = 1000;
    let hang_pen = if total_penalty > 0 {
        -total_penalty.min(agg_cap)
    } else {
        0
    };

    let mut check_bonus = 0;
    if gives_check {
        check_bonus += CHECK_TIEBREAK_BASE;
        check_bonus += check_mobility_bonus_for_side(post_after, opp);
    }

    apply_for_side(hang_pen + check_bonus, side)
}

#[inline]
fn queen_kingside_pressure_bonus(base_board: &Board, side: Color, from: (usize,usize), to: (usize,usize)) -> i32 {
    if let Some(mp) = base_board.get(from.0, from.1) {
        if mp.get_type() == PieceType::Queen {
            let (post, _) = simulate_move(base_board, from, to);
            let targets: &[(usize,usize)] = if side == Color::White { &[(1,5),(1,7)] } else { &[(6,5),(6,7)] };
            let mut hit_count = 0;
            for &sq in targets {
                let mut tmp = post.clone();
                let active_for_query = opposite_color(side);
                if is_square_attacked_by_opponent(&mut tmp, sq, active_for_query) { hit_count += 1; }
            }
            if hit_count > 0 {
                let bonus = match hit_count { 1 => 1, _ => 2 };
                return apply_for_side(bonus, side);
            }
        }
    }
    0
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
    history: &History,
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
            &mut history.get_rep_stack(),
        )
    } else {
        let mut rep_stack = history.get_rep_stack();
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
    let mut adj = adjust_root_score(
        base_board, side, from, to, base_hmc, is_capture, moved_is_pawn, score_raw, ps,
    );
    
    // Safety rule: if search says a move is losing, do not let root heuristics 
    // pump it up to look like a win. This prevents gambling against the search result.
    if score_raw < 0 {
        if side == Color::White {
            adj = adj.min(score_raw);
        } else {
            adj = adj.max(score_raw);
        }
    }

    // If it's a draw by repetition (raw score 0 and likely from alphabeta),
    // ensure heuristics don't drag it down.
    if score_raw == 0 {
        adj = adj.max(-10);
    }
    adj
}



