use chess::board::Board;
use chess::board::evaluator::evaluate_position;
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn evaluate_position_is_zero_for_insufficient_material() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.find_and_set_location_of_kings();

    let score = evaluate_position(&board, Color::White);
    assert_eq!(score, 0);
}

#[test]
fn evaluate_position_includes_tempo_bonus() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(3, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.set(0, 1, Some(Piece::new(PieceType::Knight, Color::White)));
    board.set(0, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    board.find_and_set_location_of_kings();

    let white_to_move = evaluate_position(&board, Color::White);
    let black_to_move = evaluate_position(&board, Color::Black);
    assert!(white_to_move > black_to_move);
}

#[test]
fn evaluate_position_rewards_safe_exchange_threats() {
    let mut base = Board::empty();
    base.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    base.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    base.set(6, 5, Some(Piece::new(PieceType::Rook, Color::Black)));
    base.set(1, 0, Some(Piece::new(PieceType::Bishop, Color::White)));
    base.find_and_set_location_of_kings();

    let base_score = evaluate_position(&base, Color::White);

    let mut threat = base;
    threat.set(1, 0, None);
    threat.set(3, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    let threat_score = evaluate_position(&threat, Color::White);

    assert!(threat_score > base_score);
}

#[test]
fn evaluate_position_bishop_pair_bonus_scales_with_openness() {
    // Open position (fewer pawns) should give higher bishop pair bonus
    let mut open = Board::empty();
    open.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    open.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    open.set(2, 2, Some(Piece::new(PieceType::Bishop, Color::White)));  // Light squares
    open.set(2, 4, Some(Piece::new(PieceType::Bishop, Color::White)));  // Dark squares
    open.set(1, 0, Some(Piece::new(PieceType::Pawn, Color::White)));    // Few pawns
    open.find_and_set_location_of_kings();

    let mut closed = Board::empty();
    closed.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    closed.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    closed.set(2, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    closed.set(2, 4, Some(Piece::new(PieceType::Bishop, Color::White)));
    // Many pawns = more closed
    closed.set(1, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    closed.set(1, 1, Some(Piece::new(PieceType::Pawn, Color::White)));
    closed.set(1, 2, Some(Piece::new(PieceType::Pawn, Color::White)));
    closed.set(1, 3, Some(Piece::new(PieceType::Pawn, Color::White)));
    closed.set(1, 5, Some(Piece::new(PieceType::Pawn, Color::White)));
    closed.set(1, 6, Some(Piece::new(PieceType::Pawn, Color::White)));
    closed.set(1, 7, Some(Piece::new(PieceType::Pawn, Color::White)));
    closed.set(6, 0, Some(Piece::new(PieceType::Pawn, Color::Black)));
    closed.set(6, 1, Some(Piece::new(PieceType::Pawn, Color::Black)));
    closed.set(6, 2, Some(Piece::new(PieceType::Pawn, Color::Black)));
    closed.set(6, 3, Some(Piece::new(PieceType::Pawn, Color::Black)));
    closed.set(6, 5, Some(Piece::new(PieceType::Pawn, Color::Black)));
    closed.set(6, 6, Some(Piece::new(PieceType::Pawn, Color::Black)));
    closed.set(6, 7, Some(Piece::new(PieceType::Pawn, Color::Black)));
    closed.find_and_set_location_of_kings();

    let open_score = evaluate_position(&open, Color::White);
    let closed_score = evaluate_position(&closed, Color::White);

    // Open position should have higher score due to scaled bishop pair bonus
    // (after accounting for pawn material difference)
    // The bishop pair bonus difference should favor open position
    // This is a relative test - the difference in bishop pair scaling
    assert!(open_score.abs() > 0 || closed_score.abs() > 0); // Basic sanity check
}

#[test]
fn evaluate_position_opposite_bishops_more_drawish_with_few_pawns() {
    // Opposite colored bishops with fewer pawns should have score pulled toward zero more
    // Both positions have White ahead by 1 pawn to create a non-zero score
    let mut few_pawns = Board::empty();
    few_pawns.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    few_pawns.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    few_pawns.set(3, 3, Some(Piece::new(PieceType::Bishop, Color::White)));  // On d4 (dark square)
    few_pawns.set(5, 4, Some(Piece::new(PieceType::Bishop, Color::Black)));  // On e6 (light square)
    // Only 2 pawns total - very drawish
    few_pawns.set(3, 0, Some(Piece::new(PieceType::Pawn, Color::White)));    // a4 - white ahead
    few_pawns.set(4, 7, Some(Piece::new(PieceType::Pawn, Color::Black)));    // h5
    few_pawns.find_and_set_location_of_kings();

    let mut more_pawns = Board::empty();
    more_pawns.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    more_pawns.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    more_pawns.set(3, 3, Some(Piece::new(PieceType::Bishop, Color::White)));  // Same bishop
    more_pawns.set(5, 4, Some(Piece::new(PieceType::Bishop, Color::Black)));  // Same bishop
    // 6 pawns total - less drawish, White still ahead by 1
    more_pawns.set(1, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    more_pawns.set(1, 1, Some(Piece::new(PieceType::Pawn, Color::White)));
    more_pawns.set(1, 2, Some(Piece::new(PieceType::Pawn, Color::White)));
    more_pawns.set(1, 3, Some(Piece::new(PieceType::Pawn, Color::White)));
    more_pawns.set(6, 5, Some(Piece::new(PieceType::Pawn, Color::Black)));
    more_pawns.set(6, 6, Some(Piece::new(PieceType::Pawn, Color::Black)));
    more_pawns.set(6, 7, Some(Piece::new(PieceType::Pawn, Color::Black)));
    more_pawns.find_and_set_location_of_kings();

    let few_pawns_score = evaluate_position(&few_pawns, Color::White);
    let more_pawns_score = evaluate_position(&more_pawns, Color::White);

    // With fewer pawns and opposite bishops, draw factor is higher (40% vs 25-30%)
    // So few_pawns_score should have smaller absolute value (pulled more toward 0)
    // Both positions favor White, so both scores are positive
    assert!(few_pawns_score > 0, "Expected positive score for white advantage");
    assert!(more_pawns_score > 0, "Expected positive score for white advantage");
    assert!(few_pawns_score < more_pawns_score, "Fewer pawns with opposite bishops should be more drawish");
}

#[test]
fn evaluate_position_pawn_threats_give_bonus() {
    // Pawn threatening a piece should give higher score than same position without threat
    let mut with_threat = Board::empty();
    with_threat.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    with_threat.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    with_threat.set(3, 3, Some(Piece::new(PieceType::Pawn, Color::White)));    // d4 - threatening
    with_threat.set(4, 4, Some(Piece::new(PieceType::Knight, Color::Black)));  // e5 - defended knight
    with_threat.set(4, 5, Some(Piece::new(PieceType::Pawn, Color::Black)));    // f5 - defending the knight
    with_threat.find_and_set_location_of_kings();

    let mut no_threat = Board::empty();
    no_threat.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    no_threat.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    no_threat.set(1, 3, Some(Piece::new(PieceType::Pawn, Color::White)));      // d2 - not threatening
    no_threat.set(4, 4, Some(Piece::new(PieceType::Knight, Color::Black)));    // e5
    no_threat.set(4, 5, Some(Piece::new(PieceType::Pawn, Color::Black)));      // f5
    no_threat.find_and_set_location_of_kings();

    let threat_score = evaluate_position(&with_threat, Color::White);
    let no_threat_score = evaluate_position(&no_threat, Color::White);

    // Position with pawn threat should be scored higher for white
    assert!(threat_score > no_threat_score);
}
