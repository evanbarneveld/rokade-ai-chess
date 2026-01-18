use crate::board::Board;
use crate::piece::as_square_str;
use crate::piece::pieces::{opposite_color, Color, Piece, PieceType};
use crate::search::core::advanced_search::find_all_valid_moves;
use crate::state::game_state::GameState;

pub fn convert_move_to_san(
    game_state: GameState,
    generated_move: Option<((usize, usize), (usize, usize), Option<char>)>,
) -> Option<String> {
    let (from, to, promo) = generated_move?;
    let board = game_state.board();

    let moving_piece: Piece = board.get(from.0, from.1)?;
    let pt = moving_piece.get_type();
    let side = moving_piece.get_color();

    // 1) Castling detection (O-O / O-O-O)
    if pt == PieceType::King && from.0 == to.0 {
        let dc = from.1.abs_diff(to.1);
        if dc == 2 {
            // King side vs queen side
            let san = if to.1 == from.1 + 2 { "O-O" } else { "O-O-O" };
            // Determine +/#
            let check_suffix = check_or_mate_suffix(board, from, to, promo, side);
            return Some(format!("{}{}", san, check_suffix));
        }
    }

    // 2) Capture detection, including en-passant heuristic for pawns (diagonal move to empty square)
    let is_target_occupied = board.get(to.0, to.1).is_some();
    let is_pawn_diagonal = pt == PieceType::Pawn && from.1 != to.1 && from.0 != to.0;
    let is_capture = is_target_occupied || is_pawn_diagonal;

    // 3) Build SAN components
    let mut san = String::new();
    if pt != PieceType::Pawn {
        san.push(piece_letter(pt));
        // Disambiguation for N/B/R/Q when another same-type piece can also move to 'to'
        let disamb = disambiguation_string(board, side, pt, from, to);
        san.push_str(&disamb);
        if is_capture { san.push('x'); }
        san.push_str(&as_square_str(to));
    } else {
        // Pawn SAN: file letter on capture, just destination on quiet
        if is_capture {
            san.push(file_char(from.1));
            san.push('x');
        }
        san.push_str(&as_square_str(to));
        // Promotion
        if let Some(pc) = promo {
            san.push('=');
            san.push(pc.to_ascii_uppercase());
        } else if (side == Color::White && to.0 == 7) || (side == Color::Black && to.0 == 0) {
            // fallback for missing promo info
            san.push_str("=Q");
        }
    }

    // 4) Append + or #
    let suffix = check_or_mate_suffix(board, from, to, promo, side);
    san.push_str(&suffix);
    Some(san)
}

#[inline]
fn piece_letter(pt: PieceType) -> char {
    match pt {
        PieceType::King => 'K',
        PieceType::Queen => 'Q',
        PieceType::Rook => 'R',
        PieceType::Bishop => 'B',
        PieceType::Knight => 'N',
        PieceType::Pawn => panic!("Pawns have no piece letter in SAN"),
    }
}

#[inline]
fn file_char(col: usize) -> char {
    (b'a' + (col as u8)) as char
}

#[inline]
fn rank_char(row: usize) -> char {
    // internal row 0..7 == ranks 1..8
    (b'1' + (row as u8)) as char
}

// Determine if the position after (from->to) is checked or mate, and return "", "+" or "#"
fn check_or_mate_suffix(board: &Board, from: (usize, usize), to: (usize, usize), promo: Option<char>, side: Color) -> String {
    let mut tmp = *board;
    let _u = tmp.make_move_simple(from, to, promo);
    let opp = opposite_color(side);
    let in_check = tmp.is_side_in_check(opp);
    if !in_check {
        return String::new();
    }
    // Check for mate: if opponent has no legal moves
    let mut opp_state = GameState::from_board_and_side(tmp, opp);
    let legals = find_all_valid_moves(&mut opp_state);
    if legals.is_empty() { "#".to_string() } else { "+".to_string() }
}

// Compute SAN disambiguation for N/B/R/Q
fn disambiguation_string(
    board: &Board,
    side: Color,
    pt: PieceType,
    from: (usize, usize),
    to: (usize, usize),
) -> String {
    // Collect candidates: other same-type pieces of same color that can legally move to 'to'
    let mut same_file_conflict = false;
    let mut same_rank_conflict = false;
    let mut any_conflict = false;

    for r in 0..8 {
        for c in 0..8 {
            if (r, c) == from { continue; }
            if let Some(p) = board.get(r, c) {
                if p.get_color() != side || p.get_type() != pt { continue; }
                if can_piece_move_to(*board, pt, (r, c), to) {
                    any_conflict = true;
                    if c == from.1 { same_file_conflict = true; }
                    if r == from.0 { same_rank_conflict = true; }
                }
            }
        }
    }

    if !any_conflict {
        return String::new();
    }
    if !same_file_conflict {
        return file_char(from.1).to_string();
    }
    if !same_rank_conflict {
        return rank_char(from.0).to_string();
    }
    format!("{}{}", file_char(from.1), rank_char(from.0))
}

// Use validators with pin/self-check enabled to test if a piece at `from` could legally move to `to`.
fn can_piece_move_to(mut board: Board, pt: PieceType, from: (usize, usize), to: (usize, usize)) -> bool {
    match pt {
        PieceType::Knight => {
            crate::piece::move_validators::knight_move_validator::is_valid_knight_move(&mut board, from, to, true)
        }
        PieceType::Bishop => {
            crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move(&mut board, from, to, true)
        }
        PieceType::Rook => {
            crate::piece::move_validators::rook_move_validator::is_valid_rook_move(&mut board, from, to, true)
        }
        PieceType::Queen => {
            crate::piece::move_validators::queen_move_validator::is_valid_queen_move(&mut board, from, to, true)
        }
        PieceType::King => false, // no disambiguation needed (unique piece per side)
        PieceType::Pawn => false, // handled separately in SAN; not needed for disambiguation
    }
}
