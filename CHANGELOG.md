28-01-2026
Fixed LMR being disabled too aggressively in endgames, causing slow mate-in-4 searches.
- Changed LMR phase threshold from 8 to 5 (was disabling all reductions when phase <= 8)
- Phase 8 includes positions with 2 rooks + 2 minors, which is not a deep endgame
- test_mate_in_4_failure_2 now runs in ~8 seconds instead of 50+ seconds (6x speedup)

27-01-2026
Implemented evaluation refinements for improved engine strength.
- King Virtual Mobility: Penalizes kings with few safe escape squares (-24cp for 0 escapes, scaled by phase/queen)
- Bishop Outposts: Bonus for bishops on protected squares that can't be attacked by enemy pawns (+14cp MG, +6cp EG)
- Bishop Pair Scaling: Bonus now scales with position openness (80-120% of base +36/+24cp)
- Enhanced Opposite Bishops: Draw factor scales with pawn count (25-50% score reduction)
- Enhanced Threats: Extra bonus for pawn threats (+12cp), multi-threat synergy (+8cp per additional threat)
- Passed Pawn King Distance Ratio: Compares relative king distances (±15cp max)
- Tarrasch Rule: Rook behind passed pawn bonus/penalty (±12cp endgame)
Added tests in:
- evaluate_bishops_tests.rs, evaluate_king_tests.rs, evaluate_pawns_tests.rs, evaluator_tests.rs

26-01-2026
Added configurable evaluation heuristics for testing which heuristics are most effective.
- Added `eval_config.rs` module with bitflags for 10 evaluation categories
- Categories: TEMPO, HANGING, MOBILITY, CENTER, PAWN_STRUCTURE, KING_SAFETY, ROOK_ACTIVITY, THREATS, INTERACTIONS, IMBALANCE
- CLI command `eval-config` to show/set flags (e.g., `eval-config +KING_SAFETY -THREATS`)
- UCI option `EvalFlags` for GUI integration (e.g., `setoption name EvalFlags value KING_SAFETY,PAWN_STRUCTURE`)
- Material + PST evaluation always enabled; all other heuristics can be toggled
- Added tests in `tests/board/eval_config_tests.rs`

26-01-2026
Implemented staged move generation for more efficient search.
- Added `move_picker.rs` with `MovePicker` that picks moves in stages (TT → captures → killers → quiets)
- Avoids scoring/sorting moves that are never searched due to early beta cutoffs
- Separates good captures (SEE >= 0) from bad captures for better ordering
- Replaced `order_moves` with staged picker in alpha-beta search
- Added `get_killers` method to `SearchHeuristics` for move picker integration
- Added tests for move picker

26-01-2026
Implemented Lazy SMP parallel search for ~39% performance improvement (447k → 620k nps).
- Added `lazy_smp.rs` module with multi-threaded search using shared transposition table
- Modified `SearchContext` to use `Arc<TranspositionTable>` for TT sharing across threads
- Spawns up to 3 helper threads that search at different depths for diversity
- Lowered root parallel thresholds (depth 6→4, moves 4→3, time 200→50ms)
- Added tests in lazy_smp.rs

25-01-2026
Added closed/open position awareness for knight vs bishop evaluation.
- Knights receive bonus in closed positions (many blocked pawns)
- Bishops receive bonus in open positions (few pawns)
- Position openness calculated from total pawns and blocked pawn count

25-01-2026
Added safe mobility evaluation for all pieces (knights, bishops, rooks, queens).
- Pieces receive bonus/penalty based on safe squares (not attacked by enemy pawns)
- Bishops receive mobility scoring only when developed (not on home squares)
- Fixed knight outpost taper_general bug (was passing phase as mg value)

25-01-2026
Added king ring pressure scoring for unsafe attacks around the king.
Added tests in:
- evaluate_king_tests.rs

25-01-2026
Refined root ordering to keep TT/PV moves first and added conservative pruning for late quiet moves.
Added tests in:
- root_evaluator_tests.rs

25-01-2026
Tuned aspiration windows to adapt to score volatility across depths.
Added a full-width verification when deep searches succeed inside a tight window.
Added tests in:
- aspiration_tests.rs

25-01-2026
Added shallow check extensions for in-check nodes and checking moves, capped per line to prevent unbounded recursion.
Added tests in:
- alphabeta_tests.rs

25-01-2026
Added singular extensions for TT best moves with a margin-based alternative search.
Added tests in:
- alphabeta_tests.rs

25-01-2026
Added late-move pruning for quiet moves at low depth with non-PV/in-check guards.
Added tests in:
- alphabeta_tests.rs

25-01-2026
Added shallow reverse futility pruning and razoring to reduce low-value branches.
Restricted razoring/RFP to non-PV nodes away from root with an added ply/depth guard to avoid tactical misses.
Added tests in:
- alphabeta_tests.rs

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
