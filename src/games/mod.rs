pub mod snake;
pub mod hangman;
pub mod wordle;
pub mod twenty48;
pub mod blackjack;
pub mod minesweeper;
pub mod tetris;
pub mod pong;
pub mod typing_test;
pub mod simon;

use crossterm::event::KeyEvent;
use ratatui::Frame;

pub trait Game {
    fn new() -> Self where Self: Sized;
    fn handle_event(&mut self, event: KeyEvent) -> bool;
    fn update(&mut self);
    fn render(&self, frame: &mut Frame);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameChoice {
    Snake,
    Hangman,
    Wordle,
    Twenty48,
    Blackjack,
    Minesweeper,
    Tetris,
    Pong,
    TypingTest,
    Simon,
}

impl GameChoice {
    pub const ALL: [GameChoice; 10] = [
        GameChoice::Snake,
        GameChoice::Hangman,
        GameChoice::Wordle,
        GameChoice::Twenty48,
        GameChoice::Blackjack,
        GameChoice::Minesweeper,
        GameChoice::Tetris,
        GameChoice::Pong,
        GameChoice::TypingTest,
        GameChoice::Simon,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            GameChoice::Snake => "Snake",
            GameChoice::Hangman => "Hangman",
            GameChoice::Wordle => "Wordle",
            GameChoice::Twenty48 => "2048",
            GameChoice::Blackjack => "Blackjack",
            GameChoice::Minesweeper => "Minesweeper",
            GameChoice::Tetris => "Tetris",
            GameChoice::Pong => "Pong",
            GameChoice::TypingTest => "Typing Test",
            GameChoice::Simon => "Simon",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            GameChoice::Snake => "Classic snake game - eat apples, grow longer!",
            GameChoice::Hangman => "Guess the word before the hangman is complete",
            GameChoice::Wordle => "Guess the 5-letter word in 6 tries",
            GameChoice::Twenty48 => "Slide and merge tiles to reach 2048",
            GameChoice::Blackjack => "Beat the dealer at 21",
            GameChoice::Minesweeper => "Clear the minefield without hitting a mine",
            GameChoice::Tetris => "Stack and clear lines with falling pieces",
            GameChoice::Pong => "Classic paddle ball game vs CPU",
            GameChoice::TypingTest => "Test your typing speed and accuracy",
            GameChoice::Simon => "Memorize and repeat the color sequence",
        }
    }
}
