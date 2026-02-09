use chess::board::Board;
use chess::board::test_support::{
    build_attack_maps,
    evaluate_pawn,
    has_clear_promotion_path,
    is_passed_pawn,
    pawn_majority_bonus,
    pawn_file_counts,
    evaluate_pawn_islands,
};
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn is_passed_pawn_detects_clear_file() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(3, 3, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.find_and_set_location_of_kings();

    assert!(is_passed_pawn(&board, 3, 3, Color::White));

    board.set(5, 3, Some(Piece::new(PieceType::Pawn, Color::Black)));
    assert!(!is_passed_pawn(&board, 3, 3, Color::White));
}

#[test]
fn has_clear_promotion_path_respects_blockers() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(6, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.find_and_set_location_of_kings();

    assert!(has_clear_promotion_path(&board, 6, 0, Color::White));

    board.set(7, 0, Some(Piece::new(PieceType::Knight, Color::Black)));
    assert!(!has_clear_promotion_path(&board, 6, 0, Color::White));
}

#[test]
fn evaluate_pawn_islands_penalizes_multiple_islands() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(1, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.set(1, 2, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.find_and_set_location_of_kings();

    let counts = pawn_file_counts(&board);
    let penalty = evaluate_pawn_islands(&counts, Color::White, 24);
    assert!(penalty < 0);
}

#[test]
fn evaluate_passed_pawn_connected_bonus_increases_score() {
    let mut base = Board::empty();
    base.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    base.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    base.set(4, 2, Some(Piece::new(PieceType::Pawn, Color::White)));
    base.find_and_set_location_of_kings();

    let (att_w, att_b) = build_attack_maps(&base);
    let counts = pawn_file_counts(&base);
    let base_score = evaluate_pawn(
        &base,
        4,
        2,
        Color::White,
        24,
        Some(base.get_king_location(Color::White)),
        Some(base.get_king_location(Color::Black)),
        &att_w,
        &att_b,
        &counts,
    );

    let mut connected = base;
    connected.set(4, 3, Some(Piece::new(PieceType::Pawn, Color::White)));
    let (att_w2, att_b2) = build_attack_maps(&connected);
    let counts2 = pawn_file_counts(&connected);
    let connected_score = evaluate_pawn(
        &connected,
        4,
        2,
        Color::White,
        24,
        Some(connected.get_king_location(Color::White)),
        Some(connected.get_king_location(Color::Black)),
        &att_w2,
        &att_b2,
        &counts2,
    );

    assert!(connected_score > base_score);
}

#[test]
fn evaluate_passed_pawn_blockade_penalty_applies() {
    let mut free = Board::empty();
    free.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    free.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    free.set(5, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    free.find_and_set_location_of_kings();

    let (att_w, att_b) = build_attack_maps(&free);
    let counts = pawn_file_counts(&free);
    let free_score = evaluate_pawn(
        &free,
        5,
        4,
        Color::White,
        12,
        Some(free.get_king_location(Color::White)),
        Some(free.get_king_location(Color::Black)),
        &att_w,
        &att_b,
        &counts,
    );

    let mut blocked = free;
    blocked.set(6, 4, Some(Piece::new(PieceType::Knight, Color::Black)));
    let (att_w2, att_b2) = build_attack_maps(&blocked);
    let counts2 = pawn_file_counts(&blocked);
    let blocked_score = evaluate_pawn(
        &blocked,
        5,
        4,
        Color::White,
        12,
        Some(blocked.get_king_location(Color::White)),
        Some(blocked.get_king_location(Color::Black)),
        &att_w2,
        &att_b2,
        &counts2,
    );

    assert!(blocked_score < free_score);
}

#[test]
fn pawn_majority_bonus_rewards_wing_majority() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(1, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.set(1, 1, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.set(1, 2, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.set(6, 0, Some(Piece::new(PieceType::Pawn, Color::Black)));
    board.find_and_set_location_of_kings();

    let counts = pawn_file_counts(&board);
    let bonus = pawn_majority_bonus(&counts, Color::White, 0);
    assert!(bonus > 0);
}

#[test]
fn tarrasch_rule_rewards_rook_behind_passed_pawn() {
    // White passed pawn with white rook behind it should score higher
    let mut rook_behind = Board::empty();
    rook_behind.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    rook_behind.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    rook_behind.set(5, 0, Some(Piece::new(PieceType::Pawn, Color::White)));  // a6 passed pawn
    rook_behind.set(2, 0, Some(Piece::new(PieceType::Rook, Color::White)));  // Rook behind on a3
    rook_behind.find_and_set_location_of_kings();

    let (att_w, att_b) = build_attack_maps(&rook_behind);
    let counts = pawn_file_counts(&rook_behind);
    let behind_score = evaluate_pawn(
        &rook_behind,
        5,
        0,
        Color::White,
        0,  // Endgame phase (Tarrasch matters most in endgame)
        Some(rook_behind.get_king_location(Color::White)),
        Some(rook_behind.get_king_location(Color::Black)),
        &att_w,
        &att_b,
        &counts,
    );

    let mut rook_away = Board::empty();
    rook_away.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    rook_away.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    rook_away.set(5, 0, Some(Piece::new(PieceType::Pawn, Color::White)));  // a6 passed pawn
    rook_away.set(2, 7, Some(Piece::new(PieceType::Rook, Color::White)));  // Rook on different file
    rook_away.find_and_set_location_of_kings();

    let (att_w2, att_b2) = build_attack_maps(&rook_away);
    let counts2 = pawn_file_counts(&rook_away);
    let away_score = evaluate_pawn(
        &rook_away,
        5,
        0,
        Color::White,
        0,
        Some(rook_away.get_king_location(Color::White)),
        Some(rook_away.get_king_location(Color::Black)),
        &att_w2,
        &att_b2,
        &counts2,
    );

    assert!(behind_score > away_score);
}

#[test]
fn tarrasch_rule_penalizes_enemy_rook_behind_passer() {
    // White passed pawn with black rook behind it should score lower
    let mut enemy_behind = Board::empty();
    enemy_behind.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    enemy_behind.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    enemy_behind.set(5, 0, Some(Piece::new(PieceType::Pawn, Color::White)));  // a6 passed pawn
    enemy_behind.set(2, 0, Some(Piece::new(PieceType::Rook, Color::Black)));  // Enemy rook behind on a3
    enemy_behind.find_and_set_location_of_kings();

    let (att_w, att_b) = build_attack_maps(&enemy_behind);
    let counts = pawn_file_counts(&enemy_behind);
    let enemy_score = evaluate_pawn(
        &enemy_behind,
        5,
        0,
        Color::White,
        0,
        Some(enemy_behind.get_king_location(Color::White)),
        Some(enemy_behind.get_king_location(Color::Black)),
        &att_w,
        &att_b,
        &counts,
    );

    let mut no_rook = Board::empty();
    no_rook.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    no_rook.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    no_rook.set(5, 0, Some(Piece::new(PieceType::Pawn, Color::White)));  // a6 passed pawn
    no_rook.find_and_set_location_of_kings();

    let (att_w2, att_b2) = build_attack_maps(&no_rook);
    let counts2 = pawn_file_counts(&no_rook);
    let no_rook_score = evaluate_pawn(
        &no_rook,
        5,
        0,
        Color::White,
        0,
        Some(no_rook.get_king_location(Color::White)),
        Some(no_rook.get_king_location(Color::Black)),
        &att_w2,
        &att_b2,
        &counts2,
    );

    assert!(enemy_score < no_rook_score);
}

#[test]
fn king_distance_ratio_favors_closer_friendly_king() {
    // Passed pawn with friendly king closer should score higher
    let mut friendly_close = Board::empty();
    friendly_close.set(4, 0, Some(Piece::new(PieceType::King, Color::White)));  // King close to pawn
    friendly_close.set(7, 7, Some(Piece::new(PieceType::King, Color::Black)));  // Enemy king far
    friendly_close.set(5, 1, Some(Piece::new(PieceType::Pawn, Color::White)));  // b6 passed pawn
    friendly_close.find_and_set_location_of_kings();

    let (att_w, att_b) = build_attack_maps(&friendly_close);
    let counts = pawn_file_counts(&friendly_close);
    let close_score = evaluate_pawn(
        &friendly_close,
        5,
        1,
        Color::White,
        0,  // Endgame
        Some(friendly_close.get_king_location(Color::White)),
        Some(friendly_close.get_king_location(Color::Black)),
        &att_w,
        &att_b,
        &counts,
    );

    let mut enemy_close = Board::empty();
    enemy_close.set(0, 7, Some(Piece::new(PieceType::King, Color::White)));  // Friendly king far
    enemy_close.set(6, 1, Some(Piece::new(PieceType::King, Color::Black)));  // Enemy king close
    enemy_close.set(5, 1, Some(Piece::new(PieceType::Pawn, Color::White)));  // b6 passed pawn
    enemy_close.find_and_set_location_of_kings();

    let (att_w2, att_b2) = build_attack_maps(&enemy_close);
    let counts2 = pawn_file_counts(&enemy_close);
    let far_score = evaluate_pawn(
        &enemy_close,
        5,
        1,
        Color::White,
        0,
        Some(enemy_close.get_king_location(Color::White)),
        Some(enemy_close.get_king_location(Color::Black)),
        &att_w2,
        &att_b2,
        &counts2,
    );

    assert!(close_score > far_score);
}
