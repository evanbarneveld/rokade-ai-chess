use chess::board::Board;
use chess::piece::move_validators::knight_move_validator::is_valid_knight_move;

#[test]
fn knight_move_validates_l_shape() {
    let mut board = Board::empty();
    assert!(is_valid_knight_move(&mut board, (0, 1), (2, 2), false));
    assert!(!is_valid_knight_move(&mut board, (0, 1), (0, 2), false));
}
