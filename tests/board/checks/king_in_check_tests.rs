use chess::board::Board;
use chess::board::test_support::is_king_in_check_after_move;
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn is_king_in_check_after_move_detects_discovered_check() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 7, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(1, 4, Some(Piece::new(PieceType::Rook, Color::White)));
    board.set(7, 4, Some(Piece::new(PieceType::Rook, Color::Black)));
    board.find_and_set_location_of_kings();

    let in_check = is_king_in_check_after_move(&mut board, (1, 4), (1, 5), None);
    assert!(in_check, "moving rook off the file should expose check");
}

#[test]
fn is_king_in_check_after_move_handles_en_passant_capture() {
    let mut board = Board::empty();
    board.set(0, 4, Some(Piece::new(PieceType::King, Color::White)));
    board.set(7, 7, Some(Piece::new(PieceType::King, Color::Black)));
    board.set(4, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    board.set(4, 3, Some(Piece::new(PieceType::Pawn, Color::Black)));
    board.set(7, 4, Some(Piece::new(PieceType::Rook, Color::Black)));
    board.find_and_set_location_of_kings();

    let ep_target = Some((5, 3));
    let in_check = is_king_in_check_after_move(&mut board, (4, 4), (5, 3), ep_target);
    assert!(in_check, "en passant should expose the e-file check");
}
