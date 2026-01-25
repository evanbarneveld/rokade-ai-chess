use chess::history::history::History;
use chess::state::fen::reader::reset_from_fen;
use chess::state::outcome::{recompute_outcome, OutcomeType};

#[test]
fn recompute_outcome_detects_checkmate() {
    let mut gs = reset_from_fen("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1")
        .expect("Invalid FEN");
    let history = History::new();
    let outcome = recompute_outcome(&mut gs, &history);
    assert_eq!(outcome, OutcomeType::Checkmate { winner: chess::piece::pieces::Color::White });
}

#[test]
fn recompute_outcome_detects_stalemate() {
    let mut gs = reset_from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1")
        .expect("Invalid FEN");
    let history = History::new();
    let outcome = recompute_outcome(&mut gs, &history);
    assert_eq!(outcome, OutcomeType::Stalemate);
}

#[test]
fn recompute_outcome_detects_fifty_move_rule() {
    let mut gs = reset_from_fen("8/8/8/8/8/8/6k1/5K1P w - - 0 1")
        .expect("Invalid FEN");
    gs.set_half_move_clock(100);
    let history = History::new();
    let outcome = recompute_outcome(&mut gs, &history);
    assert_eq!(outcome, OutcomeType::DrawByFiftyMoveRule);
}
