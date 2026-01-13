use serial_test::serial;
use chess::generator::move_generator::generate_move_as_san;
use chess::search::SearchMode;
use chess::search::set_deterministic;
use chess::state::fen::reader::reset_from_fen;
use chess::history::history::History;
use chess::search::playing_strength::PLAYING_STRENGTH_MAX;
use chess::piece::piece_mover::PieceMover;
use chess::board::san_move::convert_move_to_san;
use chess::state::game_state::GameState;
use chess::piece::pieces::{Color, PieceType};

const TEST_MOVE_TIME: usize = 500;

/// this test checks if, for a given FEN, the blundering move is not made by the engine
#[test]
#[serial]
fn test_blunder_avoidance() {
    // Ensure deterministic behavior for this test run
    set_deterministic(true);
    // Each case: (FEN, known blunder in coordinate SAN form like "g7g6")

    let mut test_count = 0; // 0 means all tests

    let cases = [
        (
            "rnbq1br1/1pppp1kp/p5pn/4PpN1/2BP4/2N5/PPP2PPP/R1BQK2R w KQ - 0 10",
            "Ne6+",
       ),
       (
            "r1b1k2B/ppp4p/5p2/3pp3/8/8/PP1NPPPP/R3KB1R b q - 0 13",
            "Bc8h3",
        ),
        (
            "r1bqkb1r/pppppppp/2n5/7n/3PP3/2N2N2/PPP2PPP/R1BQKB1R b KQkq - 2 4",
            "Nc6xd4", // Nc6xd4 looses the knight
        ),
        (
            "rnbqkb1r/pppppppp/8/5P2/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3",
            "e7e5", // e7-e5 drops the pawn to dxe5
        ),
        (
            "rnbqkb1r/ppppppp1/5n1p/8/3PP3/2N5/PPP2PPP/R1BQKBNR b KQkq e3 0 3",
            "Nf6g4", // Nf6g4 loses to Qd1xg4
        ),
        (
            "rnbqkb1r/pp1ppppp/7n/2P5/4P3/8/PPP2PPP/RNBQKBNR b KQkq e3 0 3",
            "Nh6f5", // Nh6f5 walks into exf5
        ),
        (
            // Reported case: engine picked ...Bf5-h3 hanging the bishop
            "rn1qkbnr/1pp1pppp/8/p2P1b2/4P3/3P1N2/PP3PPP/RNBQKB1R b KQkq - 0 5",
            "Bf5h3",
        ),
        (
            // Reported case: engine picked ...Bc8-h3 hanging the bishop
            "rnbqkb1r/ppp1pp1p/7n/3p2p1/4P3/2NB1N2/PPPP1PPP/R1BQK2R b KQkq - 1 4",
            "Bc8h3",
        ),
    ];

    let mut current_test_ = 0;
    if test_count == 0 { test_count = cases.len(); }

    for (fen, blunder) in cases {
        let got = best_move_using_depth_search_with_time_limit_for_fen(fen, 4)
            .unwrap_or_else(|| panic!("No move found for FEN: {}", fen));
        eprintln!("FEN: {}\nchose: {}\n", fen, got);
        assert_ne!(
            got, blunder,
            "Engine chose a known blunder at depth 4: {} for FEN {}",
            blunder, fen
        );
        current_test_ += 1;
        if current_test_ >= test_count { break; }
    }
}

#[test]
#[serial]
fn debug_moves_after_e7e5() {
    // Diagnostic: verify that after ...e7e5, White has d4xe5 available
    set_deterministic(true);
    let fen = "rnbqkb1r/pppppppp/8/5P2/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3";
    let mut gs = reset_from_fen(fen).expect("valid FEN");

    // Enumerate Black legal moves and locate e7e5 by SAN string (local generator using public APIs)
    let black_moves = enumerate_legal_moves(&gs);
    let mut e7e5_from_to: Option<((usize, usize), (usize, usize))> = None;
    for (from, to, promo) in &black_moves {
        if let Some(s) = convert_move_to_san(gs, Some((*from, *to, *promo))) {
            //println!("SAN: {}", s);
            if s == "e5" {
                e7e5_from_to = Some((*from, *to));
                break;
            }
        }
    }
    let e7e5 = e7e5_from_to.expect("e7e5 should be a legal move for Black in this FEN");

    // Apply ...e7e5 on full GameState (legality/path handled inside PieceMover)
    let is_capture = gs.board().get(e7e5.1.0, e7e5.1.1).is_some();
    assert!(PieceMover::move_piece(&mut gs, e7e5.0, e7e5.1, is_capture, None));
    gs.switch_player_turn(); // White to move

    // Enumerate White replies and collect SAN strings
    let white_moves = enumerate_legal_moves(&gs);
    let mut sans: Vec<String> = Vec::new();
    let mut has_dxe5 = false;
    for (from, to, promo) in &white_moves {
        if let Some(s) = convert_move_to_san(gs, Some((*from, *to, *promo))) {
            if s.contains('x') { // capture formatting uses 'x'
                sans.push(s.clone());
            }
            if s == "dxe5" { // accept either strict or coordinate capture style
                has_dxe5 = true;
            }
        }
    }
    eprintln!("After ...e7e5, White capture SANs: {:?}", sans);
    assert!(has_dxe5, "Expected d4xe5 to be legal after ...e7e5");
}

// Local minimal legal move enumeration for tests (public-API only)
fn enumerate_legal_moves(game_state: &GameState) -> Vec<((usize, usize), (usize, usize), Option<char>)> {
    let mut result: Vec<((usize, usize), (usize, usize), Option<char>)> = Vec::new();
    let board = game_state.board();
    let active_color = game_state.active_color();

    for r in 0..8 {
        for c in 0..8 {
            let piece = match board.get(r, c) { Some(p) => p, None => continue };
            if piece.get_color() != active_color { continue; }
            for tr in 0..8 { for tc in 0..8 {
                let from = (r, c); let to = (tr, tc);
                if from == to { continue; }
                let is_capture = board.get(tr, tc).is_some()
                    || (piece.get_type() == PieceType::Pawn && game_state.en_passant_target().is_some() && to == game_state.en_passant_target().unwrap());
                let is_pawn_move = piece.get_type() == PieceType::Pawn;
                if !game_state.board().move_from_and_to_validation_check(from, to, active_color, is_capture, is_pawn_move, game_state.en_passant_target()) { continue; }
                // Legality via applying move on a copy using PieceMover
                let mut gs2 = *game_state;
                let is_pawn_promotion = is_pawn_move && ((active_color == Color::White && tr == 7) || (active_color == Color::Black && tr == 0));
                if is_pawn_promotion {
                    // Just try Queen promotion for test enumeration
                    let promo_piece = Some(chess::piece::pieces::Piece::new(PieceType::Queen, active_color));
                    if PieceMover::move_piece(&mut gs2, from, to, is_capture, promo_piece) {
                        result.push((from, to, Some('q')));
                    }
                } else {
                    if PieceMover::move_piece(&mut gs2, from, to, is_capture, None) {
                        result.push((from, to, None));
                    }
                }
            }}
        }
    }
    result
}

fn best_move_using_depth_search_with_time_limit_for_fen(fen: &str, depth: usize) -> Option<String> {
    let gs = reset_from_fen(fen).expect("valid FEN");
    let history = History::new();
    generate_move_as_san(SearchMode::Normal, gs, &history, depth, TEST_MOVE_TIME, PLAYING_STRENGTH_MAX)
}
