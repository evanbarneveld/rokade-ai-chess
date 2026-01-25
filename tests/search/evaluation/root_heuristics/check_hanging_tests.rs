use chess::search::test_support::check_mobility_bonus_for_side;
use chess::state::fen::reader::reset_from_fen;
use chess::piece::pieces::Color;

#[test]
fn check_mobility_bonus_rewards_forcing_checks() {
    let gs = reset_from_fen("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1")
        .expect("Invalid FEN");
    let bonus = check_mobility_bonus_for_side(gs.board(), Color::Black);
    assert!(bonus > 0);
}
