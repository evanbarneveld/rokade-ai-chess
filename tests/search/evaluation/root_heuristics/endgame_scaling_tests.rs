use chess::piece::pieces::Color;
use chess::search::test_support::endgame_50move_scaling;

#[test]
fn endgame_scaling_penalizes_non_captures_when_draw_near() {
    let delta = endgame_50move_scaling(Color::White, 500, 90, false, false);
    assert!(delta < 0);
}

#[test]
fn endgame_scaling_rewards_captures_when_draw_near() {
    let delta = endgame_50move_scaling(Color::White, 500, 90, true, false);
    assert!(delta > 0);
}
