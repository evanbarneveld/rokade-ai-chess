use chess::search::integration::playing_strength::{select_move_based_using_strength, strength_noise_sigma};

#[test]
fn select_move_based_on_strength_is_deterministic_when_requested() {
    let moves = vec![
        ((0, 0), (0, 1), 100),
        ((1, 0), (1, 1), 50),
    ];
    let pick = select_move_based_using_strength(&moves, 500, true).expect("pick");
    assert_eq!(pick, ((0, 0), (0, 1)));
}

#[test]
fn strength_noise_sigma_zero_at_max_strength() {
    assert_eq!(strength_noise_sigma(1000), 0);
    assert!(strength_noise_sigma(1) > 0);
}
