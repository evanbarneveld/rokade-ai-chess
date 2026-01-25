use crate::piece::pieces::{Color, Piece, PieceType};
use crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move;
use crate::piece::move_validators::king_move_validator::is_valid_king_move;
use crate::piece::move_validators::knight_move_validator::is_valid_knight_move;
use crate::piece::move_validators::pawn_move_validator::is_valid_pawn_move;
use crate::piece::move_validators::queen_move_validator::is_valid_queen_move;
use crate::piece::move_validators::rook_move_validator::is_valid_rook_move;
use crate::state::game_state::GameState;

/// Generate target squares for a piece based on its type and position.
/// Returns a vector of potential destination squares (not validated yet).
#[inline]
fn generate_piece_targets_into(
    piece_type: PieceType,
    from: (usize, usize),
    color: Color,
    targets: &mut Vec<(usize, usize)>,
) {
    let (r, c) = from;
    targets.clear();

    match piece_type {
        PieceType::Knight => {
            // Knight: exactly 8 possible moves
            const KNIGHT_MOVES: [(isize, isize); 8] = [
                (2, 1), (2, -1), (-2, 1), (-2, -1),
                (1, 2), (1, -2), (-1, 2), (-1, -2)
            ];
            for (dr, dc) in KNIGHT_MOVES {
                let nr = r as isize + dr;
                let nc = c as isize + dc;
                if (0..8).contains(&nr) && (0..8).contains(&nc) {
                    targets.push((nr as usize, nc as usize));
                }
            }
        }
        PieceType::King => {
            // King: up to 8 adjacent squares
            for dr in -1..=1 {
                for dc in -1..=1 {
                    if dr == 0 && dc == 0 { continue; }
                    let nr = r as isize + dr;
                    let nc = c as isize + dc;
                    if (0..8).contains(&nr) && (0..8).contains(&nc) {
                        targets.push((nr as usize, nc as usize));
                    }
                }
            }

            // Castling: king moves 2 squares left or right from starting position
            let start_row = if color == Color::White { 0 } else { 7 };
            let start_col = 4;
            if from == (start_row, start_col) {
                // Kingside castling (move to column 6)
                targets.push((start_row, 6));
                // Queenside castling (move to column 2)
                targets.push((start_row, 2));
            }
        }
        PieceType::Pawn => {
            // Pawn: forward moves and captures
            let forward = if color == Color::White { 1 } else { -1 };
            let start_rank = if color == Color::White { 1 } else { 6 };

            // Single push
            let nr = r as isize + forward;
            if (0..8).contains(&nr) {
                targets.push((nr as usize, c));

                // Double push from starting position
                if r == start_rank {
                    let nr2 = r as isize + 2 * forward;
                    if (0..8).contains(&nr2) {
                        targets.push((nr2 as usize, c));
                    }
                }
            }

            // Captures (diagonal)
            for dc in [-1, 1] {
                let nr = r as isize + forward;
                let nc = c as isize + dc;
                if (0..8).contains(&nr) && (0..8).contains(&nc) {
                    targets.push((nr as usize, nc as usize));
                }
            }
        }
        PieceType::Rook | PieceType::Bishop | PieceType::Queen => {
            // Sliding pieces: generate rays
            let directions = match piece_type {
                PieceType::Rook => vec![(0, 1), (0, -1), (1, 0), (-1, 0)],
                PieceType::Bishop => vec![(1, 1), (1, -1), (-1, 1), (-1, -1)],
                PieceType::Queen => vec![
                    (0, 1), (0, -1), (1, 0), (-1, 0),
                    (1, 1), (1, -1), (-1, 1), (-1, -1)
                ],
                _ => vec![],
            };

            for (dr, dc) in directions {
                let mut nr = r as isize + dr;
                let mut nc = c as isize + dc;
                while (0..8).contains(&nr) && (0..8).contains(&nc) {
                    targets.push((nr as usize, nc as usize));
                    nr += dr;
                    nc += dc;
                }
            }
        }
    }
}

/// Helper function to convert promotion char to PieceType
#[inline]
fn promo_char_to_piece_type(pc: char) -> PieceType {
    match pc {
        'q' => PieceType::Queen,
        'r' => PieceType::Rook,
        'b' => PieceType::Bishop,
        'n' => PieceType::Knight,
        _ => PieceType::Queen,
    }
}

/// Helper function to convert PieceType to promotion char
#[inline]
fn piece_type_to_promo_char(pt: PieceType) -> Option<char> {
    match pt {
        PieceType::Queen => Some('q'),
        PieceType::Rook => Some('r'),
        PieceType::Bishop => Some('b'),
        PieceType::Knight => Some('n'),
        _ => None,
    }
}

/// Check if a pawn move is a promotion
#[inline]
fn is_promotion(to: (usize, usize), active_color: Color) -> bool {
    (active_color == Color::White && to.0 == 7)
        || (active_color == Color::Black && to.0 == 0)
}

/// Try to make a move and check if it's legal (doesn't leave king in check).
/// Returns true if the move is legal.
/// This is a lightweight wrapper around the move + legality check to make the intent clear.
#[inline]
fn try_move_and_check_legality(
    game_state: &mut GameState,
    from: (usize, usize),
    to: (usize, usize),
    promo: Option<char>,
    active_color: Color,
) -> bool {
    let u = game_state.make_move_fast(from, to, promo);
    let in_check = game_state.mutable_board().is_side_in_check(active_color);
    game_state.unmake_move_fast(u);
    !in_check
}

#[inline]
fn is_valid_piece_move(
    game_state: &mut GameState,
    piece_type: PieceType,
    from: (usize, usize),
    to: (usize, usize),
    is_capture: bool,
    promo_piece: Option<Piece>,
    active_color: Color,
) -> bool {
    match piece_type {
        PieceType::Pawn => {
            let ep = game_state.en_passant_target();
            let board = game_state.mutable_board();
            is_valid_pawn_move(board, from, to, is_capture, ep, active_color, promo_piece, false)
        }
        PieceType::Knight => {
            let board = game_state.mutable_board();
            is_valid_knight_move(board, from, to, false)
        }
        PieceType::Bishop => {
            let board = game_state.mutable_board();
            is_valid_bishop_move(board, from, to, false)
        }
        PieceType::Rook => {
            let board = game_state.mutable_board();
            is_valid_rook_move(board, from, to, false)
        }
        PieceType::Queen => {
            let board = game_state.mutable_board();
            is_valid_queen_move(board, from, to, false)
        }
        PieceType::King => is_valid_king_move(game_state, from, to),
    }
}

fn piece_attacks_square(
    game_state: &mut GameState,
    from: (usize, usize),
    to: (usize, usize),
    piece_type: PieceType,
    active_color: Color,
) -> bool {
    let board = game_state.mutable_board();
    match piece_type {
        PieceType::Pawn => {
            let dr = match active_color {
                Color::White => 1i32,
                Color::Black => -1i32,
            };
            let rr = from.0 as i32 + dr;
            if rr != to.0 as i32 {
                return false;
            }
            let dc = (to.1 as i32 - from.1 as i32).abs();
            dc == 1
        }
        PieceType::Knight => is_valid_knight_move(board, from, to, false),
        PieceType::Bishop => is_valid_bishop_move(board, from, to, false),
        PieceType::Rook => is_valid_rook_move(board, from, to, false),
        PieceType::Queen => is_valid_queen_move(board, from, to, false),
        PieceType::King => {
            let dr = from.0.abs_diff(to.0);
            let dc = from.1.abs_diff(to.1);
            dr <= 1 && dc <= 1 && (dr != 0 || dc != 0)
        }
    }
}

fn checker_line_squares(
    king: (usize, usize),
    checker: (usize, usize),
) -> Vec<(usize, usize)> {
    let mut squares = Vec::new();
    let dr = checker.0 as i32 - king.0 as i32;
    let dc = checker.1 as i32 - king.1 as i32;
    let step_r = dr.signum();
    let step_c = dc.signum();

    if dr == 0 || dc == 0 || dr.abs() == dc.abs() {
        let mut r = king.0 as i32 + step_r;
        let mut c = king.1 as i32 + step_c;
        while (r, c) != (checker.0 as i32, checker.1 as i32) {
            squares.push((r as usize, c as usize));
            r += step_r;
            c += step_c;
        }
    }

    squares
}

fn generate_king_evasions(
    game_state: &mut GameState,
    from: (usize, usize),
    active_color: Color,
    out: &mut Vec<((usize, usize), (usize, usize), Option<char>)>,
) {
    let mut targets: Vec<(usize, usize)> = Vec::with_capacity(8);
    generate_piece_targets_into(PieceType::King, from, active_color, &mut targets);
    for to in targets {
        let is_capture = game_state.board().get(to.0, to.1).is_some();
        if !is_valid_piece_move(game_state, PieceType::King, from, to, is_capture, None, active_color) {
            continue;
        }
        out.push((from, to, None));
    }
}

pub fn find_all_valid_moves(
    game_state: &mut GameState,
) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    let mut result: Vec<((usize, usize), (usize, usize), Option<char>)> = Vec::new();
    let active_color = game_state.active_color();
    let en_passant_target = game_state.en_passant_target();
    let mut targets: Vec<(usize, usize)> = Vec::with_capacity(32);

    for r in 0..8 {
        for c in 0..8 {
            let piece = match game_state.board().get(r, c) {
                Some(p) => p,
                None => continue,
            };
            if piece.get_color() != active_color {
                continue;
            }

            let from = (r, c);
            let piece_type = piece.get_type();

            // Generate piece-specific target squares instead of checking all 64 squares
            generate_piece_targets_into(piece_type, from, active_color, &mut targets);

            for to in targets.iter().copied() {
                let target_piece = game_state.board().get(to.0, to.1);
                let is_capture = target_piece.is_some()
                    || (piece_type == PieceType::Pawn
                        && en_passant_target.is_some()
                        && to == en_passant_target.unwrap());

                if !game_state.move_from_and_to_validation_check(
                    from,
                    to,
                    is_capture,
                    piece_type == PieceType::Pawn,
                ) {
                    continue;
                }

                let is_pawn_promotion = piece_type == PieceType::Pawn && is_promotion(to, active_color);

                if is_pawn_promotion {
                    // Generate all 4 promotion options and filter by legality
                    let promo_chars = ['q', 'r', 'b', 'n'];
                    for &pc in promo_chars.iter() {
                        let promo_piece = Some(Piece::new(promo_char_to_piece_type(pc), active_color));
                        if !is_valid_piece_move(game_state, piece_type, from, to, is_capture, promo_piece, active_color) {
                            continue;
                        }
                        if piece_type == PieceType::King
                            || try_move_and_check_legality(game_state, from, to, Some(pc), active_color)
                        {
                            result.push((from, to, Some(pc)));
                        }
                    }
                } else {
                    // Regular move - check legality
                    if !is_valid_piece_move(game_state, piece_type, from, to, is_capture, None, active_color) {
                        continue;
                    }
                    if piece_type == PieceType::King
                        || try_move_and_check_legality(game_state, from, to, None, active_color)
                    {
                        result.push((from, to, None));
                    }
                }
            }
        }
    }
    result
}

/// Generate only legal check evasion moves for the active side.
pub fn find_all_evasion_moves(
    game_state: &mut GameState,
) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    let active_color = game_state.active_color();
    if !game_state.mutable_board().is_side_in_check(active_color) {
        return find_all_valid_moves(game_state);
    }

    if game_state.en_passant_target().is_some() {
        return find_all_valid_moves(game_state);
    }

    let king_sq = game_state.board().get_king_location(active_color);
    let mut checkers: Vec<((usize, usize), PieceType)> = Vec::new();
    for r in 0..8 {
        for c in 0..8 {
            let piece = match game_state.board().get(r, c) {
                Some(p) => p,
                None => continue,
            };
            if piece.get_color() == active_color {
                continue;
            }
            if piece_attacks_square(game_state, (r, c), king_sq, piece.get_type(), piece.get_color()) {
                checkers.push(((r, c), piece.get_type()));
            }
        }
    }

    let mut result: Vec<((usize, usize), (usize, usize), Option<char>)> = Vec::new();
    let king_from = king_sq;
    generate_king_evasions(game_state, king_from, active_color, &mut result);

    if checkers.len() != 1 {
        return result;
    }

    let checker_sq = checkers[0].0;
    let checker_pt = checkers[0].1;
    let mut block_squares = Vec::new();
    if matches!(checker_pt, PieceType::Bishop | PieceType::Rook | PieceType::Queen) {
        block_squares = checker_line_squares(king_sq, checker_sq);
    }

    let mut targets: Vec<(usize, usize)> = Vec::with_capacity(32);
    for r in 0..8 {
        for c in 0..8 {
            let piece = match game_state.board().get(r, c) {
                Some(p) => p,
                None => continue,
            };
            if piece.get_color() != active_color || piece.get_type() == PieceType::King {
                continue;
            }

            let from = (r, c);
            let piece_type = piece.get_type();
            generate_piece_targets_into(piece_type, from, active_color, &mut targets);
            for to in targets.iter().copied() {
                if to != checker_sq && !block_squares.contains(&to) {
                    continue;
                }
                let target_piece = game_state.board().get(to.0, to.1);
                let is_capture = target_piece.is_some();
                if !is_valid_piece_move(game_state, piece_type, from, to, is_capture, None, active_color) {
                    continue;
                }
                if piece_type == PieceType::King
                    || try_move_and_check_legality(game_state, from, to, None, active_color)
                {
                    result.push((from, to, None));
                }
            }
        }
    }

    result
}

/// Generate only capturing moves (including en passant) and promotions.
pub fn find_all_capture_moves(
    game_state: &mut GameState,
) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    let mut result: Vec<((usize, usize), (usize, usize), Option<char>)> = Vec::new();
    let active_color = game_state.active_color();
    let en_passant_target = game_state.en_passant_target();
    let mut targets: Vec<(usize, usize)> = Vec::with_capacity(32);

    for r in 0..8 {
        for c in 0..8 {
            let piece = match game_state.board().get(r, c) {
                Some(p) => p,
                None => continue,
            };
            if piece.get_color() != active_color {
                continue;
            }

            let from = (r, c);
            let piece_type = piece.get_type();
            generate_piece_targets_into(piece_type, from, active_color, &mut targets);

            for to in targets.iter().copied() {
                let is_ep = piece_type == PieceType::Pawn
                    && en_passant_target.is_some()
                    && to == en_passant_target.unwrap()
                    && game_state.board().get(to.0, to.1).is_none();
                let target_piece = game_state.board().get(to.0, to.1);
                let is_capture = target_piece.is_some() || is_ep;
                let is_pawn_promotion = piece_type == PieceType::Pawn && is_promotion(to, active_color);

                if !is_capture && !is_pawn_promotion {
                    continue;
                }

                if !game_state.move_from_and_to_validation_check(
                    from,
                    to,
                    is_capture,
                    piece_type == PieceType::Pawn,
                ) {
                    continue;
                }

                if is_pawn_promotion {
                    let promo_chars = ['q', 'r', 'b', 'n'];
                    for &pc in promo_chars.iter() {
                        let promo_piece = Some(Piece::new(promo_char_to_piece_type(pc), active_color));
                        if !is_valid_piece_move(game_state, piece_type, from, to, is_capture, promo_piece, active_color) {
                            continue;
                        }
                        if piece_type == PieceType::King
                            || try_move_and_check_legality(game_state, from, to, Some(pc), active_color)
                        {
                            result.push((from, to, Some(pc)));
                        }
                    }
                } else {
                    if !is_valid_piece_move(game_state, piece_type, from, to, is_capture, None, active_color) {
                        continue;
                    }
                    if piece_type == PieceType::King
                        || try_move_and_check_legality(game_state, from, to, None, active_color)
                    {
                        result.push((from, to, None));
                    }
                }
            }
        }
    }
    result
}

// Lightweight move for perft: includes capture flag and promotion marker.
#[derive(Clone, Copy, Debug)]
pub struct PerftMove {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub is_capture: bool,
    pub promo: Option<char>,
}

/// Fill `out` with all legal moves for the active side, including capture flag and promotion marker.
/// This mirrors `find_all_valid_moves` but avoids allocating a new Vec every call and returns flags.
pub fn find_all_valid_moves_into_perft(game_state: &mut GameState, out: &mut Vec<PerftMove>) {
    out.clear();
    let active_color = game_state.active_color();
    let en_passant_target = game_state.en_passant_target();
    let mut targets: Vec<(usize, usize)> = Vec::with_capacity(32);

    for r in 0..8 {
        for c in 0..8 {
            let piece = match game_state.board().get(r, c) { Some(p) => p, None => continue };
            if piece.get_color() != active_color { continue; }

            let from = (r, c);
            let piece_type = piece.get_type();

            // Generate piece-specific target squares instead of checking all 64 squares
            generate_piece_targets_into(piece_type, from, active_color, &mut targets);

            for to in targets.iter().copied() {
                let target_piece_is_some = game_state.board().get(to.0, to.1).is_some();
                let is_capture = target_piece_is_some
                    || (piece_type == PieceType::Pawn
                        && en_passant_target.is_some()
                        && to == en_passant_target.unwrap());
                let is_pawn_move = piece_type == PieceType::Pawn;
                if !game_state.move_from_and_to_validation_check(
                    from, to, is_capture, is_pawn_move,
                ) { continue; }

                let is_pawn_promotion = piece_type == PieceType::Pawn && is_promotion(to, active_color);

                if is_pawn_promotion {
                    // Generate all 4 promotion options and filter by legality
                    let promo_types = [
                        PieceType::Queen,
                        PieceType::Rook,
                        PieceType::Bishop,
                        PieceType::Knight,
                    ];
                    for pt in promo_types.iter() {
                        let promo_piece = Some(Piece::new(*pt, active_color));
                        if !is_valid_piece_move(game_state, piece_type, from, to, is_capture, promo_piece, active_color) {
                            continue;
                        }
                        if piece_type == PieceType::King
                            || try_move_and_check_legality(game_state, from, to, piece_type_to_promo_char(*pt), active_color)
                        {
                            out.push(PerftMove {
                                from,
                                to,
                                is_capture,
                                promo: piece_type_to_promo_char(*pt)
                            });
                        }
                    }
                } else {
                    // Regular move - check legality
                    if !is_valid_piece_move(game_state, piece_type, from, to, is_capture, None, active_color) {
                        continue;
                    }
                    if piece_type == PieceType::King
                        || try_move_and_check_legality(game_state, from, to, None, active_color)
                    {
                        out.push(PerftMove { from, to, is_capture, promo: None });
                    }
                }
            }
        }
    }
}

/// Dump all legal moves for the given side from the current board, formatted as SAN or coordinate pairs.
/// This is intended for debugging/tests. It returns a single string with moves separated by spaces.
/// By default it uses simple coordinate notation like e2e4; when `to_san` is true and a GameState is
/// provided, it will attempt to convert to SAN.
pub fn _dump_all_valid_moves(
    game_state: &GameState,
    to_san: bool,
) {
    use crate::board::san_move::convert_move_to_san;
    let mut gs = *game_state;
    let moves = find_all_valid_moves(&mut gs);
    if moves.is_empty() {
        println!("No moves");
        return;
    }
    if to_san {
        let mut parts: Vec<String> = Vec::with_capacity(moves.len());
        for (from, to, promo) in moves {
            if let Some(s) = convert_move_to_san(game_state, Some((from, to, promo))) {
                parts.push(s);
            } else {
                // fallback to coord if SAN conversion fails
                let s = format!(
                    "{}{}{}{}{}",
                    (b'a' + from.1 as u8) as char,
                    (b'1' + from.0 as u8) as char,
                    (b'a' + to.1 as u8) as char,
                    (b'1' + to.0 as u8) as char,
                    promo.unwrap_or('\0')
                );
                parts.push(s.trim_end_matches('\0').to_string());
            }
        }
        println!("{}", parts.join(" "));
    } else {
        let mut parts: Vec<String> = Vec::with_capacity(moves.len());
        for (from, to, promo) in moves {
            let s = if let Some(pc) = promo {
                format!(
                    "{}{}{}{}{}",
                    (b'a' + from.1 as u8) as char,
                    (b'1' + from.0 as u8) as char,
                    (b'a' + to.1 as u8) as char,
                    (b'1' + to.0 as u8) as char,
                    pc
                )
            } else {
                format!(
                    "{}{}{}{}",
                    (b'a' + from.1 as u8) as char,
                    (b'1' + from.0 as u8) as char,
                    (b'a' + to.1 as u8) as char,
                    (b'1' + to.0 as u8) as char
                )
            };
            parts.push(s);
        }
        println!("{}", parts.join(" "));
    }
}
