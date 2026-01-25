Codex Review

Here are the highest‑impact ways to improve strength, roughly ordered by typical Elo gain per effort. Pick a direction and I’ll propose a concrete plan.

High impact
[X] Search improvements: add/re-tune pruning (RFP/razoring/LMP), smarter LMR, singular extensions, better null‑move conditions; refine move ordering (counter‑move, history/continuation history, improved SEE).
[-] Evaluation tuning: retune existing eval weights (Texel tuning), add missing terms (king safety nuance, pawn structure, passed‑pawn races, bishop pair/rook activity scaling, initiative/tempo scaling).
[-] Endgame knowledge: integrate Syzygy tablebases (5‑6‑piece) for perfect endgames.

Medium impact
[ ] Quiescence: better SEE pruning, include safe checks/recaptures, reduce tactical noise.
[ ] Root heuristics: calibrate or prune root adjustments so they don’t override search; ensure symmetry/scale.

Optional bigger bets
[-] NNUE evaluation (bigger change but big strength gain).
[-] Opening book improvements or Polyglot integration.

Which direction do you want to pursue first? If you want, I can prioritize by effort vs. expected gain and start with a concrete change list.
