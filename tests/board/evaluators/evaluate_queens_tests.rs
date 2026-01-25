use chess::board::Board;
use chess::board::test_support::{
    early_queen_penalty,
    evaluate_queen,
    pawn_file_counts,
    queen_on_semi_open_file_bonus,
};
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn early_queen_penalty_applies_when_minors_at_home() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(3, 7, Some(Piece::new(PieceType::Queen, Color::White)));
    board.set(0, 1, Some(Piece::new(PieceType::Knight, Color::White)));
    board.set(0, 2, Some(Piece::new(PieceType::Bishop, Color::White)));
    board.set(0, 5, Some(Piece::new(PieceType::Bishop, Color::White)));
    board.set(0, 6, Some(Piece::new(PieceType::Knight, Color::White)));
    board.find_and_set_location_of_kings();

    let counts = pawn_file_counts(&board);
    let penalty = early_queen_penalty(&board, Color::White, &counts);
    assert!(penalty > 0);
}

#[test]
fn queen_on_semi_open_file_bonus_requires_open_file() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(3, 3, Some(Piece::new(PieceType::Queen, Color::White)));
    board.find_and_set_location_of_kings();

    let counts = pawn_file_counts(&board);
    let bonus = queen_on_semi_open_file_bonus(&board, Color::White, &counts);
    assert!(bonus > 0);
}

#[test]
fn evaluate_queen_rewards_endgame_centralization() {
    let mut center = Board::empty();
    center.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    center.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    center.set(3, 3, Some(Piece::new(PieceType::Queen, Color::White)));
    center.find_and_set_location_of_kings();

    let mut corner = Board::empty();
    corner.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    corner.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    corner.set(0, 0, Some(Piece::new(PieceType::Queen, Color::White)));
    corner.find_and_set_location_of_kings();

    let center_score = evaluate_queen(&center, 3, 3, Color::White, 0);
    let corner_score = evaluate_queen(&corner, 0, 0, Color::White, 0);
    assert!(center_score > corner_score);
}
