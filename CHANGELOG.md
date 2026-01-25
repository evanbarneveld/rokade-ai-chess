25-01-2026
Tightened null-move pruning with static-eval gating, dynamic reductions, and endgame safety.
Added tests in:
- prune_null_moves_tests.rs

25-01-2026
Retuned LMR reductions with killer/counter/continuation awareness and safer check handling.
Added tests in:
- alphabeta_tests.rs

25-01-2026
Improved move ordering with SEE capture scoring plus counter/continuation history heuristics.
Added tests in:
- heuristics_tests.rs

25-01-2026
Refined king safety pawn shields, added pawn majority bonuses, retuned pawn structure penalties, and added endgame queen centralization.
Added tests in:
- evaluate_king_tests.rs
- evaluate_pawns_tests.rs
- evaluate_queens_tests.rs

25-01-2026
Added bishop mobility evaluation and tuned bishop development bonus.
Added tests in:
- evaluate_bishops_tests.rs

25-01-2026
Added rook blockade evaluation for enemy passed pawns and knight rim penalties.
Added tests in:
- evaluate_rooks_tests.rs
- evaluate_knights_tests.rs

25-01-2026
Fixed deterministic time budget test expectation to match the +50% bonus.

25-01-2026
Expanded evaluation for passed pawns, king safety, minor pieces, rook/queen activity, and threats.
The following tests were added:
- evaluate_pawns_tests.rs
- evaluate_king_tests.rs
- evaluate_bishops_tests.rs
- evaluate_rooks_tests.rs
- evaluator_tests.rs

25-01-2026
Adjusted time budget semantics (0ms means immediate) and clarified UCI go infinite handling.
The following tests were added:
- time_control_tests.rs

25-01-2026
Reduced deterministic time budget extension from +100% to +50%.

25-01-2026
Capped PV-change time extensions to +75% of the base budget.
