use crossterm::event::{KeyCode, KeyEvent};
use rand::seq::SliceRandom;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::games::Game;

// ---------------------------------------------------------------------------
// Word list
// ---------------------------------------------------------------------------

const WORDS: &[&str] = &[
    "abandon", "bicycle", "cascade", "dolphin", "eclipse",
    "flannel", "gravity", "horizon", "impulse", "jasmine",
    "kingdom", "lantern", "mystery", "nucleus", "orchard",
    "phantom", "quarrel", "rainbow", "sailing", "thunder",
    "uranium", "volcano", "whisper", "anxiety", "zephyr",
    "archive", "buffalo", "chimney", "digital", "embrace",
    "fervent", "glimpse", "hamster", "Integer", "journal",
    "ketchup", "luggage", "mammoth", "natural", "opinion",
    "pancake", "quantum", "reactor", "stadium", "textile",
    "upright", "venture", "weather", "paradox", "fiction",
];

// ---------------------------------------------------------------------------
// ASCII art for the gallows (6 error stages + empty)
// ---------------------------------------------------------------------------

/// Returns the hangman ASCII art lines for a given number of errors (0..=6).
fn hangman_art(errors: u8) -> [&'static str; 9] {
    match errors {
        0 => [
            "  ┌──────┐ ",
            "  │      │ ",
            "  │        ",
            "  │        ",
            "  │        ",
            "  │        ",
            "  │        ",
            "══╧══════  ",
            "           ",
        ],
        1 => [
            "  ┌──────┐ ",
            "  │      │ ",
            "  │      O ",
            "  │        ",
            "  │        ",
            "  │        ",
            "  │        ",
            "══╧══════  ",
            "           ",
        ],
        2 => [
            "  ┌──────┐ ",
            "  │      │ ",
            "  │      O ",
            "  │      │ ",
            "  │      │ ",
            "  │        ",
            "  │        ",
            "══╧══════  ",
            "           ",
        ],
        3 => [
            "  ┌──────┐ ",
            "  │      │ ",
            "  │      O ",
            "  │     /│ ",
            "  │      │ ",
            "  │        ",
            "  │        ",
            "══╧══════  ",
            "           ",
        ],
        4 => [
            "  ┌──────┐ ",
            "  │      │ ",
            "  │      O ",
            "  │     /│\\",
            "  │      │ ",
            "  │        ",
            "  │        ",
            "══╧══════  ",
            "           ",
        ],
        5 => [
            "  ┌──────┐ ",
            "  │      │ ",
            "  │      O ",
            "  │     /│\\",
            "  │      │ ",
            "  │     /  ",
            "  │        ",
            "══╧══════  ",
            "           ",
        ],
        _ => [
            "  ┌──────┐ ",
            "  │      │ ",
            "  │      O ",
            "  │     /│\\",
            "  │      │ ",
            "  │     / \\",
            "  │        ",
            "══╧══════  ",
            "           ",
        ],
    }
}

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameState {
    Playing,
    Won,
    Lost,
}

pub struct HangmanGame {
    /// The secret word (lowercased).
    word: String,
    /// Set of letters the player has guessed so far.
    guessed: Vec<char>,
    /// Number of incorrect guesses.
    errors: u8,
    /// Current game state.
    state: GameState,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_ERRORS: u8 = 6;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Pick a random word from the word list.
fn pick_word() -> String {
    let mut rng = rand::thread_rng();
    WORDS
        .choose(&mut rng)
        .expect("word list is not empty")
        .to_lowercase()
}

/// Build the masked display string: revealed letters and underscores.
fn masked_word(word: &str, guessed: &[char]) -> String {
    word.chars()
        .map(|c| {
            if guessed.contains(&c) {
                c
            } else {
                '_'
            }
        })
        .collect::<Vec<char>>()
        .chunks(1)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<String>>()
        .join(" ")
}

/// Check whether every letter in the word has been guessed.
fn is_word_revealed(word: &str, guessed: &[char]) -> bool {
    word.chars().all(|c| guessed.contains(&c))
}

/// Return a `Rect` of size `w x h` centred inside `area`.
fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

impl Game for HangmanGame {
    fn new() -> Self {
        HangmanGame {
            word: pick_word(),
            guessed: Vec::new(),
            errors: 0,
            state: GameState::Playing,
        }
    }

    fn handle_event(&mut self, event: KeyEvent) -> bool {
        match self.state {
            GameState::Won | GameState::Lost => {
                match event.code {
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        *self = HangmanGame::new();
                        return true;
                    }
                    KeyCode::Esc => return false,
                    _ => return true,
                }
            }
            GameState::Playing => {
                match event.code {
                    KeyCode::Esc => return false,
                    KeyCode::Char(c) if c.is_ascii_alphabetic() => {
                        let ch = c.to_ascii_lowercase();
                        if !self.guessed.contains(&ch) {
                            self.guessed.push(ch);
                            if !self.word.contains(ch) {
                                self.errors += 1;
                            }
                            // Check win/lose conditions.
                            if is_word_revealed(&self.word, &self.guessed) {
                                self.state = GameState::Won;
                            } else if self.errors >= MAX_ERRORS {
                                self.state = GameState::Lost;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        true
    }

    fn update(&mut self) {
        // Hangman is purely event-driven; nothing to do on tick.
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Outer border.
        let outer_block = Block::default()
            .title(" Hangman ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Split into left (gallows) and right (word + info) panels.
        let columns = Layout::horizontal([
            Constraint::Length(16),
            Constraint::Min(20),
        ])
        .split(inner);

        self.render_gallows(frame, columns[0]);
        self.render_info_panel(frame, columns[1]);

        // Overlay for win/lose.
        match self.state {
            GameState::Won => self.render_overlay(frame, area, true),
            GameState::Lost => self.render_overlay(frame, area, false),
            GameState::Playing => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

impl HangmanGame {
    /// Render the ASCII-art gallows on the left panel.
    fn render_gallows(&self, frame: &mut Frame, area: Rect) {
        let art = hangman_art(self.errors);
        let lines: Vec<Line> = art
            .iter()
            .map(|line| {
                Line::from(Span::styled(
                    *line,
                    Style::default().fg(Color::White),
                ))
            })
            .collect();

        let gallows = Paragraph::new(lines).block(
            Block::default()
                .title(" Gallows ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(gallows, area);
    }

    /// Render the right panel: word, guessed letters, remaining attempts, help.
    fn render_info_panel(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(5),  // Word display
            Constraint::Length(7),  // Guessed letters
            Constraint::Length(4),  // Attempts remaining
            Constraint::Min(3),    // Help text
        ])
        .split(area);

        // --- Word display ---
        let display = masked_word(&self.word, &self.guessed);
        let word_color = match self.state {
            GameState::Won => Color::Green,
            GameState::Lost => Color::Red,
            GameState::Playing => Color::Yellow,
        };
        let word_paragraph = Paragraph::new(Line::from(Span::styled(
            display,
            Style::default()
                .fg(word_color)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(" Word ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        frame.render_widget(word_paragraph, chunks[0]);

        // --- Guessed letters ---
        let mut correct: Vec<char> = Vec::new();
        let mut wrong: Vec<char> = Vec::new();
        for &ch in &self.guessed {
            if self.word.contains(ch) {
                correct.push(ch);
            } else {
                wrong.push(ch);
            }
        }
        correct.sort();
        wrong.sort();

        let correct_str: String = correct.iter().map(|c| format!("{} ", c)).collect();
        let wrong_str: String = wrong.iter().map(|c| format!("{} ", c)).collect();

        let guessed_lines = vec![
            Line::from(vec![
                Span::styled("Correct: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(
                    if correct_str.is_empty() { "-".to_string() } else { correct_str },
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Wrong:   ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(
                    if wrong_str.is_empty() { "-".to_string() } else { wrong_str },
                    Style::default().fg(Color::Red),
                ),
            ]),
        ];

        let guessed_paragraph = Paragraph::new(guessed_lines).block(
            Block::default()
                .title(" Guessed Letters ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );
        frame.render_widget(guessed_paragraph, chunks[1]);

        // --- Attempts remaining ---
        let remaining = MAX_ERRORS.saturating_sub(self.errors);
        let bar_len = remaining as usize;
        let bar: String = "█".repeat(bar_len) + &"░".repeat((MAX_ERRORS as usize).saturating_sub(bar_len));
        let bar_color = match remaining {
            0 => Color::Red,
            1..=2 => Color::LightRed,
            3..=4 => Color::Yellow,
            _ => Color::Green,
        };

        let attempts_lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("  {} / {} ", remaining, MAX_ERRORS),
                    Style::default().fg(bar_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("[{}]", bar),
                    Style::default().fg(bar_color),
                ),
            ]),
        ];

        let attempts_paragraph = Paragraph::new(attempts_lines).block(
            Block::default()
                .title(" Attempts Left ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );
        frame.render_widget(attempts_paragraph, chunks[2]);

        // --- Help text ---
        let help_lines = vec![
            Line::from(Span::styled(
                "Type a-z to guess a letter",
                Style::default().fg(Color::Gray),
            )),
            Line::from(Span::styled(
                "Esc to quit",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let help_paragraph = Paragraph::new(help_lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" Help ")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(help_paragraph, chunks[3]);
    }

    /// Render a centred win or lose overlay.
    fn render_overlay(&self, frame: &mut Frame, area: Rect, won: bool) {
        let popup_w: u16 = 44.min(area.width.saturating_sub(4));
        let popup_h: u16 = 9.min(area.height.saturating_sub(4));
        let popup = centered_rect(popup_w, popup_h, area);

        frame.render_widget(Clear, popup);

        let (title, msg, color) = if won {
            (" You Win! ", "YOU WIN!", Color::Green)
        } else {
            (" Game Over ", "GAME OVER", Color::Red)
        };

        let lines = vec![
            Line::from(Span::styled(
                msg,
                Style::default()
                    .fg(color)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("The word was: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    self.word.to_uppercase(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'r' to restart or Esc to quit",
                Style::default().fg(Color::Gray),
            )),
        ];

        let paragraph = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(color))
                    .title(title)
                    .title_alignment(Alignment::Center),
            );

        frame.render_widget(paragraph, popup);
    }
}
