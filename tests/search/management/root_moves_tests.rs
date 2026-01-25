use chess::board::Board;
use chess::piece::pieces::Color;
use chess::search::context::SearchContext;
use chess::search::test_support::{adjusted_root_eval_for_move, build_pv_for_root, root_move_bonus};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn root_move_bonus_rewards_development_moves() {
    let board = Board::new();
    let bonus = root_move_bonus(&board, (0, 1), (2, 2), Color::White);
    assert_eq!(bonus, 5);

    let bonus_black = root_move_bonus(&board, (7, 1), (5, 2), Color::Black);
    assert_eq!(bonus_black, -5);
}

#[test]
fn adjusted_root_eval_for_move_keeps_mate_scores() {
    let board = Board::new();
    let score = adjusted_root_eval_for_move(
        &board,
        Color::White,
        (1, 4),
        (3, 4),
        None,
        0,
        30000,
        false,
        true,
    );
    assert_eq!(score, 30000);
}

#[test]
fn build_pv_for_root_returns_single_move_without_tt() {
    let gs = reset_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        .expect("Invalid FEN");
    let ctx = SearchContext::new();
    let pv = build_pv_for_root(&gs, (1, 4), (3, 4), None, ctx.tt(), 3);
    assert_eq!(pv.len(), 1);
    assert_eq!(pv[0], ((1, 4), (3, 4), None));
}
