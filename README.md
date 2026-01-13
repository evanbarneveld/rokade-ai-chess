Rokade-AI Chess — UCI Chess Engine

This repository contains a chess engine written (mostly) by AI in Rust. 
Expect rough edges, active iteration, and frequent improvements.

Disclaimer: the code in this project does not reflect what I (evanbarneveld) consider clean code.
A lot of refactoring (by AI) is required to clean up the code and make it more readable/maintainable.

<img src="https://github.com/evanbarneveld/rokade-ai-chess/blob/main/rokade-ai-chess.png?raw=true" alt="Rokade-Ai Chess" style="width:30%; height:auto;">

This engine is a single executable and has a command line interface. 
Start the engine 'rokade-ai-chess' and try the 'help' command for a list of commands.

(When used as an UCI engine by a chess GUI, the GUI will send the 'uci' command to the engine)

This engine works with Arena, Cute Chess, En-Croissant, Hiarcs Chess, and other UCI-compatible Chess GUIs.
The engine logs to 'rokade-ai-chess.#####.log' in the current directory.

Highlights
- Written in Rust, with a modular layout (board, move generation, search, parsing, UCI/CLI, etc.)
- Parallel search via Rayon
- CLI entry point for playing/running locally
- PGN parsing utilities and test suites

Project Status
- Work in progress: playable, working, but strength and stability are evolving.
- Learning-focused: learning how to co-engineer with AI is prioritized over ultimate engine strength (for now).
- AI-assisted development: significant parts were generated or refactored with AI help.

Why this exists
- Provide a playable engine for local play and its source code. 
- For me: learning the Rust language and explore chess engine architecture and search heuristics.

Getting Started

Prerequisites
- Rust toolchain (rustup + cargo)

Build
```
cargo build
```

The default binary runs a simple CLI:
```
cargo run --release
```

Tests
- Run the full test suite:
```
cargo test -- --nocapture
```

Known Issues
- Deterministic mode still shows move variations (non-determinism) in some runs.

Roadmap
- Improve heuristics with targeted tests for poor moves.
- Make the engine play a decent endgame.

Contributing
This is primarily a learning project, but suggestions, bug reports, and PRs are welcome. Please keep the learning spirit and AI-assisted approach in mind when proposing changes.

License
This project is licensed under the MIT License. You are free to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the software under the terms of the MIT license. See the LICENSE file for the full text.
