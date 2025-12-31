use crate::board::Board;
use crate::history::history::History;
use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{opposite_color, piece_value_cp, Color, Piece, PieceType};
use crate::search::search::find_all_valid_moves;
use crate::search::see::{see_dest_estimate, SEE_MINOR_SAC_THRESHOLD_CP};
use crate::search::tt::{decode_move, TranspositionTable};
use crate::search::zobrist::compute_zobrist;
use crate::state::fen::writer::game_state_to_fen_string;
use crate::state::game_state::GameState;


/// build_pv_for_root constructs the principal variation (PV) line starting from a given root move
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
        let legals = find_all_valid_moves(&tmp, side);
        if !legals.contains(&next) {
            break;
        }
        pv.push(next);
        let _u = tmp.make_move_simple( (nfr, nfc), (ntr, ntc));
        side = opposite_color(side);
    }
    pv
}

pub fn hard_root_filter(board: &Board, active_color: Color, v: &mut Vec<((usize, usize), (usize, usize))>, filtered: &mut Vec<((usize, usize), (usize, usize))>) {
    for (from, to) in v {
        let piece = board.get(from.0, from.1);
        let mut drop = false;
        if let Some(p) = piece {
            // simulate move on a temp board and evaluate SEE(dest)
            let mut post = board.clone();
            let captured = board.get(to.0, to.1);
            post.set(from.0, from.1, None);
            post.set(to.0, to.1, Some(p));
            let cap_val = captured
                .map(|cp| piece_value_cp(cp.get_type()))
                .unwrap_or(0);
            let see = see_dest_estimate(&post, active_color, *to, cap_val);
            match p.get_type() {
                PieceType::Queen => {
                    if see < 0 {
                        drop = true;
                    }
                }
                PieceType::Bishop | PieceType::Knight => {
                    // keep potential sound sacs if the move gives check; otherwise filter clearly losing ones
                    let gives_check =
                        post.is_side_in_check(opposite_color(active_color));
                    if see <= SEE_MINOR_SAC_THRESHOLD_CP && !gives_check {
                        drop = true;
                    }
                }
                _ => {}
            }
        }
        if !drop {
            filtered.push((*from, *to));
        }
    }
}

pub fn get_root_moves(game_state: GameState, history: &History, board: &Board, active_color: Color, moves: &Vec<((usize, usize), (usize, usize))>, v: &mut Vec<((usize, usize), (usize, usize))>) {
    for &(from, to) in moves {
        let is_capture = board.get(to.0, to.1).is_some();
        let mut gs = game_state; // GameState is Copy
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

// Small, root-level heuristic bonus used to break ties at low depth.
// Positive favors White; negative favors Black (we add for side to move).
pub fn root_move_bonus(board: &Board, from: (usize, usize), to: (usize, usize), side: Color) -> i32 {
    let mut bonus: i32 = 0;

    // Identify piece and basic metadata
    let piece = match board.get(from.0, from.1) {
        Some(p) => p,
        None => return 0,
    };
    let pt = piece.get_type();

    // Opening-principle nudges (very small):
    // - prefer central pawn advances (d/e pawns); discourage a/h pawn pushes
    // - prefer knights to c3/f3 and bishops to c4/f4 for White (mirror for Black)
    let (fr, fc) = from;
    let (tr, tc) = to;

    // discourage rook pawns (files a/h -> col 0/7) pushing as early plan
    if pt == PieceType::Pawn && (fc == 0 || fc == 7) {
        // stronger if double push (two ranks)
        let dr = if fr > tr {
            fr as i32 - tr as i32
        } else {
            tr as i32 - fr as i32
        };
        bonus -= if dr >= 2 { 35 } else { 25 };
    }

    // prefer central pawn advances on d/e files, especially 2-step from home
    if pt == PieceType::Pawn && (fc == 3 || fc == 4) {
        let dr = if fr > tr {
            fr as i32 - tr as i32
        } else {
            tr as i32 - fr as i32
        };
        bonus += if dr >= 2 { 35 } else { 20 };
    }

    // Removed older special-case guards; SEE-based root filter and penalties handle safety now.

    // Knights to c3/f3 (White) or c6/f6 (Black)
    if pt == PieceType::Knight {
        match side {
            Color::White => {
                if (tr, tc) == (2, 2) || (tr, tc) == (2, 5) {
                    bonus += 20;
                }
            }
            Color::Black => {
                if (tr, tc) == (5, 2) || (tr, tc) == (5, 5) {
                    bonus += 20;
                }
            }
        }
    }

    // Bishops to c4/f4 for White; c5/f5 for Black
    if pt == PieceType::Bishop {
        match side {
            Color::White => {
                if (tr, tc) == (3, 2) || (tr, tc) == (3, 5) {
                    bonus += 12;
                }
            }
            Color::Black => {
                if (tr, tc) == (4, 2) || (tr, tc) == (4, 5) {
                    bonus += 12;
                }
            }
        }
    }

    // Very small central control nudge for landing on or influencing center rings
    let central_files = tc >= 2 && tc <= 5; // c..f
    let central_ranks_white = tr >= 2 && tr <= 4; // ranks 3..5 from White pov
    let central_ranks_black = tr >= 3 && tr <= 5; // ranks 4..6 from White rows ~ Black push
    if central_files
        && ((side == Color::White && central_ranks_white)
        || (side == Color::Black && central_ranks_black))
    {
        bonus += 5;
    }

    // Apply sign for side to move (we always add for the maximizing side at root)
    match side {
        Color::White => bonus,
        Color::Black => -bonus,
    }
}

