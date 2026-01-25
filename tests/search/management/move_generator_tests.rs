use chess::search::test_support::{find_all_capture_moves, find_all_evasion_moves};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn find_all_capture_moves_excludes_quiet() {
    let fen = "4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let moves = find_all_capture_moves(&mut gs);

    assert!(moves.iter().any(|(f, t, _)| *f == (3, 4) && *t == (4, 3)));
    assert!(!moves.iter().any(|(f, t, _)| *f == (3, 4) && *t == (4, 4)));
}

#[test]
fn find_all_capture_moves_includes_en_passant() {
    let fen = "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let moves = find_all_capture_moves(&mut gs);

    assert!(moves.iter().any(|(f, t, _)| *f == (4, 4) && *t == (5, 3)));
}

#[test]
fn find_all_evasion_moves_blocks_check() {
    let fen = "4k3/8/8/8/8/8/4r3/4K3 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let moves = find_all_evasion_moves(&mut gs);

    assert!(moves.iter().any(|(f, t, _)| *f == (0, 4) && *t == (0, 5)));
    assert!(moves.iter().any(|(f, t, _)| *f == (0, 4) && *t == (1, 4)));
    assert!(!moves.iter().any(|(f, t, _)| *f == (0, 4) && *t == (0, 6)));
}

#[test]
fn find_all_evasion_moves_double_check_only_king() {
    let fen = "4k3/8/8/8/8/4b3/3r4/4K3 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let moves = find_all_evasion_moves(&mut gs);

    assert!(moves.iter().all(|(f, _, _)| *f == (0, 4)));
}
