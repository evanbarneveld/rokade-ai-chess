use chess::board::san_move::convert_move_to_san;
use chess::history::history::History;
use chess::piece::piece_mover::PieceMover;
use chess::piece::pieces::{Color, Piece};
use chess::search::core::advanced_search::{find_all_valid_moves, find_best_move, find_best_move_with_ranking};
use chess::search::{find_best_move_with_mode, SearchMode};
use chess::search::SearchContext;
use chess::search::test_support::{score_raw_for_strength_move, Bound, to_tt_score};
use chess::state::fen::reader::reset_from_fen;

#[test]
fn test_mate_in_1_using_advanced_search() {
    // Back rank mate: White rook on d1, Black king on g8, White king on g1
    // The only winning move is Rd8# (checkmate)
    let fen = "6k1/5ppp/8/8/8/8/r4PPP/3R2K1 w - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    // Depth 2 is sufficient to find mate-in-1
    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Normal, &gs, &history, 2,1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_ne!(san_move.clone().unwrap(), "Rd7"); //weak move
    assert_eq!(san_move.unwrap(), "Rd8#"); //best move, checkmate!
}

#[test]
fn test_mate_in_2_using_advanced_search() {
    let fen = "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Normal, &gs, &history, 4,1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_eq!(san_move.unwrap(), "Nf6+")
}

#[test]
fn test_mate_in_2_using_advanced_search_2() {
    let fen = "r1nk3r/2b2ppp/p3b3/3NN3/Q2P3q/B2B4/P4PPP/4R1K1 w - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    // Depth 4 is sufficient to find mate-in-2
    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Normal, &gs, &history, 4,1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_ne!(san_move.clone().unwrap(), "Nc6+"); //not the best move
    assert_eq!(san_move.unwrap(), "Qd7+"); //best move
}

#[test]
fn test_mate_in_2_using_advanced_search_3() {
    //this test fails when QUIESCENCE_ENABLED is set to false!
    let fen = "r2qk2r/pb4pp/1n2Pb2/2B2Q2/p1p5/2P5/2B2PPP/RN2R1K1 w - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    // Depth 4 is sufficient to find mate-in-2
    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Normal, &gs, &history, 4,1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_ne!(san_move.clone().unwrap(), "Qh5+"); //not the best move (mate in 3)
    assert_eq!(san_move.unwrap(), "Qg6+"); //best move (mate in 2)
}

#[test]
fn test_mate_in_1_using_advanced_search_4() {
    let fen = "6r1/1r2p2k/3pP1pB/2p1p3/P6Q/P1q3P1/7P/5BK1 w - - 0 2";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Normal, &gs, &history, 3,1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_eq!(san_move.unwrap(), "Bf8#"); //best move (mate in 2)
}

#[test]
fn test_mate_in_2_using_advanced_search_5() {
    let fen = "5r2/7p/3R4/p3pk2/1p2N2p/1P2BP2/6PK/4r3 w - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Normal, &gs, &history, 2,1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_ne!(san_move.clone().unwrap(), "g4#"); //there is no mate yet
    assert_eq!(san_move.unwrap(), "g4+"); //best move (mate in 2)
}


#[test]
fn test_mate_in_1_using_advanced_search_6() {

    let fen = "6k1/1p5p/3P3B/4p3/2N1P1p1/PPr5/3R1b1K/5b1R b - - 0 2";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Normal, &gs, &history, 2,1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    //assert_ne!(san_move.clone().unwrap(), "g4#"); //there is no mate yet
    assert_eq!(san_move.unwrap(), "Rh3#"); //or Rh3# best move (mate in 2)
}

#[test]
fn test_mate_in_3_using_advanced_search_1() {

    let fen = "r2n1rk1/1ppb2pp/1p1p4/3Ppq1n/2B3P1/2P4P/PP1N1P1K/R2Q1RN1 b - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    // Mate-in-3 requires depth 6 to see full line: 1...Qxf2+ 2.Rxf2 Rxf2+ 3.Kh1 Ng3#
    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Normal, &gs, &history, 6, 1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_ne!(san_move.clone().unwrap(), "Qf4+"); //not the best move, this is not a mate in 3
    assert_eq!(san_move.unwrap(), "Qxf2+"); //best move
}


#[test]
fn test_mate_in_3_using_advanced_search_2() {

    let fen = "N1bk4/pp1p1Qpp/8/2b5/3n3q/8/PPP2RPP/RNB1rBK1 b - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    // Mate-in-3 requires depth 6
    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Normal, &gs, &history, 6, 1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_eq!(san_move.unwrap(), "Ne2+"); //best move
}

#[test]
fn test_mate_in_3_using_advanced_search_3() {
    //from Stellan Brynell vs Lars Karlsson, Malme, 1986
    let fen = "r4r2/p1p4p/1p2R3/5p2/2B2K2/7k/PPP2P2/8 w - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    // Mate-in-3 requires depth 6
    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Normal, &gs, &history, 6, 1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_ne!(san_move.clone().unwrap(), "Rh6+"); //bad move
    assert_eq!(san_move.unwrap(), "Bd5"); //best move
}

#[test]
fn test_mate_in_3_using_advanced_search_4() {
    //from Chan Wei-Xuan vs Wesley So, Singapore, 2007
    let fen = "8/6k1/3p1rp1/3Bp1p1/1pP1P1K1/4bPR1/P5Q1/4q3 b - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    // Mate-in-3 requires depth 6
    let generated_move = find_best_move_with_mode(&ctx, SearchMode::Normal, &gs, &history, 6, 1000).map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo));
    let san_move = convert_move_to_san(&gs , generated_move);

    println!("Selected move: {:?}", san_move);

    assert_ne!(san_move.clone().unwrap(), "Bc5"); //bad move
    assert_eq!(san_move.unwrap(), "Rf4+"); //best move
}

#[test]
fn find_best_move_with_ranking_includes_all_moves() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();
    ctx.set_order_book_enabled(false);

    let mut gs_moves = gs;
    let moves = find_all_valid_moves(&mut gs_moves);
    let ranks = find_best_move_with_ranking(&ctx, &gs, &history, 1);

    assert_eq!(ranks.len(), moves.len());
}

#[test]
fn find_best_move_with_ranking_sorts_for_black() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    let ranks = find_best_move_with_ranking(&ctx, &gs, &history, 1);
    for i in 1..ranks.len() {
        assert!(ranks[i - 1].1 <= ranks[i].1);
    }
}

#[test]
fn find_best_move_returns_none_when_no_legal_moves() {
    let fen = "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();

    let mv = find_best_move(&ctx, &gs, &history, 2, 1000);
    assert!(mv.is_none());
}

#[test]
fn find_best_move_with_ranking_iterative_deepening_same_as_final_depth() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();
    ctx.set_order_book_enabled(false);

    let ranks = find_best_move_with_ranking(&ctx, &gs, &history, 3);
    let mv = find_best_move(&ctx, &gs, &history, 3, 1000)
        .map(|(from, to, promo, _score_cp, _depth_used)| (from, to, promo))
        .expect("expected a best move");
    let top = ranks[0].0.clone();
    let top_from_to = find_all_valid_moves(&mut gs.clone()).into_iter().find_map(|(from, to, promo)| {
        let san = convert_move_to_san(&gs, Some((from, to, promo)))?;
        if san == top {
            Some((from, to, promo))
        } else {
            None
        }
    });

    assert_eq!(Some(mv), top_from_to);
}

#[test]
fn find_best_move_returns_depth_used_from_iterative_deepening() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("Invalid FEN");
    let history = History::new();
    let ctx = deterministic_ctx();
    ctx.set_order_book_enabled(false);

    let mv = find_best_move(&ctx, &gs, &history, 3, 1000).expect("expected a best move");
    assert_eq!(mv.4, 3);
}

#[test]
fn strength_score_uses_tt_sign() {
    let fen = "6k1/5ppp/8/8/8/8/r4PPP/3R2K1 w - - 0 1";
    let mut gs = reset_from_fen(fen).expect("Invalid FEN");
    let ctx = SearchContext::new();
    let tt = ctx.tt();
    let moves = find_all_valid_moves(&mut gs);
    assert!(!moves.is_empty(), "expected legal moves");

    let (from, to, promo) = moves[0];
    let mut temp_gs = gs;
    let is_capture = temp_gs.board().get(to.0, to.1).is_some();
    let promotion_piece = promo.and_then(Piece::from_fen_char);

    assert!(
        PieceMover::move_piece(&mut temp_gs, from, to, is_capture, promotion_piece),
        "expected move to be legal"
    );
    temp_gs.switch_player_turn();

    let score = 321;
    let key = temp_gs.zobrist_key();
    tt.store(key, 1, Bound::Exact, to_tt_score(score, 0), None, None);

    let score_raw = score_raw_for_strength_move(&temp_gs, Color::White, tt);
    assert_eq!(score_raw, score);
}


fn deterministic_ctx() -> SearchContext {
    let ctx = SearchContext::new();
    ctx.set_deterministic(true);
    ctx
}
