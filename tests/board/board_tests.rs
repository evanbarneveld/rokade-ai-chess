use chess::board::Board;
use chess::piece::pieces::{Color, Piece, PieceType, piece_value_cp};

#[test]
fn board_has_non_pawn_material_in_initial_position() {
    let board = Board::new();
    assert!(board.has_non_pawn_material(Color::White));
    assert!(board.has_non_pawn_material(Color::Black));
}

#[test]
fn move_piece_basic_updates_king_location() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.find_and_set_location_of_kings();

    assert!(board.move_piece_basic((0, 4), (1, 4)));
    assert_eq!(board.get_king_location(Color::White), (1, 4));
    assert!(board.get(1, 4).is_some());
}

#[test]
fn is_passed_pawn_simple_detects_blocking_pawns() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(3, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.find_and_set_location_of_kings();

    assert!(board.is_passed_pawn_simple(3, 4, Color::White));

    board.set(5, 4, Some(Piece::new(PieceType::Pawn, Color::Black)));
    assert!(!board.is_passed_pawn_simple(3, 4, Color::White));
}

#[test]
fn move_score_mvv_lva_scores_captures() {
    let mut board = Board::empty();
    board.set(1, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.set(2, 0, Some(Piece::new(PieceType::Queen, Color::Black)));
    let score = board.move_score_mvv_lva((1, 0), (2, 0));

    let expected = piece_value_cp(PieceType::Queen) * 10 - piece_value_cp(PieceType::Pawn) / 10;
    assert_eq!(score, expected);
}

#[test]
fn is_square_pawn_attacked_by_detects_attack() {
    let mut board = Board::empty();
    board.set(4, 4, Some(Piece::new(PieceType::Pawn, Color::Black)));
    assert!(board.is_square_pawn_attacked_by(Color::Black, (3, 3)));
    assert!(board.is_square_pawn_attacked_by(Color::Black, (3, 5)));
}
