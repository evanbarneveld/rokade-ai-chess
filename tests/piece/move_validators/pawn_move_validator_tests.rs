use chess::board::Board;
use chess::piece::move_validators::pawn_move_validator::is_valid_pawn_move;
use chess::piece::pieces::{Color, Piece, PieceType};

#[test]
fn pawn_move_allows_double_step_from_start() {
    let mut board = Board::empty();
    board.set(1, 4, Some(Piece::new(PieceType::Pawn, Color::White)));
    let ok = is_valid_pawn_move(&mut board, (1, 4), (3, 4), false, None, Color::White, None, false);
    assert!(ok);
}

#[test]
fn pawn_move_requires_promotion_piece_on_last_rank() {
    let mut board = Board::empty();
    board.set(6, 0, Some(Piece::new(PieceType::Pawn, Color::White)));
    let no_promo = is_valid_pawn_move(&mut board, (6, 0), (7, 0), false, None, Color::White, None, false);
    let promo = is_valid_pawn_move(
        &mut board,
        (6, 0),
        (7, 0),
        false,
        None,
        Color::White,
        Some(Piece::new(PieceType::Queen, Color::White)),
        false,
    );
    assert!(!no_promo);
    assert!(promo);
}
