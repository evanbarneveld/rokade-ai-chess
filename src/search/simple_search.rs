use crate::board::evaluator::evaluate_position;
use crate::history::history::History;
use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{Color, Piece, PieceType};
use crate::search::advanced_search::find_all_valid_moves;
use crate::search::Search;
use crate::state::game_state::GameState;

/// A minimal `Search` implementation using minimax depth-limited search.
///
pub struct SimpleSearch;

impl Search for SimpleSearch {
    fn find_best_move(
        game_state: &GameState,
        _history: &History,
        search_depth: usize,
        _playing_strength: usize,
    ) -> Option<((usize, usize), (usize, usize), Option<char>, i32, usize)> {

        let depth_limit: usize = search_depth;

        // Generate all legal root moves
        let moves = find_all_valid_moves(game_state);
        if moves.is_empty() {
            return None;
        }

        let root_side = game_state.active_color();

        // Evaluate each move with a simple depth-limited minimax
        let mut best: Option<((usize, usize), (usize, usize), Option<char>, i32)> = None;
        for (from, to, promo) in moves {
            let mut gs = *game_state;

            // Determine capture and promotion piece
            let is_capture = gs.board().get(to.0, to.1).is_some()
                || (matches!(promo, None)
                    && gs.board().get(from.0, from.1).map(|p| p.get_type()) == Some(PieceType::Pawn)
                    && gs.get_en_passant_target().is_some()
                    && Some(to) == gs.get_en_passant_target());

            let promotion_piece: Option<Piece> = match promo {
                Some('q') => Some(Piece::new(PieceType::Queen, root_side)),
                Some('r') => Some(Piece::new(PieceType::Rook, root_side)),
                Some('b') => Some(Piece::new(PieceType::Bishop, root_side)),
                Some('n') => Some(Piece::new(PieceType::Knight, root_side)),
                _ => None,
            };

            if !PieceMover::move_piece(&mut gs, from, to, is_capture, promotion_piece) {
                // Should be rare since moves were pre-validated; skip if it happens
                continue;
            }

            let score = minimax(gs, depth_limit - 1, root_side);

            match best {
                None => best = Some((from, to, promo, score)),
                Some((_bf, _bt, _bp, sc)) if score > sc => best = Some((from, to, promo, score)),
                _ => {}
            }
        }

        best.map(|(bf, bt, bp, sc)| (bf, bt, bp, sc, depth_limit))
    }
}

fn minimax(state: GameState, depth: usize, root_side: Color) -> i32 {
    if depth == 0 {
        // Evaluate from White's perspective, then convert to root-side perspective.
        let white_centric = evaluate_position(state.board(), state.active_color());
        return if root_side == Color::White { white_centric } else { -white_centric };
    }

    let moves = find_all_valid_moves(&state);
    if moves.is_empty() {
        // No legal moves: checkmate or stalemate. Score from root perspective.
        // If side to move is in check -> mate (large loss for side to move).
        // Else stalemate -> draw (0).
        // We don't have a direct is_in_check here; approximate with static eval fallback
        // if no helper is available. Prefer explicit check detection if present.
        let white_centric = evaluate_position(state.board(), state.active_color());
        return if root_side == Color::White { white_centric } else { -white_centric };
    }

    let maximizing = state.active_color() == root_side;

    if maximizing {
        let mut best = i32::MIN;
        for (from, to, promo) in moves {
            let mut gs = state;
            let is_capture = gs.board().get(to.0, to.1).is_some()
                || (matches!(promo, None)
                    && gs.board().get(from.0, from.1).map(|p| p.get_type()) == Some(PieceType::Pawn)
                    && gs.get_en_passant_target().is_some()
                    && Some(to) == gs.get_en_passant_target());
            let promotion_piece: Option<Piece> = match promo {
                Some('q') => Some(Piece::new(PieceType::Queen, state.active_color())),
                Some('r') => Some(Piece::new(PieceType::Rook, state.active_color())),
                Some('b') => Some(Piece::new(PieceType::Bishop, state.active_color())),
                Some('n') => Some(Piece::new(PieceType::Knight, state.active_color())),
                _ => None,
            };
            if PieceMover::move_piece(&mut gs, from, to, is_capture, promotion_piece) {
                let val = minimax(gs, depth - 1, root_side);
                if val > best {
                    best = val;
                }
            }
        }
        best
    } else {
        let mut best = i32::MAX;
        for (from, to, promo) in moves {
            let mut gs = state;
            let is_capture = gs.board().get(to.0, to.1).is_some()
                || (matches!(promo, None)
                    && gs.board().get(from.0, from.1).map(|p| p.get_type()) == Some(PieceType::Pawn)
                    && gs.get_en_passant_target().is_some()
                    && Some(to) == gs.get_en_passant_target());
            let promotion_piece: Option<Piece> = match promo {
                Some('q') => Some(Piece::new(PieceType::Queen, state.active_color())),
                Some('r') => Some(Piece::new(PieceType::Rook, state.active_color())),
                Some('b') => Some(Piece::new(PieceType::Bishop, state.active_color())),
                Some('n') => Some(Piece::new(PieceType::Knight, state.active_color())),
                _ => None,
            };
            if PieceMover::move_piece(&mut gs, from, to, is_capture, promotion_piece) {
                let val = minimax(gs, depth - 1, root_side);
                if val < best {
                    best = val;
                }
            }
        }
        best
    }
}
