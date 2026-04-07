# cli-games

A collection of 10 classic terminal mini-games written in Rust, using ratatui for TUI rendering and crossterm for input handling. All games run directly in your terminal with no GUI dependencies.

## Games

| # | Game | Description |
|---|------|-------------|
| 1 | **Snake** | Classic snake game — eat food, grow longer, don't hit yourself |
| 2 | **Tetris** | Falling block puzzle with rotation, scoring, and increasing speed |
| 3 | **Pong** | Two-player (or vs AI) paddle game |
| 4 | **Minesweeper** | Grid-based mine detection with flag support |
| 5 | **Blackjack** | Casino card game — hit, stand, and try to beat the dealer |
| 6 | **Hangman** | Word guessing game with ASCII art |
| 7 | **Wordle** | 5-letter word guessing with color-coded feedback |
| 8 | **2048** | Sliding tile puzzle — merge numbers to reach 2048 |
| 9 | **Simon** | Memory pattern game — repeat the growing sequence |
| 10 | **Typing Test** | Test your typing speed and accuracy |

## Installation

```bash
# Clone and build from source
git clone https://github.com/nicovlr/cli-games.git
cd cli-games
cargo build --release

# Run the game launcher
./target/release/cli-games
```

## Usage

```bash
# Launch the interactive game menu
cli-games

# Play a specific game directly
cli-games --game snake
cli-games --game tetris
cli-games --game pong
cli-games --game minesweeper
cli-games --game blackjack
cli-games --game hangman
cli-games --game wordle
cli-games --game 2048
cli-games --game simon
cli-games --game typing
```

## Tech Stack

- **Language:** Rust (2021 edition)
- **TUI framework:** [ratatui](https://crates.io/crates/ratatui) v0.29
- **Terminal backend:** [crossterm](https://crates.io/crates/crossterm) v0.28
- **CLI parsing:** [clap](https://crates.io/crates/clap) v4
- **Randomness:** [rand](https://crates.io/crates/rand) v0.8

## Project Structure

```
src/
├── main.rs          → Entry point, game launcher menu
├── common/          → Shared utilities (rendering, input, etc.)
└── games/
    ├── snake.rs
    ├── tetris.rs
    ├── pong.rs
    ├── minesweeper.rs
    ├── blackjack.rs
    ├── hangman.rs
    ├── wordle.rs
    ├── twenty48.rs
    ├── simon.rs
    └── typing_test.rs
```

## Requirements

- Rust 1.70+ (2021 edition)
- A terminal that supports ANSI escape codes (most modern terminals)
- No external runtime dependencies

## License

MIT
