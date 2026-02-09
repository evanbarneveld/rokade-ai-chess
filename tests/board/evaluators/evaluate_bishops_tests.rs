use chess::board::Board;
use chess::board::test_support::{evaluate_bishop, is_bishop_outpost};
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn evaluate_bishop_rewards_developed_bishops() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(3, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    board.find_and_set_location_of_kings();

    let score = evaluate_bishop(&board, 3, 2, Color::White, 24);
    assert!(score > 0);
}

#[test]
fn evaluate_bishop_does_not_reward_home_bishops() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(0, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    board.find_and_set_location_of_kings();

    let score = evaluate_bishop(&board, 0, 2, Color::White, 24);
    assert_eq!(score, 0);
}

#[test]
fn evaluate_bishop_penalizes_bad_bishop_pawn_color() {
    let mut bad = Board::empty();
    bad.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    bad.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    bad.set(2, 4, Some(Piece::new(PieceType::Bishop, Color::White)));
    bad.set(2, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    bad.set(2, 2, Some(Piece::new(PieceType::Pawn, Color::White)));
    bad.set(2, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    bad.set(4, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    bad.find_and_set_location_of_kings();

    let mut good = Board::empty();
    good.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    good.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    good.set(2, 4, Some(Piece::new(PieceType::Bishop, Color::White)));
    good.set(1, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    good.set(1, 2, Some(Piece::new(PieceType::Pawn, Color::White)));
    good.set(1, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    good.set(1, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    good.find_and_set_location_of_kings();

    let bad_score = evaluate_bishop(&bad, 2, 4, Color::White, 24);
    let good_score = evaluate_bishop(&good, 2, 4, Color::White, 24);

    assert!(bad_score < good_score);
}

#[test]
fn evaluate_bishop_rewards_mobility() {
    let mut open = Board::empty();
    open.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    open.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    open.set(3, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    open.set(3, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    open.set(1, 2, Some(Piece::new(PieceType::Pawn, Color::White)));
    open.set(3, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    open.set(1, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    open.find_and_set_location_of_kings();

    let mut blocked = Board::empty();
    blocked.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    blocked.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    blocked.set(3, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    blocked.set(3, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    blocked.set(1, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    blocked.set(2, 1, Some(Piece::new(PieceType::Pawn, Color::White)));
    blocked.set(2, 3, Some(Piece::new(PieceType::Pawn, Color::White)));
    blocked.find_and_set_location_of_kings();

    let open_score = evaluate_bishop(&open, 3, 2, Color::White, 24);
    let blocked_score = evaluate_bishop(&blocked, 3, 2, Color::White, 24);

    assert!(open_score > blocked_score);
}

#[test]
fn is_bishop_outpost_detects_protected_bishop() {
    // White bishop on d5 (row 4, col 3) with protecting pawn on c4 (row 3, col 2)
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(4, 3, Some(Piece::new(PieceType::Bishop, Color::White)));  // d5
    board.set(3, 2, Some(Piece::new(PieceType::Pawn, Color::White)));    // c4 supporting pawn
    board.find_and_set_location_of_kings();

    assert!(is_bishop_outpost(&board, 4, 3, Color::White));
}

#[test]
fn bishop_outpost_not_detected_when_attackable_by_enemy_pawn() {
    // White bishop on d5 but black pawn on e6 can advance and attack d5
    // For outpost, we check if enemy pawns on adjacent files AHEAD can attack
    // Black pawn on e6 (row 5) can advance to e5 and attack d5
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(4, 3, Some(Piece::new(PieceType::Bishop, Color::White)));  // d5
    board.set(3, 2, Some(Piece::new(PieceType::Pawn, Color::White)));    // c4 supporting pawn
    board.set(5, 4, Some(Piece::new(PieceType::Pawn, Color::Black)));    // e6 - can attack d5 by advancing
    board.find_and_set_location_of_kings();

    assert!(!is_bishop_outpost(&board, 4, 3, Color::White));
}

#[test]
fn bishop_outpost_not_detected_without_pawn_protection() {
    // White bishop on d5 but no supporting pawn
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(4, 3, Some(Piece::new(PieceType::Bishop, Color::White)));  // d5
    board.find_and_set_location_of_kings();

    assert!(!is_bishop_outpost(&board, 4, 3, Color::White));
}

#[test]
fn evaluate_bishop_rewards_outpost() {
    // Test that outpost bonus is applied by comparing an outpost vs non-outpost position
    // Both positions have the same supporting pawn, but one has an enemy pawn that can attack
    let mut outpost = Board::empty();
    outpost.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    outpost.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    outpost.set(4, 3, Some(Piece::new(PieceType::Bishop, Color::White)));  // d5
    outpost.set(3, 2, Some(Piece::new(PieceType::Pawn, Color::White)));    // c4 supporting
    // No enemy pawns that can attack d5
    outpost.find_and_set_location_of_kings();

    // Verify it's actually an outpost
    assert!(is_bishop_outpost(&outpost, 4, 3, Color::White));

    // Same position but with enemy pawn on e6 that can attack d5
    let mut not_outpost = Board::empty();
    not_outpost.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    not_outpost.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    not_outpost.set(4, 3, Some(Piece::new(PieceType::Bishop, Color::White)));  // d5
    not_outpost.set(3, 2, Some(Piece::new(PieceType::Pawn, Color::White)));    // c4 supporting
    not_outpost.set(5, 4, Some(Piece::new(PieceType::Pawn, Color::Black)));    // e6 - can attack d5
    not_outpost.find_and_set_location_of_kings();

    // Verify it's NOT an outpost (due to enemy pawn)
    assert!(!is_bishop_outpost(&not_outpost, 4, 3, Color::White));

    let outpost_score = evaluate_bishop(&outpost, 4, 3, Color::White, 24);
    let not_outpost_score = evaluate_bishop(&not_outpost, 4, 3, Color::White, 24);

    // The outpost position should score higher
    // Note: The non-outpost has an extra enemy pawn which might affect mobility slightly,
    // but the outpost bonus should outweigh this
    assert!(outpost_score > not_outpost_score);
}
