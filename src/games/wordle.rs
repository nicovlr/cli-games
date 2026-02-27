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
// Word list (~100 common 5-letter English words)
// ---------------------------------------------------------------------------

const WORDS: &[&str] = &[
    "about", "above", "abuse", "actor", "acute", "admit", "adopt", "adult",
    "after", "again", "agent", "agree", "ahead", "alarm", "album", "alert",
    "alien", "align", "alive", "allow", "alone", "alter", "angel", "anger",
    "angle", "angry", "apart", "apple", "apply", "arena", "arise", "armor",
    "array", "aside", "asset", "avoid", "awake", "award", "aware", "basic",
    "beach", "being", "below", "bench", "birth", "blade", "blame", "blank",
    "blast", "blaze", "bleed", "blend", "blind", "block", "blood", "bloom",
    "blown", "board", "bonus", "bound", "brain", "brand", "brave", "bread",
    "break", "breed", "brick", "brief", "bring", "broad", "brown", "brush",
    "build", "built", "burst", "buyer", "cabin", "chain", "chair", "chalk",
    "charm", "chase", "cheap", "check", "chess", "chief", "child", "claim",
    "clash", "class", "clean", "clear", "climb", "cling", "clock", "close",
    "cloud", "coach", "coral", "count", "cover", "craft", "crane", "crash",
    "crazy", "cream", "crest", "crime", "crisp", "cross", "crowd", "crown",
    "crush", "curve", "cycle",
];

// ---------------------------------------------------------------------------
// Tile feedback
// ---------------------------------------------------------------------------

/// Feedback for a single letter in a submitted guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LetterStatus {
    /// Correct letter in correct position (green).
    Correct,
    /// Correct letter in wrong position (yellow).
    WrongPosition,
    /// Letter not in the word (gray).
    NotInWord,
}

impl LetterStatus {
    fn color(self) -> Color {
        match self {
            LetterStatus::Correct => Color::Green,
            LetterStatus::WrongPosition => Color::Yellow,
            LetterStatus::NotInWord => Color::DarkGray,
        }
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

/// A submitted guess: 5 letters with their feedback.
#[derive(Debug, Clone)]
struct GuessRow {
    letters: [char; 5],
    status: [LetterStatus; 5],
}

pub struct WordleGame {
    /// The secret word (lowercased).
    secret: String,
    /// Previously submitted guesses.
    guesses: Vec<GuessRow>,
    /// Current input buffer (0..=5 chars).
    current_input: Vec<char>,
    /// Current game state.
    state: GameState,
    /// Keyboard letter statuses: best status for each letter a-z.
    keyboard: [Option<LetterStatus>; 26],
    /// Error message to display briefly (e.g. "Not in word list").
    message: Option<String>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_GUESSES: usize = 6;
const WORD_LENGTH: usize = 5;

const KEYBOARD_ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Pick a random word from the word list.
fn pick_word() -> String {
    let mut rng = rand::thread_rng();
    WORDS
        .choose(&mut rng)
        .expect("word list is not empty")
        .to_string()
}

/// Evaluate a guess against the secret word and return per-letter feedback.
/// Uses the standard Wordle algorithm:
/// 1. First pass: mark exact matches (Correct).
/// 2. Second pass: for remaining letters, mark WrongPosition if the letter
///    exists in the secret (respecting remaining counts).
fn evaluate_guess(guess: &[char; 5], secret: &str) -> [LetterStatus; 5] {
    let secret_chars: Vec<char> = secret.chars().collect();
    let mut result = [LetterStatus::NotInWord; 5];
    let mut secret_remaining = [0u8; 26]; // count of unmatched secret letters

    // Count all letters in the secret.
    for &c in &secret_chars {
        let idx = (c as u8 - b'a') as usize;
        secret_remaining[idx] += 1;
    }

    // First pass: exact matches.
    for i in 0..5 {
        if guess[i] == secret_chars[i] {
            result[i] = LetterStatus::Correct;
            let idx = (guess[i] as u8 - b'a') as usize;
            secret_remaining[idx] -= 1;
        }
    }

    // Second pass: wrong position.
    for i in 0..5 {
        if result[i] == LetterStatus::Correct {
            continue;
        }
        let idx = (guess[i] as u8 - b'a') as usize;
        if secret_remaining[idx] > 0 {
            result[i] = LetterStatus::WrongPosition;
            secret_remaining[idx] -= 1;
        }
    }

    result
}

/// Return a `Rect` of size `w x h` centred inside `area`.
fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}

/// Check whether a word is in the valid word list.
fn is_valid_word(word: &str) -> bool {
    WORDS.contains(&word)
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

impl Game for WordleGame {
    fn new() -> Self {
        WordleGame {
            secret: pick_word(),
            guesses: Vec::new(),
            current_input: Vec::new(),
            state: GameState::Playing,
            keyboard: [None; 26],
            message: None,
        }
    }

    fn handle_event(&mut self, event: KeyEvent) -> bool {
        // Clear any transient message on any key press.
        self.message = None;

        match self.state {
            GameState::Won | GameState::Lost => match event.code {
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    *self = WordleGame::new();
                    true
                }
                KeyCode::Esc => false,
                _ => true,
            },
            GameState::Playing => {
                match event.code {
                    KeyCode::Esc => return false,
                    KeyCode::Char(c) if c.is_ascii_alphabetic() => {
                        if self.current_input.len() < WORD_LENGTH {
                            self.current_input.push(c.to_ascii_lowercase());
                        }
                    }
                    KeyCode::Backspace => {
                        self.current_input.pop();
                    }
                    KeyCode::Enter => {
                        if self.current_input.len() == WORD_LENGTH {
                            let word: String = self.current_input.iter().collect();

                            // Validate word is in the list.
                            if !is_valid_word(&word) {
                                self.message = Some("Not in word list!".to_string());
                                return true;
                            }

                            // Build guess array.
                            let mut letters = ['a'; 5];
                            for (i, &c) in self.current_input.iter().enumerate() {
                                letters[i] = c;
                            }

                            let status = evaluate_guess(&letters, &self.secret);

                            // Update keyboard tracking.
                            for i in 0..5 {
                                let idx = (letters[i] as u8 - b'a') as usize;
                                let new_status = status[i];
                                let current = self.keyboard[idx];
                                // Upgrade: Correct > WrongPosition > NotInWord.
                                let should_update = match current {
                                    None => true,
                                    Some(LetterStatus::NotInWord) => {
                                        new_status != LetterStatus::NotInWord
                                    }
                                    Some(LetterStatus::WrongPosition) => {
                                        new_status == LetterStatus::Correct
                                    }
                                    Some(LetterStatus::Correct) => false,
                                };
                                if should_update {
                                    self.keyboard[idx] = Some(new_status);
                                }
                            }

                            self.guesses.push(GuessRow { letters, status });
                            self.current_input.clear();

                            // Check win/lose.
                            let last = self.guesses.last().unwrap();
                            if last.status.iter().all(|s| *s == LetterStatus::Correct) {
                                self.state = GameState::Won;
                            } else if self.guesses.len() >= MAX_GUESSES {
                                self.state = GameState::Lost;
                            }
                        } else {
                            self.message = Some("Not enough letters!".to_string());
                        }
                    }
                    _ => {}
                }
                true
            }
        }
    }

    fn update(&mut self) {
        // Wordle is purely event-driven; nothing to do on tick.
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Outer border.
        let outer_block = Block::default()
            .title(" Wordle ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Vertical layout: title, grid, message, keyboard, help.
        let chunks = Layout::vertical([
            Constraint::Length(2),  // Title / guess counter
            Constraint::Length(14), // Guess grid (6 rows x 2 lines each + spacing)
            Constraint::Length(2),  // Message area
            Constraint::Length(6),  // Keyboard
            Constraint::Min(1),    // Help text
        ])
        .split(inner);

        self.render_header(frame, chunks[0]);
        self.render_grid(frame, chunks[1]);
        self.render_message(frame, chunks[2]);
        self.render_keyboard(frame, chunks[3]);
        self.render_help(frame, chunks[4]);

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

impl WordleGame {
    /// Render the header showing guess count.
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let attempts_left = MAX_GUESSES - self.guesses.len();
        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                "  Guess the word!  ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  [{}/{}]", self.guesses.len(), MAX_GUESSES),
                Style::default().fg(if attempts_left <= 2 {
                    Color::Red
                } else {
                    Color::Gray
                }),
            ),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(header, area);
    }

    /// Render the 6-row guess grid centred in the given area.
    fn render_grid(&self, frame: &mut Frame, area: Rect) {
        // Each tile is 4 chars wide (e.g. " A ") with a 1 char gap, total:
        // 5 * 4 + 4 * 1 = 24 chars wide. We centre this.
        let grid_width: u16 = 24;
        // Each row is 2 lines tall (the tile content), 6 rows = 12 lines.
        let grid_height: u16 = 12;
        let grid_area = centered_rect(grid_width, grid_height, area);

        let mut lines: Vec<Line> = Vec::new();

        for row_idx in 0..MAX_GUESSES {
            let (top_line, bot_line) = if row_idx < self.guesses.len() {
                // Submitted guess row.
                self.render_guess_row(&self.guesses[row_idx])
            } else if row_idx == self.guesses.len() && self.state == GameState::Playing {
                // Current input row.
                self.render_input_row()
            } else {
                // Empty row.
                self.render_empty_row()
            };
            lines.push(top_line);
            lines.push(bot_line);
        }

        let grid = Paragraph::new(lines).alignment(Alignment::Center);
        frame.render_widget(grid, grid_area);
    }

    /// Render a submitted guess row as two Lines (top border + letter).
    fn render_guess_row(&self, guess: &GuessRow) -> (Line<'static>, Line<'static>) {
        let mut top_spans: Vec<Span> = Vec::new();
        let mut bot_spans: Vec<Span> = Vec::new();

        for i in 0..WORD_LENGTH {
            let bg = guess.status[i].color();
            let letter = guess.letters[i].to_ascii_uppercase();
            let style = Style::default()
                .fg(Color::Black)
                .bg(bg)
                .add_modifier(Modifier::BOLD);

            top_spans.push(Span::styled("    ", style));
            bot_spans.push(Span::styled(format!(" {} ", letter), style));
            // A narrow spacer between tiles (except after the last).
            bot_spans.push(Span::raw(" "));

            if i < WORD_LENGTH - 1 {
                top_spans.push(Span::raw(" "));
            }
        }

        (Line::from(top_spans), Line::from(bot_spans))
    }

    /// Render the current input row.
    fn render_input_row(&self) -> (Line<'static>, Line<'static>) {
        let mut top_spans: Vec<Span> = Vec::new();
        let mut bot_spans: Vec<Span> = Vec::new();

        for i in 0..WORD_LENGTH {
            if i < self.current_input.len() {
                let letter = self.current_input[i].to_ascii_uppercase();
                let style = Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD);
                let border_style = Style::default().fg(Color::LightYellow);

                top_spans.push(Span::styled("----", border_style));
                bot_spans.push(Span::styled(format!(" {} ", letter), style));
                bot_spans.push(Span::raw(" "));
            } else {
                let border_style = Style::default().fg(Color::Gray);
                top_spans.push(Span::styled("----", border_style));
                bot_spans.push(Span::styled(" _ ", Style::default().fg(Color::Gray)));
                bot_spans.push(Span::raw(" "));
            }

            if i < WORD_LENGTH - 1 {
                top_spans.push(Span::raw(" "));
            }
        }

        (Line::from(top_spans), Line::from(bot_spans))
    }

    /// Render an empty row.
    fn render_empty_row(&self) -> (Line<'static>, Line<'static>) {
        let mut top_spans: Vec<Span> = Vec::new();
        let mut bot_spans: Vec<Span> = Vec::new();
        let style = Style::default().fg(Color::DarkGray);

        for i in 0..WORD_LENGTH {
            top_spans.push(Span::styled("----", style));
            bot_spans.push(Span::styled(" . ", style));
            bot_spans.push(Span::raw(" "));

            if i < WORD_LENGTH - 1 {
                top_spans.push(Span::raw(" "));
            }
        }

        (Line::from(top_spans), Line::from(bot_spans))
    }

    /// Render error/info message area.
    fn render_message(&self, frame: &mut Frame, area: Rect) {
        let text = if let Some(ref msg) = self.message {
            Line::from(Span::styled(
                msg.clone(),
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ))
        } else {
            Line::from("")
        };

        let paragraph = Paragraph::new(text).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
    }

    /// Render the on-screen keyboard with colour feedback.
    fn render_keyboard(&self, frame: &mut Frame, area: Rect) {
        let keyboard_width: u16 = 40;
        let keyboard_height: u16 = 5;
        let kb_area = centered_rect(keyboard_width, keyboard_height, area);

        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(kb_area);

        // Row label at top.
        let label = Paragraph::new(Line::from(Span::styled(
            "Keyboard",
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::DIM),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(label, rows[0]);

        // Render each keyboard row.
        for (row_idx, row_keys) in KEYBOARD_ROWS.iter().enumerate() {
            let mut spans: Vec<Span> = Vec::new();

            // Add leading padding for offset rows.
            match row_idx {
                1 => spans.push(Span::raw(" ")),
                2 => spans.push(Span::raw("   ")),
                _ => {}
            }

            for ch in row_keys.chars() {
                let idx = (ch as u8 - b'a') as usize;
                let (fg, bg, mods) = match self.keyboard[idx] {
                    Some(LetterStatus::Correct) => {
                        (Color::Black, Color::Green, Modifier::BOLD)
                    }
                    Some(LetterStatus::WrongPosition) => {
                        (Color::Black, Color::Yellow, Modifier::BOLD)
                    }
                    Some(LetterStatus::NotInWord) => {
                        (Color::DarkGray, Color::Reset, Modifier::DIM)
                    }
                    None => (Color::White, Color::Reset, Modifier::empty()),
                };

                let style = Style::default().fg(fg).bg(bg).add_modifier(mods);
                spans.push(Span::styled(
                    format!(" {} ", ch.to_ascii_uppercase()),
                    style,
                ));
            }

            let line = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
            frame.render_widget(line, rows[row_idx + 1]);
        }
    }

    /// Render help text at the bottom.
    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help = match self.state {
            GameState::Playing => {
                vec![Line::from(vec![
                    Span::styled("A-Z ", Style::default().fg(Color::Yellow)),
                    Span::styled("type  ", Style::default().fg(Color::Gray)),
                    Span::styled("Enter ", Style::default().fg(Color::Green)),
                    Span::styled("submit  ", Style::default().fg(Color::Gray)),
                    Span::styled("Bksp ", Style::default().fg(Color::Red)),
                    Span::styled("delete  ", Style::default().fg(Color::Gray)),
                    Span::styled("Esc ", Style::default().fg(Color::Magenta)),
                    Span::styled("quit", Style::default().fg(Color::Gray)),
                ])]
            }
            _ => {
                vec![Line::from(vec![
                    Span::styled("R ", Style::default().fg(Color::Yellow)),
                    Span::styled("restart  ", Style::default().fg(Color::Gray)),
                    Span::styled("Esc ", Style::default().fg(Color::Magenta)),
                    Span::styled("quit", Style::default().fg(Color::Gray)),
                ])]
            }
        };

        let paragraph = Paragraph::new(help).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
    }

    /// Render a centred win or lose overlay.
    fn render_overlay(&self, frame: &mut Frame, area: Rect, won: bool) {
        let popup_w: u16 = 44.min(area.width.saturating_sub(4));
        let popup_h: u16 = 11.min(area.height.saturating_sub(4));
        let popup = centered_rect(popup_w, popup_h, area);

        frame.render_widget(Clear, popup);

        let (title, msg, color) = if won {
            let msg = match self.guesses.len() {
                1 => "GENIUS!",
                2 => "MAGNIFICENT!",
                3 => "IMPRESSIVE!",
                4 => "SPLENDID!",
                5 => "GREAT!",
                6 => "PHEW!",
                _ => "YOU WIN!",
            };
            (" You Win! ", msg, Color::Green)
        } else {
            (" Game Over ", "GAME OVER", Color::Red)
        };

        let lines = vec![
            Line::from(""),
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
                    self.secret.to_uppercase(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                format!(
                    "Solved in {}/{} guesses",
                    self.guesses.len(),
                    MAX_GUESSES
                ),
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'r' to restart or Esc to quit",
                Style::default().fg(Color::DarkGray),
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
