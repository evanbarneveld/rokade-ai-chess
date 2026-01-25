use chess::board::Board;
use chess::board::test_support::{
    development_penalty_on_backrank,
    evaluate_king_shelter_patterns,
    is_king_in_front_of_pawn,
    king_ring_pressure,
    king_safety,
    pawn_file_counts,
    build_attack_maps,
};
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn is_king_in_front_of_pawn_detects_blocking() {
    let king = (4, 4);
    let pawn_r = 3;
    let pawn_c = 4;
    assert!(is_king_in_front_of_pawn(king, pawn_r, pawn_c, Color::White));
}

#[test]
fn development_penalty_on_backrank_applies_with_minors_home() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(0, 1, Some(Piece::new(PieceType::Knight, Color::White)));
    board.set(0, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    board.find_and_set_location_of_kings();

    let penalty = development_penalty_on_backrank(&board, Color::White, 24);
    assert!(penalty < 0);
}

#[test]
fn evaluate_king_shelter_patterns_penalizes_missing_pawns() {
    let mut board = Board::empty();
    board.set(0, 6, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(1, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.set(1, 7, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.find_and_set_location_of_kings();

    let penalty = evaluate_king_shelter_patterns(&board, Color::White, 24, Some((0, 6)));
    assert!(penalty < 0);
}

#[test]
fn king_safety_penalizes_open_file_pressure() {
    let mut open = Board::empty();
    open.set(0, 6, Some(Piece::new(PieceType::King, Color::White)));
    open.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    open.set(7, 6, Some(Piece::new(PieceType::Rook, Color::Black)));
    open.find_and_set_location_of_kings();

    let counts = pawn_file_counts(&open);
    let open_score = king_safety(&open, Color::White, 24, Some((0, 6)), &counts);

    let mut closed = open;
    closed.set(1, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    let counts_closed = pawn_file_counts(&closed);
    let closed_score = king_safety(&closed, Color::White, 24, Some((0, 6)), &counts_closed);

    assert!(open_score < closed_score);
}

#[test]
fn king_safety_scales_down_without_enemy_queen() {
    let mut with_queen = Board::empty();
    with_queen.set(0, 6, Some(Piece::new(PieceType::King, Color::White)));
    with_queen.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    with_queen.set(7, 6, Some(Piece::new(PieceType::Rook, Color::Black)));
    with_queen.set(7, 0, Some(Piece::new(PieceType::Queen, Color::Black)));
    with_queen.find_and_set_location_of_kings();

    let counts = pawn_file_counts(&with_queen);
    let with_queen_score = king_safety(&with_queen, Color::White, 24, Some((0, 6)), &counts);

    let mut no_queen = with_queen;
    no_queen.set(7, 0, None);
    let counts_no = pawn_file_counts(&no_queen);
    let no_queen_score = king_safety(&no_queen, Color::White, 24, Some((0, 6)), &counts_no);

    assert!(with_queen_score < no_queen_score);
}

#[test]
fn king_safety_prefers_home_pawn_shield() {
    let mut home = Board::empty();
    home.set(0, 6, Some(Piece::new(PieceType::King, Color::White)));
    home.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    home.set(1, 5, Some(Piece::new(PieceType::Pawn, Color::White)));
    home.set(1, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    home.set(1, 7, Some(Piece::new(PieceType::Pawn, Color::White)));
    home.find_and_set_location_of_kings();

    let counts_home = pawn_file_counts(&home);
    let home_score = king_safety(&home, Color::White, 24, Some((0, 6)), &counts_home);

    let mut advanced = home;
    advanced.set(1, 6, None);
    advanced.set(2, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    let counts_adv = pawn_file_counts(&advanced);
    let adv_score = king_safety(&advanced, Color::White, 24, Some((0, 6)), &counts_adv);

    assert!(home_score > adv_score);
}

#[test]
fn king_ring_pressure_penalizes_direct_check_lines() {
    let mut open = Board::empty();
    open.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    open.set(7, 7, Some(Piece::new(PieceType::King, Color::Black)));
    open.set(7, 4, Some(Piece::new(PieceType::Rook, Color::Black)));
    open.find_and_set_location_of_kings();
    let (att_w, att_b) = build_attack_maps(&open);
    let open_score = king_ring_pressure(&open, Color::White, 24, Some((0, 4)), &att_w, &att_b);

    let mut blocked = open;
    blocked.set(1, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    let (att_w_blocked, att_b_blocked) = build_attack_maps(&blocked);
    let blocked_score = king_ring_pressure(
        &blocked, Color::White, 24, Some((0, 4)), &att_w_blocked, &att_b_blocked
    );

    assert!(open_score < blocked_score);
}

#[test]
fn king_ring_pressure_rewards_non_king_defenders() {
    let mut unsafe_board = Board::empty();
    unsafe_board.set(0, 6, Some(Piece::new(PieceType::King, Color::White)));
    unsafe_board.set(7, 7, Some(Piece::new(PieceType::King, Color::Black)));
    unsafe_board.set(4, 2, Some(Piece::new(PieceType::Bishop, Color::Black)));
    unsafe_board.find_and_set_location_of_kings();
    let (att_w, att_b) = build_attack_maps(&unsafe_board);
    let unsafe_score = king_ring_pressure(
        &unsafe_board, Color::White, 24, Some((0, 6)), &att_w, &att_b
    );

    let mut defended_board = unsafe_board;
    defended_board.set(0, 7, Some(Piece::new(PieceType::Knight, Color::White)));
    let (att_w_def, att_b_def) = build_attack_maps(&defended_board);
    let defended_score = king_ring_pressure(
        &defended_board, Color::White, 24, Some((0, 6)), &att_w_def, &att_b_def
    );

    assert!(defended_score > unsafe_score);
}
