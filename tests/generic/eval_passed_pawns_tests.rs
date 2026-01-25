use chess::board::evaluator::evaluate_position;
use chess::state::fen::reader::reset_from_fen;

// Helper: evaluate a FEN and return score
fn eval(fen: &str) -> i32 {
    let gs = reset_from_fen(fen).expect("valid FEN");
    evaluate_position(gs.board(), gs.active_color())
}

#[test]
fn passed_pawn_clear_path_better_than_blocked() {
    // Clear path: White pawn on e6 with clear way to promote, kings far
    let clear = "4k3/8/4P3/8/8/8/4K3/8 w - - 0 1";
    // Blocked by an enemy piece (still a passed pawn but no clear path): black bishop on e7
    let blocked = "4k3/4b3/4P3/8/8/8/4K3/8 w - - 0 1";

    let s_clear = eval(clear);
    let s_blocked = eval(blocked);

    // Expect clear > blocked by a noticeable margin
    assert!(s_clear > s_blocked + 10, "clear={} blocked={}", s_clear, s_blocked);
}

#[test]
fn rook_behind_passed_pawn_endgame_is_rewarded() {
    // Endgame-like: only kings, a white rook and a white passed pawn on e5
    // Variant A: rook behind the pawn on e1 (same file, clear path)
    let fen_behind = "6k1/8/8/4P3/8/8/8/4R1K1 w - - 0 1";
    // Variant B: rook on a1 (not behind the passer)
    let fen_not_behind = "6k1/8/8/4P3/8/8/8/R5K1 w - - 0 1";

    let s_behind = eval(fen_behind);
    let s_not = eval(fen_not_behind);

    // Expect the rook-behind setup to score higher for White
    assert!(s_behind > s_not + 5, "behind={} not_behind={}", s_behind, s_not);
}

#[test]
fn black_passed_pawn_mirror_consistency() {
    // Mirror of the first scenario but for Black to ensure consistent handling
    let clear_black = "8/4k3/8/8/8/4p3/8/4K3 b - - 0 1";   // black pawn on e3, clear path
    let blocked_black = "8/4k3/8/8/8/4p3/4B3/4K3 b - - 0 1"; // white bishop on e2 blocking

    let s_clear = eval(clear_black);   // better for Black → more negative
    let s_blocked = eval(blocked_black);

    assert!(s_clear < s_blocked - 10, "clear_black={} blocked_black={}", s_clear, s_blocked);
}
