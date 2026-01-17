please Here are the orthogonal aspects of your chess engine that can be reviewed separately:

please review the <aspect> aspect of the chess engine. See <main-file> for a starting point


some logic is backwards???? unusual for a chess engine?? (white/black symmetry)

Aspect,Main File
----------------
Done (=reviewed/fixed/tested)
[X] Alpha-Beta/PVS,src/search/core/alphabeta.rs
[X] Quiescence Search,src/search/core/qsearch.rs
[X] Iterative Deepening,src/search/core/advanced_search.rs
[X] Aspiration Windows,src/search/management/aspiration.rs
[X] Null Move Pruning,src/search/management/prune_null_moves.rs
[X] Late Move Reduction,src/search/core/alphabeta.rs
[X] Futility Pruning,src/search/core/qsearch.rs
[X] Delta Pruning,src/search/core/qsearch.rs

[X] SEE Pruning,src/search/management/see.rs
[X] TT Move Priority,src/search/core/alphabeta.rs
[X] MVV-LVA,src/search/core/alphabeta.rs
[X] Killer Moves,src/search/evaluation/heuristics.rs
[X] History Heuristic,src/search/evaluation/heuristics.rs
[X] Transposition Table,src/search/state/tt.rs
[X] Zobrist Hashing,src/search/state/zobrist.rs
[X] Material + PST,src/board/pst.rs

[X] Pawn Structure,src/board/evaluators/evaluate_pawns.rs
[X] Knight Evaluation,src/board/evaluators/evaluate_knights.rs
[X] Bishop Evaluation,src/board/evaluators/evaluate_bishops.rs
[X] Rook Evaluation,src/board/evaluators/evaluate_rooks.rs
[X] Queen Evaluation,src/board/evaluators/evaluate_queens.rs
[X] King Safety,src/board/evaluators/evaluate_king.rs
[X] Mobility & Activity,src/board/evaluator.rs
[X] Hanging Pieces,src/board/evaluator.rs

[ ] Threat Resolution,src/search/evaluation/root_heuristics/threat_resolution.rs
[ ] Knight Evacuation,src/search/evaluation/root_heuristics/knight_evacuation.rs
[ ] Repetition Avoidance,src/search/evaluation/repetition.rs
[ ] Move Generation,src/search/management/move_generator.rs
[ ] Board Representation,src/board/board.rs
[ ] Attack Detection,src/board/checks/square_attacked.rs

[ ] Opening Book,src/book/book.rs
[ ] Time Management,src/search/integration/time_control.rs
[ ] UCI Protocol,src/uci/mod.rs
[ ] Opening Book,src/book/book.rs
[ ] Parallel Search,src/search/integration/threading.rs
[ ] Playing Strength,src/search/integration/playing_strength.rs

More aspects:

[ ]Piece development
[ ]Personalization, use persona's that define parameters that affect playing style
