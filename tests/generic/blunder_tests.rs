use chess::generator::move_generator::generate_move_as_san;
use chess::search::SearchMode;
use chess::search::SearchContext;
use chess::state::fen::reader::reset_from_fen;
use chess::history::history::History;
use chess::search::integration::playing_strength::PLAYING_STRENGTH_MAX;
use chess::piece::piece_mover::PieceMover;
use chess::board::san_move::convert_move_to_san;
use chess::Chess;
use chess::state::game_state::GameState;
use chess::piece::pieces::{Color, PieceType};
pub(crate) use crate::generic::utils::read_fen_and_generate_best_move;

const TEST_MOVE_TIME: usize = 500;
const TEST_DEFAULT_SEARCH_DEPTH: usize = 6;

/// this test checks if, for a given FEN, the blundering move is not made by the engine
#[test]
fn test_blunder_collection() {
    // Each case: (FEN, known blunder in coordinate SAN form like "g7g6")

    let mut test_count = 0; // 0 means all tests

    let cases = [
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
fn debug_moves_after_e7e5() {
    // Diagnostic: verify that after ...e7e5, White has d4xe5 available
    let fen = "rnbqkb1r/pppppppp/8/5P2/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3";
    let mut gs = reset_from_fen(fen).expect("valid FEN");

    // Enumerate Black legal moves and locate e7e5 by SAN string (local generator using public APIs)
    let black_moves = enumerate_legal_moves(&gs);
    let mut e7e5_from_to: Option<((usize, usize), (usize, usize))> = None;
    for (from, to, promo) in &black_moves {
        if let Some(s) = convert_move_to_san(&gs, Some((*from, *to, *promo))) {
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
        if let Some(s) = convert_move_to_san(&gs, Some((*from, *to, *promo))) {
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
                if !game_state.move_from_and_to_validation_check(from, to, is_capture, is_pawn_move) { continue; }
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
    let ctx = deterministic_ctx();
    generate_move_as_san(&ctx, SearchMode::Normal, &gs, &history, depth, TEST_MOVE_TIME, PLAYING_STRENGTH_MAX)
}

pub(crate) fn deterministic_ctx() -> SearchContext {
    let ctx = SearchContext::new();
    ctx.set_deterministic(true);
    ctx
}

#[test]
fn test_blunder_move_2() {
    let fen = "r1b1kb1r/pppq1ppp/8/3PP3/1n3Bn1/P1N2N2/1PP1QPPP/R3KB1R b KQkq - 0 9";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    //get the best move
    let history = game.get_history().clone();

    let ctx = deterministic_ctx();
    let san_move = generate_move_as_san(
        &ctx,
        game.get_search_mode(),
        game.get_game_state(),
        &history,
        TEST_DEFAULT_SEARCH_DEPTH,
        5000,
        1000,
    ).unwrap();
    println!("Selected move: {:?}", san_move);
    assert_ne!(san_move, "Na6"); //considered a blunder
}

#[test]
fn test_blunder_move_4() {
    let fen = "r1bqkb1r/ppppn1pp/2n2p2/4P3/1PBN1B2/P1N5/2P1QPPP/R4RK1 b kq - 0 12";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("bad fen");
    //get the best move
    let history = game.get_history().clone();

    let ctx = deterministic_ctx();
    let san_move = generate_move_as_san(
        &ctx,
        game.get_search_mode(),
        game.get_game_state(),
        &history,
        TEST_DEFAULT_SEARCH_DEPTH,
        3000,
        1000,
    ).unwrap();
    println!("Selected move: {:?}", san_move);

    /*
    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 7);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in ranks.iter().take(10) {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }
    */

    assert_ne!(san_move, "Nxe5"); //considered a blunder
}

#[test]
fn test_blunder_move_6() {
    let fen = "r1bqkb1r/2p2ppp/p1n2n2/4p1N1/1p1Pp3/1B6/PPP1NPPP/R1BQK2R b KQkq - 1 9";

    let (_game, _history, san_move) = read_fen_and_generate_best_move(fen, 5000);
    println!("Selected move: {:?}", san_move);

    // Debug: rank root moves with adjusted scores
    /*let ranks = debug_rank_root_moves(game.get_game_state(), &history, 7);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in &ranks {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }*/

    assert_ne!(san_move, "exd4"); //considered a blunder
}

#[test]
fn test_blunder_move_7() {
    let fen = "r1bqkb1r/2B2p2/p4n1p/n5p1/1p2N3/1B1p2N1/PP3PPP/2RQK2R b Kkq - 0 15";

    let (_game, _history, san_move) = read_fen_and_generate_best_move(fen, 5000);
    println!("Selected move: {:?}", san_move);

    // Debug: rank root moves with adjusted scores
    /*let ranks = debug_rank_root_moves(game.get_game_state(), &history, 7);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in &ranks {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }*/

    assert_ne!(san_move, "Qd4"); //considered a blunder
}

#[test]
fn _test_blunder_move_8() {
    let fen = "r1b1kb1r/2B5/p1n4p/5pp1/1p1qn3/1B2R3/PP1p1PPP/1R1Q1NK1 b kq - 1 20";

    let (_game, _history, san_move) = read_fen_and_generate_best_move(fen, 5000);
    println!("Selected move: {:?}", san_move);

    /*
    // Debug: rank root moves with adjusted scores
    let ranks = debug_rank_root_moves(game.get_game_state(), &history, 7);
    println!("Root ranks (SAN, adj, raw):");
    for (san, adj, raw) in &ranks {
        println!("  {} -> adj={}, raw={}, diff={}", san, adj, raw, adj - raw);
    }*/

    assert_ne!(san_move, "f4"); //considered a blunder
}
