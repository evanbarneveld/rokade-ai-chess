please Here are the orthogonal aspects of your chess engine that can be reviewed separately:

please review the <aspect> aspect of the chess engine. See <main-file> for a starting point

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

[X] Threat Resolution,src/search/evaluation/root_heuristics/threat_resolution.rs
[X] Knight Evacuation,src/search/evaluation/root_heuristics/knight_evacuation.rs
[X] Repetition Avoidance,src/search/evaluation/repetition.rs
[X] Move Generation,src/search/management/move_generator.rs
[X] Board Representation,src/board/board.rs
[X] Attack Detection,src/board/checks/square_attacked.rs

[X] Opening Book,src/book/book.rs
[X] Time Management,src/search/integration/time_control.rs
[X] UCI Protocol,src/uci/mod.rs
[X] Parallel Search,src/search/integration/threading.rs
[X] Playing Strength,src/search/integration/playing_strength.rs

More aspects:

[ ]Piece development
[ ]Personalization, use persona's that define parameters that affect playing style
