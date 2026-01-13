use serial_test::serial;
use chess::Chess;
use chess::search::core::advanced_search::{find_all_valid_moves, _dump_all_valid_moves};
use chess::board::san_move::convert_move_to_san;
use chess::search::set_deterministic;
use chess::piece::pieces::Color;
use chess::piece::move_validators::knight_move_validator::is_valid_knight_move;
use chess::search::core::advanced_search::debug_rank_root_moves;

// Diagnostic test to enumerate legal moves for the mate-in-2 FEN and assert Nf6+ is present (official SAN)
#[test]
#[serial]
fn debug_list_moves_mate_in_2_position() {
    let fen = "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("valid FEN");
    set_deterministic(true);

    // Dump moves in SAN for easy visual inspection
    _dump_all_valid_moves(game.get_game_state(), true);

    // Programmatically verify Nf6+ is among legal moves (official SAN)
    let moves = find_all_valid_moves(game.get_game_state());
    let from = (4, 3); // d5
    let to = (5, 5);   // f6
    let mut found_pair = false;
    for (f, t, _p) in &moves {
        if *f == from && *t == to { found_pair = true; break; }
    }
    assert!(found_pair, "Expected to find move pair d5->f6 in generated moves list");

    let mut found_nfx = false;
    for (from, to, promo) in moves {
        if let Some(san) = convert_move_to_san(*game.get_game_state(), Some((from, to, promo))) {
            if san == "Nf6+" { found_nfx = true; break; }
        }
    }
    assert!(found_nfx, "Expected to find legal move Nf6+ in generated moves list (SAN)");
}

// Pin/self-check diagnostic for the same FEN to see why Nd5-f6 might be rejected
#[test]
#[serial]
fn debug_check_self_check_for_nd5f6() {
    let fen = "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("valid FEN");
    let board = game.get_game_state().mutable_board();

    // White king location should be e1 -> (row=0, col=4)
    let w_king = board.get_king_location(Color::White);
    assert_eq!(w_king, (0, 4), "Expected white king at e1, got {:?}", w_king);

    // Test if moving the white knight from d5 (4,3) to f6 (5,5) is falsely flagged as illegal due to self-check
    let from = (4, 3); // d5
    let to = (5, 5);   // f6
    let legal_with_pin_check = is_valid_knight_move(board, from, to, true);
    assert!(legal_with_pin_check, "Nd5-f6 should be considered a legal knight move (pin check passed)");
}

// Print ranked root moves with adjusted and raw scores to diagnose ordering/selection
#[test]
#[serial]
fn debug_rank_root_moves_for_mate_in_2() {
    let fen = "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 0 1";
    let mut game = Chess::new();
    game.set_starting_fen(fen).expect("valid FEN");
    set_deterministic(true);

    let gs = *game.get_game_state();
    let hist = game.get_history().clone();
    let rankings = debug_rank_root_moves(&gs, &hist, 4);
    println!("Ranked root moves (SAN, adjusted, raw):");
    for (san, adj, raw) in rankings.iter().take(15) {
        println!("{}  adj:{}  raw:{}", san, adj, raw);
    }
}
