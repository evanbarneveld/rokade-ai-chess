use crate::board::checks::square_attacked::is_square_attacked_by_opponent;
use crate::history::history::History;
use crate::piece::move_validators::bishop_move_validator::is_valid_bishop_move;
use crate::piece::move_validators::king_move_validator::is_valid_king_move;
use crate::piece::move_validators::knight_move_validator::is_valid_knight_move;
use crate::piece::move_validators::pawn_move_validator::is_valid_pawn_move;
use crate::piece::move_validators::queen_move_validator::is_valid_queen_move;
use crate::piece::move_validators::rook_move_validator::is_valid_rook_move;
use crate::piece::pieces::{Color, PieceType};
use crate::state::game_state::GameState;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum OutcomeType {
    Ongoing,
    InCheck,
    Checkmate { winner: Color },
    Stalemate,
    DrawByInsufficientMaterial,
    DrawByFiftyMoveRule,
    DrawByThreefoldRepetition,
}

    pub fn recompute_outcome(game_state: &mut GameState, history: &History) -> OutcomeType {

        // Insufficient material check applies immediately
        if has_insufficient_material(game_state) {
            return OutcomeType::DrawByInsufficientMaterial
        }

        // 50-move rule draw
        if game_state.get_half_move_clock() >= 100 {
            return OutcomeType::DrawByFiftyMoveRule
        }

        // Determine outcome related to check states
        let active_color = game_state.active_color();

        let in_check = is_in_check(game_state, active_color);
        let legal_moves_exist = any_legal_move_exists(game_state, active_color);

        if in_check {
            // Checkmate: no move to get the king out of check
            if !legal_moves_exist {
                let winner = match active_color {
                    Color::White => Color::Black,
                    Color::Black => Color::White
                };
                return OutcomeType::Checkmate { winner };
            }
        }

        // Stalemate: side to move not in check and has no legal move
        if !in_check && !legal_moves_exist {
            return OutcomeType::Stalemate
        }

        if history.current_repetition_count() >= 3 {
            return OutcomeType::DrawByThreefoldRepetition
        };

        if in_check {
            OutcomeType::InCheck
        } else {
            OutcomeType::Ongoing
        }
    }

    fn is_in_check(game_state: &mut GameState, color: Color) -> bool {
        let king_sq = game_state.board().get_king_location(color);
        is_square_attacked_by_opponent(game_state.mutable_board(), king_sq, color)
    }

    fn any_legal_move_exists(game_state : &mut GameState, color: Color) -> bool {
        // iterate all pieces of this color and attempt any legal destination
        for r in 0..8 {
            for c in 0..8 {
                if let Some(p) = game_state.board().get(r, c) {
                    if p.get_color() != color { continue; }
                    let from = (r, c);
                    for tr in 0..8 {
                        for tc in 0..8 {
                            let to = (tr, tc);
                            if from == to { continue; }
                            // basic target occupancy
                            let target = game_state.board().get(tr, tc);
                            let is_capture = target.is_some() && target.unwrap().get_color() != color ||
                                (p.get_type() == PieceType::Pawn && game_state.en_passant_target().is_some() && game_state.en_passant_target().unwrap() == to);

                            if !game_state.move_from_and_to_validation_check(from, to, color, is_capture, p.get_type() == PieceType::Pawn, game_state.en_passant_target()) {
                                continue;
                            }

                            let ep = game_state.en_passant_target(); // copy to avoid borrow conflict
                            let legal = match p.get_type() {
                                PieceType::Pawn => is_valid_pawn_move(game_state.mutable_board(), from, to, is_capture, ep, color, None, true),
                                PieceType::Knight => is_valid_knight_move(game_state.mutable_board(), from, to, true),
                                PieceType::Bishop => is_valid_bishop_move(game_state.mutable_board(), from, to, true),
                                PieceType::Rook => is_valid_rook_move(game_state.mutable_board(), from, to, true),
                                PieceType::Queen => is_valid_queen_move(game_state.mutable_board(), from, to, true),
                                PieceType::King => is_valid_king_move(game_state, from, to),
                            };
                            if legal {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn has_insufficient_material(game_state: &mut GameState) -> bool {
        let mut pawns = 0;
        let mut rooks = 0;
        let mut queens = 0;
        let mut knights = 0;
        let mut bishops: Vec<(usize, usize)> = Vec::new();

        for r in 0..8 {
            for c in 0..8 {
                if let Some(p) = game_state.board().get(r, c) {
                    match p.get_type() {
                        PieceType::Pawn => pawns += 1,
                        PieceType::Rook => rooks += 1,
                        PieceType::Queen => queens += 1,
                        PieceType::Knight => knights += 1,
                        PieceType::Bishop => bishops.push((r, c)),
                        PieceType::King => {}
                    }
                }
            }
        }

        // Any heavy material or pawns means mating material exists
        if pawns > 0 || rooks > 0 || queens > 0 { return false; }

        // King vs King
        if knights == 0 && bishops.is_empty() { return true; }

        // King and single minor vs King
        if (knights == 1 && bishops.is_empty()) || (knights == 0 && bishops.len() == 1) { return true; }

        // King and bishop vs king and bishop, bishops on same color
        if knights == 0 && !bishops.is_empty() {
            let mut color_opt: Option<bool> = None; // true for light, false for dark
            let mut all_same = true;
            for (r, c) in bishops {
                let light = (r + c) % 2 == 0; // a1 (0,0) is light in this board mapping
                if let Some(prev) = color_opt {
                    if prev != light {
                        all_same = false;
                        break;
                    }
                } else { color_opt = Some(light); }
            }
            if all_same { return true; }
        }

        false
    }
