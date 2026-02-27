use crossterm::event::{KeyCode, KeyEvent};
use rand::seq::SliceRandom;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame,
};
use std::time::Instant;

use crate::games::Game;

// ---------------------------------------------------------------------------
// Paragraphs to type
// ---------------------------------------------------------------------------

const TEXTS: &[&str] = &[
    "The quick brown fox jumps over the lazy dog near the riverbank while the sun sets behind the mountains in a blaze of orange and purple light.",
    "Programming is the art of telling another human what one wants the computer to do. Good code is its own best documentation and reads like well-written prose.",
    "Rust is a systems programming language focused on safety, speed, and concurrency. It achieves memory safety without garbage collection through its ownership model.",
    "In the middle of difficulty lies opportunity. The only way to do great work is to love what you do. Stay hungry, stay foolish, and never stop learning.",
    "The terminal is a powerful interface that lets you communicate directly with your computer. Mastering the command line can dramatically improve your productivity.",
    "Touch typing is an essential skill for any programmer. By keeping your eyes on the screen instead of the keyboard, you can code faster and with fewer errors.",
    "A journey of a thousand miles begins with a single step. Every expert was once a beginner, and every professional was once an amateur who refused to give up.",
    "Algorithms and data structures form the backbone of computer science. Understanding them deeply allows you to solve complex problems efficiently and elegantly.",
    "The best way to predict the future is to invent it. Technology is nothing until it becomes a tool in the hands of creative people who push boundaries every day.",
    "Open source software has transformed the world of technology. Collaboration across borders and time zones produces remarkable tools that benefit everyone equally.",
    "Practice makes perfect, but only if you practice with intention. Mindless repetition builds habits, while deliberate practice builds genuine skill and mastery.",
    "The ocean stretched endlessly before them, waves crashing against the rocky shore. Seagulls circled overhead, calling out to one another in the salty breeze.",
    "Coffee is the fuel that powers the modern world. From small artisan roasters to massive chains, the culture of coffee connects billions of people every morning.",
    "Debugging is twice as hard as writing the code in the first place. Therefore, if you write the code as cleverly as possible, you are not smart enough to debug it.",
    "The stars twinkled in the midnight sky as the campfire crackled softly. Stories were shared, laughter echoed through the trees, and memories were made forever.",
];

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Waiting for the first keypress to begin.
    Waiting,
    /// Timer is running and the player is typing.
    Running,
    /// Time ran out or the text was completed.
    Finished,
}

pub struct TypingTestGame {
    /// The target text the user must type.
    target: Vec<char>,
    /// Characters the user has typed so far (one per target position).
    typed: Vec<char>,
    /// Current cursor position in the target text.
    cursor: usize,
    /// Number of correctly typed characters.
    correct: usize,
    /// Total characters typed (including corrections via backspace are not counted;
    /// each position counts once when a character is committed).
    total_typed: usize,
    /// Current game phase.
    phase: Phase,
    /// When the first character was typed (timer start).
    start_time: Option<Instant>,
    /// Elapsed seconds (frozen on finish).
    elapsed_secs: f64,
    /// Timer duration in seconds (30 or 60).
    duration_secs: u64,
    /// Computed words per minute.
    wpm: f64,
    /// Computed accuracy percentage.
    accuracy: f64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pick_random_text() -> &'static str {
    let mut rng = rand::thread_rng();
    TEXTS.choose(&mut rng).unwrap_or(&TEXTS[0])
}

fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

impl Game for TypingTestGame {
    fn new() -> Self {
        let text = pick_random_text();
        TypingTestGame {
            target: text.chars().collect(),
            typed: Vec::new(),
            cursor: 0,
            correct: 0,
            total_typed: 0,
            phase: Phase::Waiting,
            start_time: None,
            elapsed_secs: 0.0,
            duration_secs: 30,
            wpm: 0.0,
            accuracy: 0.0,
        }
    }

    fn handle_event(&mut self, event: KeyEvent) -> bool {
        match self.phase {
            Phase::Waiting => self.handle_waiting(event),
            Phase::Running => self.handle_running(event),
            Phase::Finished => self.handle_finished(event),
        }
    }

    fn update(&mut self) {
        if self.phase != Phase::Running {
            return;
        }

        if let Some(start) = self.start_time {
            self.elapsed_secs = start.elapsed().as_secs_f64();

            // Calculate WPM: (chars_typed / 5) / minutes
            let minutes = self.elapsed_secs / 60.0;
            if minutes > 0.0 {
                self.wpm = (self.correct as f64 / 5.0) / minutes;
            }

            // Calculate accuracy
            if self.total_typed > 0 {
                self.accuracy = (self.correct as f64 / self.total_typed as f64) * 100.0;
            }

            // Check if time is up
            if self.elapsed_secs >= self.duration_secs as f64 {
                self.elapsed_secs = self.duration_secs as f64;
                self.phase = Phase::Finished;
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Main layout: title, body, footer
        let chunks = Layout::vertical([
            Constraint::Length(3), // Title bar
            Constraint::Length(3), // Stats bar
            Constraint::Length(3), // Progress bar
            Constraint::Min(5),   // Text area
            Constraint::Length(3), // Help bar
        ])
        .split(area);

        self.render_title(frame, chunks[0]);
        self.render_stats(frame, chunks[1]);
        self.render_progress(frame, chunks[2]);
        self.render_text(frame, chunks[3]);
        self.render_help(frame, chunks[4]);

        if self.phase == Phase::Finished {
            self.render_results(frame, area);
        }
    }
}

// ---------------------------------------------------------------------------
// Input handling
// ---------------------------------------------------------------------------

impl TypingTestGame {
    fn handle_waiting(&mut self, event: KeyEvent) -> bool {
        match event.code {
            // Esc is handled by the game loop, but we also return false to quit
            KeyCode::Esc => return false,
            KeyCode::Tab => {
                // Toggle between 30s and 60s
                self.duration_secs = if self.duration_secs == 30 { 60 } else { 30 };
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.restart();
            }
            KeyCode::Char(c) => {
                // First keypress starts the timer
                self.phase = Phase::Running;
                self.start_time = Some(Instant::now());
                self.process_char(c);
            }
            _ => {}
        }
        true
    }

    fn handle_running(&mut self, event: KeyEvent) -> bool {
        match event.code {
            KeyCode::Esc => return false,
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    // Check if the removed character was correct and adjust counts
                    if let Some(&typed_ch) = self.typed.get(self.cursor) {
                        if typed_ch == self.target[self.cursor] {
                            self.correct = self.correct.saturating_sub(1);
                        }
                        self.total_typed = self.total_typed.saturating_sub(1);
                    }
                    self.typed.truncate(self.cursor);
                }
            }
            KeyCode::Char(c) => {
                self.process_char(c);
            }
            _ => {}
        }
        true
    }

    fn handle_finished(&mut self, event: KeyEvent) -> bool {
        match event.code {
            KeyCode::Esc => return false,
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.restart();
            }
            _ => {}
        }
        true
    }

    fn process_char(&mut self, c: char) {
        if self.cursor >= self.target.len() {
            // Text fully typed
            self.phase = Phase::Finished;
            return;
        }

        self.typed.push(c);
        self.total_typed += 1;
        if c == self.target[self.cursor] {
            self.correct += 1;
        }
        self.cursor += 1;

        // Check if the user just finished the entire text
        if self.cursor >= self.target.len() {
            self.phase = Phase::Finished;
        }
    }

    fn restart(&mut self) {
        let text = pick_random_text();
        self.target = text.chars().collect();
        self.typed.clear();
        self.cursor = 0;
        self.correct = 0;
        self.total_typed = 0;
        self.phase = Phase::Waiting;
        self.start_time = None;
        self.elapsed_secs = 0.0;
        self.wpm = 0.0;
        self.accuracy = 0.0;
        // Keep the current duration_secs setting
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

impl TypingTestGame {
    fn render_title(&self, frame: &mut Frame, area: Rect) {
        let duration_label = format!("{}s", self.duration_secs);
        let title = format!(" Typing Test  |  {} ", duration_label);
        let block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        frame.render_widget(block, area);
    }

    fn render_stats(&self, frame: &mut Frame, area: Rect) {
        let time_remaining = if self.phase == Phase::Waiting {
            self.duration_secs as f64
        } else {
            (self.duration_secs as f64 - self.elapsed_secs).max(0.0)
        };

        let wpm_color = if self.wpm >= 60.0 {
            Color::Green
        } else if self.wpm >= 30.0 {
            Color::Yellow
        } else {
            Color::White
        };

        let acc_color = if self.accuracy >= 95.0 {
            Color::Green
        } else if self.accuracy >= 80.0 {
            Color::Yellow
        } else if self.total_typed > 0 {
            Color::Red
        } else {
            Color::White
        };

        let spans = vec![
            Span::styled("  WPM: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.0}", self.wpm),
                Style::default().fg(wpm_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  |  Accuracy: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.1}%", self.accuracy),
                Style::default().fg(acc_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  |  Time: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.0}s", time_remaining),
                Style::default()
                    .fg(if time_remaining <= 5.0 && self.phase == Phase::Running {
                        Color::Red
                    } else {
                        Color::White
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  |  {}/{} chars", self.cursor, self.target.len()),
                Style::default().fg(Color::DarkGray),
            ),
        ];

        let stats = Paragraph::new(Line::from(spans))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(stats, area);
    }

    fn render_progress(&self, frame: &mut Frame, area: Rect) {
        let ratio = if self.phase == Phase::Waiting {
            1.0
        } else {
            let remaining = (self.duration_secs as f64 - self.elapsed_secs).max(0.0);
            (remaining / self.duration_secs as f64).clamp(0.0, 1.0)
        };

        let color = if ratio <= 0.15 {
            Color::Red
        } else if ratio <= 0.33 {
            Color::Yellow
        } else {
            Color::Green
        };

        let label = if self.phase == Phase::Waiting {
            format!("{}s remaining - start typing!", self.duration_secs)
        } else {
            let remaining = (self.duration_secs as f64 - self.elapsed_secs).max(0.0);
            format!("{:.0}s remaining", remaining)
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(" Time ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .gauge_style(Style::default().fg(color).bg(Color::Rgb(40, 40, 50)))
            .ratio(ratio)
            .label(Span::styled(
                label,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(gauge, area);
    }

    fn render_text(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Type the text below ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Build spans character by character
        let mut spans: Vec<Span> = Vec::with_capacity(self.target.len());

        for (i, &target_ch) in self.target.iter().enumerate() {
            if i < self.cursor {
                // Already typed
                let typed_ch = self.typed.get(i).copied().unwrap_or(' ');
                if typed_ch == target_ch {
                    // Correct: green
                    spans.push(Span::styled(
                        String::from(target_ch),
                        Style::default().fg(Color::Green),
                    ));
                } else {
                    // Wrong: show the expected char in red on red background
                    spans.push(Span::styled(
                        String::from(typed_ch),
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Red),
                    ));
                }
            } else if i == self.cursor {
                // Current position: highlighted/underlined
                spans.push(Span::styled(
                    String::from(target_ch),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::UNDERLINED | Modifier::BOLD),
                ));
            } else {
                // Not yet typed: gray
                spans.push(Span::styled(
                    String::from(target_ch),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }

        // We need to wrap the text. Build lines that fit within `inner.width`.
        let max_width = inner.width as usize;
        let lines = wrap_spans(&spans, &self.target, max_width);

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_spans = match self.phase {
            Phase::Waiting => vec![
                Span::styled(
                    " Tab ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("toggle 30s/60s  "),
                Span::styled(
                    " r ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("new text  "),
                Span::styled(
                    " Esc ",
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("quit  "),
                Span::styled(
                    " Start typing to begin! ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ],
            Phase::Running => vec![
                Span::styled(
                    " Backspace ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("correct  "),
                Span::styled(
                    " Esc ",
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("quit"),
            ],
            Phase::Finished => vec![
                Span::styled(
                    " r ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("restart  "),
                Span::styled(
                    " Esc ",
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("quit"),
            ],
        };

        let help = Paragraph::new(Line::from(help_spans))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(help, area);
    }

    fn render_results(&self, frame: &mut Frame, area: Rect) {
        let popup_w: u16 = 50.min(area.width.saturating_sub(4));
        let popup_h: u16 = 13.min(area.height.saturating_sub(4));
        let popup = centered_rect(popup_w, popup_h, area);

        frame.render_widget(Clear, popup);

        let final_accuracy = if self.total_typed > 0 {
            (self.correct as f64 / self.total_typed as f64) * 100.0
        } else {
            0.0
        };

        let elapsed = self.elapsed_secs.max(0.001);
        let final_wpm = (self.correct as f64 / 5.0) / (elapsed / 60.0);

        let wpm_color = if final_wpm >= 60.0 {
            Color::Green
        } else if final_wpm >= 30.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        let acc_color = if final_accuracy >= 95.0 {
            Color::Green
        } else if final_accuracy >= 80.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        let grade = if final_wpm >= 80.0 && final_accuracy >= 95.0 {
            ("Excellent!", Color::Green)
        } else if final_wpm >= 50.0 && final_accuracy >= 90.0 {
            ("Great Job!", Color::Cyan)
        } else if final_wpm >= 30.0 && final_accuracy >= 80.0 {
            ("Good Work!", Color::Yellow)
        } else {
            ("Keep Practicing!", Color::Red)
        };

        let lines = vec![
            Line::from(Span::styled(
                "TIME'S UP!",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  WPM:       ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{:.0}", final_wpm),
                    Style::default()
                        .fg(wpm_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Accuracy:  ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{:.1}%", final_accuracy),
                    Style::default()
                        .fg(acc_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Correct:   ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{}", self.correct),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!(" / {} chars", self.total_typed),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Time:      ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{:.1}s", self.elapsed_secs),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                grade.0,
                Style::default()
                    .fg(grade.1)
                    .add_modifier(Modifier::BOLD),
            )),
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
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(" Results ")
                    .title_alignment(Alignment::Center),
            );

        frame.render_widget(paragraph, popup);
    }
}

// ---------------------------------------------------------------------------
// Text wrapping helper
// ---------------------------------------------------------------------------

/// Wraps a list of single-character `Span`s into `Line`s that fit within
/// `max_width` columns, breaking at word boundaries when possible.
fn wrap_spans<'a>(spans: &[Span<'a>], _target: &[char], max_width: usize) -> Vec<Line<'a>> {
    if max_width == 0 {
        return vec![];
    }

    let mut lines: Vec<Line<'a>> = Vec::new();
    let mut current_line: Vec<Span<'a>> = Vec::new();
    let mut col = 0;

    for (i, span) in spans.iter().enumerate() {
        if col >= max_width {
            lines.push(Line::from(current_line));
            current_line = Vec::new();
            col = 0;
        }
        current_line.push(span.clone());
        col += 1;

        // If we hit a space at the end of max_width, break nicely
        if col >= max_width && i + 1 < spans.len() {
            lines.push(Line::from(current_line));
            current_line = Vec::new();
            col = 0;
        }
    }

    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    lines
}
