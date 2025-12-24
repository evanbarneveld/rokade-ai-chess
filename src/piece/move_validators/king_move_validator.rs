use crate::board::checks::king_in_check::is_king_in_check_after_move;
use crate::piece::pieces::{Color, PieceType};
use crate::state::game_state::GameState;
use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::piece::as_square_str;

pub fn is_valid_king_move(game_state: &mut GameState, from: (usize, usize), to: (usize, usize)) -> bool {

    if from == to { return false; }

    // standard king move: one square any direction
    let dr = if from.0 > to.0 { from.0 - to.0 } else { to.0 - from.0 };
    let dc = if from.1 > to.1 { from.1 - to.1 } else { to.1 - from.1 };
    if dr <= 1 && dc <= 1 {
        //if this move puts the move in check, its not a valid move
        let active_color = game_state.active_color();
        let en_passant_target = game_state.en_passant_target();
        if is_king_in_check_after_move(game_state.mutable_board(), from, to, en_passant_target) {
            //if is_square_attacked_by_opponent(game_state.mutable_board(), to, active_color) {
            //println!("invalid king move (leads to check): {}", as_square_str(from,to));
            return false;
            //}
        }

        //println!("valid king move: {}", as_square_str(from, to));
        return true;
    }

    // castling: same rank and two files over
    let same_rank = from.0 == to.0;
    let file_diff = if from.1 > to.1 { from.1 - to.1 } else { to.1 - from.1 };
    if !(same_rank && file_diff == 2) { return false; }

    // Determine color and initial positions
    let king_piece = match game_state.board().get(from.0, from.1) { Some(p) => p, None => return false };
    if king_piece.get_type() != PieceType::King { return false; }
    let color = king_piece.get_color();

    let row = from.0;
    let (start_row, start_col) = match color {
        Color::White => (0usize, 4usize),
        Color::Black => (7usize, 4usize),
    };
    if from != (start_row, start_col) { return false; }

    // Target sides and rook positions
    let is_kingside = to.1 == 6;
    let is_queenside = to.1 == 2;
    if !is_kingside && !is_queenside { return false; }

    // Check castling rights from GameState
    let rights = game_state.castling_rights();
    let rights_ok = match color {
        Color::White => if is_kingside { rights.white_kingside() } else { rights.white_queenside() },
        Color::Black => if is_kingside { rights.black_kingside() } else { rights.black_queenside() },
    };
    if !rights_ok { return false; }

    // Rook square and empty path checks
    if is_kingside {
        let rook_col = 7usize;
        // rook must exist and match color
        if !matches!(game_state.board().get(row, rook_col), Some(p) if p.get_type()==PieceType::Rook && p.get_color()==color) { return false; }
        // squares between king and rook must be empty: f (5) and g (6)
        if !game_state.board_square_is_empty((row, 5)) { return false; }
        if !game_state.board_square_is_empty((row, 6)) { return false; }
        // squares not under attack: current (4), f (5), g (6)
        if is_square_attacked_by_opponent(game_state.mutable_board(), (row, 4), color) { return false; }
        if is_square_attacked_by_opponent(game_state.mutable_board(), (row, 5), color) { return false; }
        if is_square_attacked_by_opponent(game_state.mutable_board(), (row, 6), color) { return false; }
        true
    } else {
        // queenside
        let rook_col = 0usize;
        if !matches!(game_state.board().get(row, rook_col), Some(p) if p.get_type()==PieceType::Rook && p.get_color()==color) { return false; }
        // empty: b (1), c (2), d (3)
        if !game_state.board_square_is_empty((row, 1)) { return false; }
        if !game_state.board_square_is_empty((row, 2)) { return false; }
        if !game_state.board_square_is_empty((row, 3)) { return false; }
        // not attacked: e (4), d (3), c (2)
        if is_square_attacked_by_opponent(game_state.mutable_board(), (row, 4), color) { return false; }
        if is_square_attacked_by_opponent(game_state.mutable_board(), (row, 3), color) { return false; }
        if is_square_attacked_by_opponent(game_state.mutable_board(), (row, 2), color) { return false; }
        true
    }
}

