use chess::board::Board;
use chess::piece::pieces::{Color, Piece, PieceType};
use chess::search::test_support::{
    attacked_by_pawn,
    pawn_attacked_minor_penalty,
    see_after,
    see_dest_estimate,
};

fn setup_board_from_pieces(pieces: &[((usize, usize), PieceType, Color)]) -> Board {
    let mut board = Board::empty();
    for &((r, c), piece_type, color) in pieces {
        board.set(r, c, Some(Piece::new(piece_type, color)));
    }
    board
}

#[test]
fn test_see_simple_pawn_capture() {
    let mut board = setup_board_from_pieces(&[
        ((3, 4), PieceType::Pawn, Color::White), // e4
    ]);
    board.set(3, 3, Some(Piece::new(PieceType::Pawn, Color::White))); // d5
    board.set(3, 4, None);

    let see = see_dest_estimate(&board, Color::White, (3, 3), 100);
    assert_eq!(see, 100, "Undefended pawn capture should return pawn value");
}

#[test]
fn test_see_knight_takes_pawn_defended_by_pawn() {
    let mut board = setup_board_from_pieces(&[
        ((4, 2), PieceType::Pawn, Color::Black), // Defender pawn at c4
    ]);
    board.set(3, 3, Some(Piece::new(PieceType::Knight, Color::White))); // d3

    let see = see_dest_estimate(&board, Color::White, (3, 3), 100);
    assert!(see < 0, "Knight taking defended pawn should be negative SEE, got {}", see);
}

#[test]
fn test_see_equal_trade_knight_for_knight() {
    let mut board = setup_board_from_pieces(&[
        ((4, 2), PieceType::Pawn, Color::Black),
    ]);
    board.set(3, 3, Some(Piece::new(PieceType::Knight, Color::White)));

    let see = see_dest_estimate(&board, Color::White, (3, 3), 320);
    assert_eq!(see, 0, "Equal trade should result in SEE of 0");
}

#[test]
fn test_see_queen_takes_pawn_attacked_by_pawn() {
    let mut board = setup_board_from_pieces(&[
        ((4, 2), PieceType::Pawn, Color::Black),
    ]);
    board.set(3, 3, Some(Piece::new(PieceType::Queen, Color::White)));

    let see = see_dest_estimate(&board, Color::White, (3, 3), 100);
    assert!(see < -700, "Queen taking pawn attacked by pawn should be very negative");
}

#[test]
fn test_see_xray_attack_rook_behind_bishop() {
    let mut board = setup_board_from_pieces(&[
        ((3, 4), PieceType::Rook, Color::White),
        ((2, 5), PieceType::Pawn, Color::Black),
    ]);
    board.set(2, 4, Some(Piece::new(PieceType::Bishop, Color::White)));

    let see = see_dest_estimate(&board, Color::White, (2, 4), 100);
    assert!(see <= 100, "SEE should handle X-ray attacks");
}

#[test]
fn test_see_multi_piece_exchange() {
    let mut board = setup_board_from_pieces(&[
        ((2, 5), PieceType::Knight, Color::Black),
        ((1, 6), PieceType::Bishop, Color::Black),
    ]);
    board.set(3, 3, Some(Piece::new(PieceType::Queen, Color::White)));

    let see = see_dest_estimate(&board, Color::White, (3, 3), 100);
    assert!(see < 0, "Queen taking defended pawn should be negative");
}

#[test]
fn test_see_no_attacker() {
    let mut board = setup_board_from_pieces(&[]);
    board.set(3, 3, Some(Piece::new(PieceType::Knight, Color::White)));

    let see = see_dest_estimate(&board, Color::White, (3, 3), 100);
    assert_eq!(see, 100, "No attackers => captured value");
}

#[test]
fn test_see_king_capture() {
    let mut board = setup_board_from_pieces(&[
        ((2, 3), PieceType::King, Color::Black),
    ]);
    board.set(3, 3, Some(Piece::new(PieceType::Pawn, Color::White)));

    let see = see_dest_estimate(&board, Color::White, (3, 3), 100);
    assert_eq!(see, 0, "Equal exchange should result in SEE of 0, got {}", see);
}

#[test]
fn test_see_after_helper() {
    let board = setup_board_from_pieces(&[
        ((3, 3), PieceType::Knight, Color::White),
        ((2, 2), PieceType::Pawn, Color::Black),
    ]);

    let captured = Some(Piece::new(PieceType::Pawn, Color::Black));
    let see = see_after(&board, Color::White, (3, 3), captured);

    assert!(see <= 100, "see_after should call see_dest_estimate correctly");
}

#[test]
fn test_attacked_by_pawn() {
    let board = setup_board_from_pieces(&[
        ((2, 2), PieceType::Pawn, Color::White),
    ]);

    assert!(attacked_by_pawn(&board, (3, 3), Color::White), "Should detect pawn attack on d5");
    assert!(attacked_by_pawn(&board, (3, 1), Color::White), "Should detect pawn attack on b5");
    assert!(!attacked_by_pawn(&board, (3, 2), Color::White), "Should not detect attack on c5");
}

#[test]
fn test_pawn_attacked_minor_penalty() {
    let board = setup_board_from_pieces(&[
        ((4, 2), PieceType::Pawn, Color::Black),
    ]);

    let penalty = pawn_attacked_minor_penalty(&board, Color::White, (3, 3), PieceType::Knight);
    assert_eq!(penalty, 1200, "Knight attacked by pawn should have penalty");

    let no_penalty = pawn_attacked_minor_penalty(&board, Color::White, (3, 3), PieceType::Queen);
    assert_eq!(no_penalty, 0, "Queen shouldn't have minor pawn attack penalty");
}
