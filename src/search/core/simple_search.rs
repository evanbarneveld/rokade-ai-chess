use crate::board::evaluator::{evaluate_position, MATE_VALUE};
use crate::history::history::History;
use crate::piece::piece_mover::PieceMover;
use crate::piece::pieces::{Color, Piece, PieceType};
use crate::search::advanced_search::find_all_valid_moves;
use crate::search::context::SearchContext;
use crate::search::Search;
use crate::state::game_state::GameState;

/// A minimal `Search` implementation using minimax depth-limited search.
///
/// Uses copy-based approach: GameState is Copy so cloning is just a memcpy.
/// This is efficient because it avoids the overhead of make_move_fast which
/// recomputes Zobrist hashes (unnecessary for simple search without TT).
pub struct SimpleSearch;

impl Search for SimpleSearch {
    fn find_best_move(
        _ctx: &SearchContext,
        game_state: &GameState,
        _history: &History,
        search_depth: usize,
        _playing_strength: usize,
    ) -> Option<((usize, usize), (usize, usize), Option<char>, i32, usize)> {

        let depth_limit: usize = search_depth;

        // Generate all legal root moves
        let mut gs_root = *game_state;
        let moves = find_all_valid_moves(&mut gs_root);
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
                || promo.is_none()
                    && gs.board().get(from.0, from.1).map(|p| p.get_type()) == Some(PieceType::Pawn)
                    && gs.en_passant_target().is_some()
                    && Some(to) == gs.en_passant_target();

            let promotion_piece: Option<Piece> = match promo {
                Some('q') => Some(Piece::new(PieceType::Queen, root_side)),
                Some('r') => Some(Piece::new(PieceType::Rook, root_side)),
                Some('b') => Some(Piece::new(PieceType::Bishop, root_side)),
                Some('n') => Some(Piece::new(PieceType::Knight, root_side)),
                _ => None,
            };

            if !PieceMover::move_piece(&mut gs, from, to, is_capture, promotion_piece) {
                continue;
            }
            gs.switch_player_turn();

            let score = minimax(&gs, depth_limit - 1, root_side);

            match best {
                None => best = Some((from, to, promo, score)),
                Some((_bf, _bt, _bp, sc)) if score > sc => best = Some((from, to, promo, score)),
                _ => {}
            }
        }

        best.map(|(bf, bt, bp, sc)| (bf, bt, bp, sc, depth_limit))
    }
}

fn minimax(state: &GameState, depth: usize, root_side: Color) -> i32 {
    if depth == 0 {
        // Evaluate from White's perspective, then convert to root-side perspective.
        let white_centric = evaluate_position(state.board(), state.active_color());
        return if root_side == Color::White { white_centric } else { -white_centric };
    }

    let mut state_copy = *state;
    let moves = find_all_valid_moves(&mut state_copy);

    if moves.is_empty() {
        // No legal moves: checkmate or stalemate. Score from root perspective.
        let side_to_move = state.active_color();
        let in_check = state_copy.mutable_board().is_side_in_check(side_to_move);
        if in_check {
            // Checkmate: side to move loses
            // Use depth to prefer faster mates: higher depth = closer to root = faster mate
            // When mating opponent: higher score is better, so add depth
            // When being mated: lower (more negative) score is worse, so subtract depth
            return if side_to_move == root_side {
                -MATE_VALUE - depth as i32  // Being mated sooner is worse
            } else {
                MATE_VALUE + depth as i32   // Mating sooner is better
            };
        } else {
            // Stalemate: draw
            return 0;
        }
    }

    let maximizing = state.active_color() == root_side;

    if maximizing {
        let mut best = i32::MIN;
        for (from, to, promo) in moves {
            let mut gs = state_copy;
            let is_capture = gs.board().get(to.0, to.1).is_some()
                || promo.is_none()
                    && gs.board().get(from.0, from.1).map(|p| p.get_type()) == Some(PieceType::Pawn)
                    && gs.en_passant_target().is_some()
                    && Some(to) == gs.en_passant_target();
            let promotion_piece: Option<Piece> = match promo {
                Some('q') => Some(Piece::new(PieceType::Queen, state.active_color())),
                Some('r') => Some(Piece::new(PieceType::Rook, state.active_color())),
                Some('b') => Some(Piece::new(PieceType::Bishop, state.active_color())),
                Some('n') => Some(Piece::new(PieceType::Knight, state.active_color())),
                _ => None,
            };
            if PieceMover::move_piece(&mut gs, from, to, is_capture, promotion_piece) {
                gs.switch_player_turn();
                let val = minimax(&gs, depth - 1, root_side);
                if val > best {
                    best = val;
                }
            }
        }
        best
    } else {
        let mut best = i32::MAX;
        for (from, to, promo) in moves {
            let mut gs = state_copy;
            let is_capture = gs.board().get(to.0, to.1).is_some()
                || promo.is_none()
                    && gs.board().get(from.0, from.1).map(|p| p.get_type()) == Some(PieceType::Pawn)
                    && gs.en_passant_target().is_some()
                    && Some(to) == gs.en_passant_target();
            let promotion_piece: Option<Piece> = match promo {
                Some('q') => Some(Piece::new(PieceType::Queen, state.active_color())),
                Some('r') => Some(Piece::new(PieceType::Rook, state.active_color())),
                Some('b') => Some(Piece::new(PieceType::Bishop, state.active_color())),
                Some('n') => Some(Piece::new(PieceType::Knight, state.active_color())),
                _ => None,
            };
            if PieceMover::move_piece(&mut gs, from, to, is_capture, promotion_piece) {
                gs.switch_player_turn();
                let val = minimax(&gs, depth - 1, root_side);
                if val < best {
                    best = val;
                }
            }
        }
        best
    }
}
