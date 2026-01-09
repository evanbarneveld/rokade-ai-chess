use serial_test::serial;
use chess::board::evaluator::evaluate_position;
use chess::state::fen::reader::reset_from_fen;

// Mirrors a FEN across the horizontal midline and swaps colors.
// - Flips rank order and swaps piece colors by case
// - Swaps active color w<->b
// - Castling rights letters swap case (KQkq -> kqKQ)
// - En passant target rank is mirrored if present
fn mirror_fen(fen: &str) -> String {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    assert!(parts.len() >= 4, "Invalid FEN");
    let board = parts[0];
    let active = parts[1];
    let castling = parts[2];
    let ep = parts[3];
    let halfmove = if parts.len() > 4 { parts[4] } else { "0" };
    let fullmove = if parts.len() > 5 { parts[5] } else { "1" };

    // Flip ranks and swap piece cases
    let ranks: Vec<&str> = board.split('/').collect();
    let mut flipped: Vec<String> = Vec::with_capacity(8);
    for r in ranks.into_iter().rev() {
        let mut row = String::with_capacity(r.len());
        for ch in r.chars() {
            if ch.is_ascii_alphabetic() {
                if ch.is_ascii_lowercase() { row.push(ch.to_ascii_uppercase()); } else { row.push(ch.to_ascii_lowercase()); }
            } else {
                row.push(ch);
            }
        }
        flipped.push(row);
    }
    let new_board = flipped.join("/");

    let new_active = if active == "w" { "b" } else { "w" };

    let new_castling: String = castling.chars().map(|c| {
        match c {
            '-' => '-',
            'K' => 'k', 'Q' => 'q', 'k' => 'K', 'q' => 'Q',
            _ => c, // ignore non-standard
        }
    }).collect();
    let new_castling = if new_castling.is_empty() { String::from("-") } else { new_castling };

    let new_ep = if ep == "-" {
        String::from("-")
    } else {
        // Flip rank number: 'a3' -> 'a6'
        let bytes = ep.as_bytes();
        if bytes.len() == 2 {
            let file = bytes[0] as char;
            let rank = bytes[1] as char;
            if rank.is_ascii_digit() {
                let r = rank as i32 - '0' as i32; // 1..8
                let new_r = 9 - r; // mirror
                format!("{}{}", file, new_r)
            } else { ep.to_string() }
        } else { ep.to_string() }
    };

    format!("{} {} {} {} {} {}", new_board, new_active, new_castling, new_ep, halfmove, fullmove)
}

#[test]
#[serial]
fn eval_is_approximately_antisymmetric_startpos() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let gs = reset_from_fen(fen).expect("valid FEN");
    let s1 = evaluate_position(gs.board(), gs.active_color());

    let fen_m = mirror_fen(fen);
    let gs_m = reset_from_fen(&fen_m).expect("valid mirrored FEN");
    let s2 = evaluate_position(gs_m.board(), gs_m.active_color());

    let eps = 8; // 8 centipawns tolerance
    assert!( (s1 + s2).abs() <= eps, "startpos antisymmetry: s1={}, s2={}, eps={}\nmirrored FEN: {}", s1, s2, eps, fen_m);
}

#[test]
#[serial]
fn eval_is_approximately_antisymmetric_examples() {
    let examples = vec![
        // Slight white edge position from eval tests
        "r1bqkbnr/1ppp1ppp/p1n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4",
        // Endgame rook+pawns vs rook motif
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        // Simple passer setup
        "8/8/3k4/8/3P4/8/8/3K4 w - - 0 1",
    ];

    let eps = 8; // 8 centipawns
    for fen in examples {
        let gs = reset_from_fen(fen).expect("valid FEN");
        let s1 = evaluate_position(gs.board(), gs.active_color());

        let fen_m = mirror_fen(fen);
        let gs_m = reset_from_fen(&fen_m).expect("valid mirrored FEN");
        let s2 = evaluate_position(gs_m.board(), gs_m.active_color());

        assert!( (s1 + s2).abs() <= eps, "antisymmetry failed for FEN {}\ns1={}, s2={}, eps={}\nmirrored FEN: {}", fen, s1, s2, eps, fen_m);
    }
}
