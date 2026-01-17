please Here are the orthogonal aspects of your chess engine that can be reviewed separately:

please review the <aspect> aspect of the chess engine. See <main-file> for a starting point


Alpha-Beta/PVS,src/search/core/alphabeta.rs results: 

├───────────────────────────────────────────────────┼──────────────┤
│ Passed pawn extension                             │ ✓ Reasonable │
└───────────────────────────────────────────────────┴──────────────┘
  ---
Summary
┌──────────┬───────────────────────────────────────────┬────────────────────────┐
│ Severity │                   Issue                   │        Location        │
├──────────┼───────────────────────────────────────────┼────────────────────────┤
│ High     │ Mate scoring uses depth instead of ply    │ Line 127               │  FIXED
├──────────┼───────────────────────────────────────────┼────────────────────────┤
│ Medium   │ Dead killer move code in else-if branches │ Lines 279-281, 294-296 │
├──────────┼───────────────────────────────────────────┼────────────────────────┤
│ Low      │ Linear repetition stack search            │ Line 54                │
├──────────┼───────────────────────────────────────────┼────────────────────────┤
│ Info     │ Non-standard quiet move definition        │ Lines 229-230          │
└──────────┴───────────────────────────────────────────┴────────────────────────┘

Aspect,Main File
----------------
Cycle (reviewed/fixed/tested)
1 Alpha-Beta/PVS,src/search/core/alphabeta.rs
1 Quiescence Search,src/search/core/qsearch.rs
1 Iterative Deepening,src/search/core/advanced_search.rs
1 Aspiration Windows,src/search/management/aspiration.rs
1 Null Move Pruning,src/search/management/prune_null_moves.rs
1 Late Move Reduction,src/search/core/alphabeta.rs
0 Futility Pruning,src/search/core/qsearch.rs
0 Delta Pruning,src/search/core/qsearch.rs
0 SEE Pruning,src/board/see.rs
0 TT Move Priority,src/search/core/alphabeta.rs
0 MVV-LVA,src/search/core/alphabeta.rs
0 Killer Moves,src/search/evaluation/heuristics.rs
0 History Heuristic,src/search/evaluation/heuristics.rs
0 Transposition Table,src/search/state/tt.rs
0 Zobrist Hashing,src/search/state/zobrist.rs
0 Material + PST,src/board/evaluators/pst.rs
0 Pawn Structure,src/board/evaluators/evaluate_pawns.rs
0 Knight Evaluation,src/board/evaluators/evaluate_knights.rs
0 Bishop Evaluation,src/board/evaluators/evaluate_bishops.rs
0 Rook Evaluation,src/board/evaluators/evaluate_rooks.rs
0 Queen Evaluation,src/board/evaluators/evaluate_queens.rs
0 King Safety,src/board/evaluators/evaluate_king.rs
0 Mobility & Activity,src/board/evaluator.rs
0 Hanging Pieces,src/board/evaluator.rs
0 Threat Resolution,src/search/evaluation/root_heuristics/threat_resolution.rs
0 Knight Evacuation,src/search/evaluation/root_heuristics/knight_evacuation.rs
0 Repetition Avoidance,src/search/evaluation/repetition.rs
0 Move Generation,src/search/management/move_generator.rs
0 Board Representation,src/board/board.rs
0 Attack Detection,src/board/checks/square_attacked.rs
0 Time Management,src/search/integration/time_control.rs
0 Opening Book,src/book/book.rs
0 UCI Protocol,src/uci/mod.rs
0 Parallel Search,src/search/integration/threading.rs
0 Playing Strength,src/search/integration/playing_strength.rs

More aspects:

0 Piece development
0 Personalization, use persona's that define parameters that affect playing style


Search Algorithms
┌─────────────────────┬─────────────────────────────────┬─────────────────────────────────────────────┐
│       Aspect        │             File(s)             │                 Description                 │
├─────────────────────┼─────────────────────────────────┼─────────────────────────────────────────────┤
│ Alpha-Beta/PVS      │ search/core/alphabeta.rs        │ Principal Variation Search with null-window │
├─────────────────────┼─────────────────────────────────┼─────────────────────────────────────────────┤
│ Quiescence Search   │ search/core/qsearch.rs          │ Tactical continuation at leaf nodes         │
├─────────────────────┼─────────────────────────────────┼─────────────────────────────────────────────┤
│ Iterative Deepening │ search/core/advanced_search.rs  │ Progressive depth increases                 │
├─────────────────────┼─────────────────────────────────┼─────────────────────────────────────────────┤
│ Aspiration Windows  │ search/management/aspiration.rs │ ±30cp initial window with expansion         │
└─────────────────────┴─────────────────────────────────┴─────────────────────────────────────────────┘
Pruning Techniques
┌─────────────────────┬───────────────────────────────────────┬───────────────────────────────────────────┐
│       Aspect        │                File(s)                │                Description                │
├─────────────────────┼───────────────────────────────────────┼───────────────────────────────────────────┤
│ Null Move Pruning   │ search/management/prune_null_moves.rs │ Skip turn to prove position strength      │
├─────────────────────┼───────────────────────────────────────┼───────────────────────────────────────────┤
│ Late Move Reduction │ alphabeta.rs:442-508                  │ Reduce depth on quiet moves after move 3  │
├─────────────────────┼───────────────────────────────────────┼───────────────────────────────────────────┤
│ Futility Pruning    │ qsearch.rs                            │ 40cp margin cutoff in quiescence          │
├─────────────────────┼───────────────────────────────────────┼───────────────────────────────────────────┤
│ Delta Pruning       │ qsearch.rs                            │ 120cp threshold for unlikely improvements │
├─────────────────────┼───────────────────────────────────────┼───────────────────────────────────────────┤
│ SEE Pruning         │ qsearch.rs + see.rs                   │ Static Exchange Evaluation filtering      │
└─────────────────────┴───────────────────────────────────────┴───────────────────────────────────────────┘
Move Ordering
┌───────────────────┬─────────────────────────────────┬────────────────────────────────────────────┐
│      Aspect       │             File(s)             │                Description                 │
├───────────────────┼─────────────────────────────────┼────────────────────────────────────────────┤
│ TT Move Priority  │ alphabeta.rs                    │ Best move from transposition table first   │
├───────────────────┼─────────────────────────────────┼────────────────────────────────────────────┤
│ MVV-LVA           │ alphabeta.rs:405-410            │ Capture ordering by victim/attacker value  │
├───────────────────┼─────────────────────────────────┼────────────────────────────────────────────┤
│ Killer Moves      │ search/evaluation/heuristics.rs │ 2 killers per ply, +200k bonus             │
├───────────────────┼─────────────────────────────────┼────────────────────────────────────────────┤
│ History Heuristic │ search/evaluation/heuristics.rs │ Quadratic depth bonus for good quiet moves │
└───────────────────┴─────────────────────────────────┴────────────────────────────────────────────┘
Transposition & Hashing
┌─────────────────────┬─────────────────────────┬─────────────────────────────────────┐
│       Aspect        │         File(s)         │             Description             │
├─────────────────────┼─────────────────────────┼─────────────────────────────────────┤
│ Transposition Table │ search/state/tt.rs      │ 2^21 entries, age-based replacement │
├─────────────────────┼─────────────────────────┼─────────────────────────────────────┤
│ Zobrist Hashing     │ search/state/zobrist.rs │ 64-bit incremental position keys    │
└─────────────────────┴─────────────────────────┴─────────────────────────────────────┘
Evaluation
┌─────────────────────┬─────────────────────────────────────────────┬──────────────────────────────────────────────┐
│       Aspect        │                   File(s)                   │                 Description                  │
├─────────────────────┼─────────────────────────────────────────────┼──────────────────────────────────────────────┤
│ Material + PST      │ board/evaluator.rs, board/evaluators/pst.rs │ Tapered piece values with square tables      │
├─────────────────────┼─────────────────────────────────────────────┼──────────────────────────────────────────────┤
│ Pawn Structure      │ evaluators/evaluate_pawns.rs                │ Doubled, isolated, backward, passed pawns    │
├─────────────────────┼─────────────────────────────────────────────┼──────────────────────────────────────────────┤
│ Knight Evaluation   │ evaluators/evaluate_knights.rs              │ Outposts, mobility                           │
├─────────────────────┼─────────────────────────────────────────────┼──────────────────────────────────────────────┤
│ Bishop Evaluation   │ evaluators/evaluate_bishops.rs              │ Pair bonus, color control                    │
├─────────────────────┼─────────────────────────────────────────────┼──────────────────────────────────────────────┤
│ Rook Evaluation     │ evaluators/evaluate_rooks.rs                │ Open files, 7th rank, doubled rooks          │
├─────────────────────┼─────────────────────────────────────────────┼──────────────────────────────────────────────┤
│ Queen Evaluation    │ evaluators/evaluate_queens.rs               │ Kingside pressure, early development penalty │
├─────────────────────┼─────────────────────────────────────────────┼──────────────────────────────────────────────┤
│ King Safety         │ evaluators/evaluate_king.rs                 │ Pawn shield, enemy proximity, cornering      │
├─────────────────────┼─────────────────────────────────────────────┼──────────────────────────────────────────────┤
│ Mobility & Activity │ evaluator.rs:403-423                        │ Piece mobility scaled by phase               │
├─────────────────────┼─────────────────────────────────────────────┼──────────────────────────────────────────────┤
│ Hanging Pieces      │ evaluator.rs:376-401                        │ Penalty for undefended attacked pieces       │
└─────────────────────┴─────────────────────────────────────────────┴──────────────────────────────────────────────┘
Root-Level Heuristics
┌──────────────────────┬──────────────────────────────────────┬───────────────────────────────────────┐
│        Aspect        │               File(s)                │              Description              │
├──────────────────────┼──────────────────────────────────────┼───────────────────────────────────────┤
│ Threat Resolution    │ root_heuristics/threat_resolution.rs │ Penalize leaving pieces hanging       │
├──────────────────────┼──────────────────────────────────────┼───────────────────────────────────────┤
│ Knight Evacuation    │ root_heuristics/knight_evacuation.rs │ Prioritize moving attacked knights    │
├──────────────────────┼──────────────────────────────────────┼───────────────────────────────────────┤
│ Repetition Avoidance │ search/evaluation/repetition.rs      │ -2000cp penalty for 3-fold if winning │
└──────────────────────┴──────────────────────────────────────┴───────────────────────────────────────┘
Infrastructure
┌──────────────────────┬────────────────────────────────────────┬─────────────────────────────────────┐
│        Aspect        │                File(s)                 │             Description             │
├──────────────────────┼────────────────────────────────────────┼─────────────────────────────────────┤
│ Move Generation      │ search/management/move_generator.rs    │ Legal move enumeration              │
├──────────────────────┼────────────────────────────────────────┼─────────────────────────────────────┤
│ Board Representation │ board/board.rs, state/game_state.rs    │ 8×8 array (not bitboards)           │
├──────────────────────┼────────────────────────────────────────┼─────────────────────────────────────┤
│ Attack Detection     │ board/checks/square_attacked.rs        │ Brute-force piece scanning          │
├──────────────────────┼────────────────────────────────────────┼─────────────────────────────────────┤
│ Time Management      │ search/integration/time_control.rs     │ Deadline-based budget               │
├──────────────────────┼────────────────────────────────────────┼─────────────────────────────────────┤
│ Opening Book         │ book/book.rs                           │ Weighted random from static entries │
├──────────────────────┼────────────────────────────────────────┼─────────────────────────────────────┤
│ UCI Protocol         │ uci/mod.rs                             │ Standard UCI interface              │
├──────────────────────┼────────────────────────────────────────┼─────────────────────────────────────┤
│ Parallel Search      │ search/integration/threading.rs        │ Rayon-based root parallelization    │
├──────────────────────┼────────────────────────────────────────┼─────────────────────────────────────┤
│ Playing Strength     │ search/integration/playing_strength.rs │ Blunder injection for weaker play   │
└──────────────────────┴────────────────────────────────────────┴─────────────────────────────────────┘
