use chess::piece::pieces::Color;
use chess::search::test_support::SearchHeuristics;

#[test]
fn killer_moves_rotate_slots() {
    let mut heur = SearchHeuristics::new(8);
    heur.add_killer(0, (0, 0), (0, 1));
    heur.add_killer(0, (1, 0), (1, 1));

    assert!(heur.is_killer(0, (0, 0), (0, 1)));
    assert!(heur.is_killer(0, (1, 0), (1, 1)));

    heur.add_killer(0, (2, 0), (2, 1));
    assert!(heur.is_killer(0, (2, 0), (2, 1)));
    assert!(heur.is_killer(0, (1, 0), (1, 1)));
    assert!(!heur.is_killer(0, (0, 0), (0, 1)));
}

#[test]
fn history_scores_track_sides_independently() {
    let mut heur = SearchHeuristics::new(8);
    heur.add_history(Color::White, (0, 0), (0, 1), 30);
    heur.add_history(Color::Black, (0, 0), (0, 1), 20);

    assert_eq!(heur.history_score(Color::White, (0, 0), (0, 1)), 30);
    assert_eq!(heur.history_score(Color::Black, (0, 0), (0, 1)), 20);
}

#[test]
fn history_scores_cap_at_limit() {
    let mut heur = SearchHeuristics::new(8);
    heur.add_history(Color::White, (0, 0), (0, 1), 10_000_000);
    heur.add_history(Color::Black, (0, 0), (0, 1), -10_000_000);

    let white = heur.history_score(Color::White, (0, 0), (0, 1));
    let black = heur.history_score(Color::Black, (0, 0), (0, 1));

    assert!(white <= 1_000_000);
    assert!(black >= -1_000_000);
    assert!(white > 0);
    assert!(black < 0);
}

#[test]
fn counter_moves_track_prev_move_per_side() {
    let mut heur = SearchHeuristics::new(8);
    let prev_from = (1, 1);
    let prev_to = (2, 2);
    let reply_from = (0, 0);
    let reply_to = (0, 1);

    heur.set_counter_move(Color::White, prev_from, prev_to, reply_from, reply_to);

    assert!(heur.is_counter_move(Color::White, prev_from, prev_to, reply_from, reply_to));
    assert!(!heur.is_counter_move(Color::Black, prev_from, prev_to, reply_from, reply_to));
    assert!(!heur.is_counter_move(Color::White, (1, 2), prev_to, reply_from, reply_to));
}

#[test]
fn continuation_history_tracks_prev_to_square() {
    let mut heur = SearchHeuristics::new(8);
    let prev_to = (4, 4);
    let from = (1, 1);
    let to = (2, 2);

    heur.add_continuation_history(Color::White, prev_to, from, to, 120);

    assert_eq!(heur.continuation_score(Color::White, prev_to, from, to), 120);
    assert_eq!(heur.continuation_score(Color::White, (4, 5), from, to), 0);
}
