use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::games::Game;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The four Simon colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SimonColor {
    Red,
    Green,
    Blue,
    Yellow,
}

impl SimonColor {
    const ALL: [SimonColor; 4] = [
        SimonColor::Red,
        SimonColor::Green,
        SimonColor::Blue,
        SimonColor::Yellow,
    ];

    /// The dim (inactive) color for this panel.
    fn dim(self) -> Color {
        match self {
            SimonColor::Red => Color::Rgb(100, 20, 20),
            SimonColor::Green => Color::Rgb(20, 80, 20),
            SimonColor::Blue => Color::Rgb(20, 20, 100),
            SimonColor::Yellow => Color::Rgb(100, 100, 20),
        }
    }

    /// The bright (active/lit) color for this panel.
    fn bright(self) -> Color {
        match self {
            SimonColor::Red => Color::Rgb(255, 60, 60),
            SimonColor::Green => Color::Rgb(60, 255, 60),
            SimonColor::Blue => Color::Rgb(80, 80, 255),
            SimonColor::Yellow => Color::Rgb(255, 255, 60),
        }
    }

    /// The text color used on top of the bright panel.
    fn text_color(self) -> Color {
        match self {
            SimonColor::Red => Color::White,
            SimonColor::Green => Color::Black,
            SimonColor::Blue => Color::White,
            SimonColor::Yellow => Color::Black,
        }
    }

    /// The label shown on the panel.
    fn label(self) -> &'static str {
        match self {
            SimonColor::Red => "R",
            SimonColor::Green => "G",
            SimonColor::Blue => "B",
            SimonColor::Yellow => "Y",
        }
    }

    /// The full name for display purposes.
    fn name(self) -> &'static str {
        match self {
            SimonColor::Red => "Red",
            SimonColor::Green => "Green",
            SimonColor::Blue => "Blue",
            SimonColor::Yellow => "Yellow",
        }
    }

    /// Random color.
    fn random() -> Self {
        let mut rng = rand::thread_rng();
        SimonColor::ALL[rng.gen_range(0..4)]
    }
}

/// The game phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Brief pause before showing the sequence.
    PreShow,
    /// Animating the sequence: lighting up colors one at a time.
    ShowSequence,
    /// Waiting for the player to reproduce the sequence.
    PlayerInput,
    /// Wrong input was given.
    GameOver,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Ticks for the pause before showing the sequence.
const PRE_SHOW_TICKS: u32 = 20; // ~320ms at 16ms/tick

/// Base ticks each color stays lit during ShowSequence.
const BASE_SHOW_ON_TICKS: u32 = 25; // ~400ms

/// Base ticks of gap (no light) between sequence items.
const BASE_SHOW_OFF_TICKS: u32 = 12; // ~192ms

/// Minimum on-ticks as speed increases.
const MIN_SHOW_ON_TICKS: u32 = 10; // ~160ms

/// Minimum off-ticks as speed increases.
const MIN_SHOW_OFF_TICKS: u32 = 5; // ~80ms

/// Speed reduction per round (in ticks).
const SPEED_REDUCTION: u32 = 2;

/// How many ticks the player flash feedback lasts.
const PLAYER_FLASH_TICKS: u32 = 8; // ~128ms

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

pub struct SimonGame {
    /// The full sequence generated so far.
    sequence: Vec<SimonColor>,
    /// Current round (1-based, equals sequence length).
    round: u32,
    /// Current game phase.
    phase: Phase,
    /// Index into the sequence for show/input phases.
    seq_index: usize,
    /// Tick counter within the current animation step.
    tick: u32,
    /// Whether we are in the "on" (lit) or "off" (gap) part of the show phase.
    show_on: bool,
    /// Which color is currently highlighted (during show or player flash).
    active_color: Option<SimonColor>,
    /// Remaining ticks for the player input flash feedback.
    player_flash_remaining: u32,
    /// The highest round the player completed (for game-over display).
    best_round: u32,
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

impl Game for SimonGame {
    fn new() -> Self {
        let first_color = SimonColor::random();
        SimonGame {
            sequence: vec![first_color],
            round: 1,
            phase: Phase::PreShow,
            seq_index: 0,
            tick: 0,
            show_on: true,
            active_color: None,
            player_flash_remaining: 0,
            best_round: 0,
        }
    }

    fn handle_event(&mut self, event: KeyEvent) -> bool {
        match self.phase {
            Phase::GameOver => match event.code {
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    *self = SimonGame::new();
                    true
                }
                KeyCode::Esc => false,
                _ => true,
            },
            Phase::PlayerInput => {
                let pressed = match event.code {
                    KeyCode::Char('r') | KeyCode::Char('R') => Some(SimonColor::Red),
                    KeyCode::Char('g') | KeyCode::Char('G') => Some(SimonColor::Green),
                    KeyCode::Char('b') | KeyCode::Char('B') => Some(SimonColor::Blue),
                    KeyCode::Char('y') | KeyCode::Char('Y') => Some(SimonColor::Yellow),
                    KeyCode::Esc => return false,
                    _ => None,
                };

                if let Some(color) = pressed {
                    // Flash feedback for the pressed color.
                    self.active_color = Some(color);
                    self.player_flash_remaining = PLAYER_FLASH_TICKS;

                    let expected = self.sequence[self.seq_index];
                    if color == expected {
                        // Correct!
                        self.seq_index += 1;
                        if self.seq_index >= self.sequence.len() {
                            // Round complete! Advance to next round.
                            self.best_round = self.round;
                            self.round += 1;
                            self.sequence.push(SimonColor::random());
                            self.seq_index = 0;
                            self.tick = 0;
                            self.show_on = true;
                            self.phase = Phase::PreShow;
                            // active_color will clear after flash
                        }
                    } else {
                        // Wrong! Game over.
                        self.phase = Phase::GameOver;
                    }
                }

                true
            }
            // During PreShow and ShowSequence, ignore input except Esc.
            _ => match event.code {
                KeyCode::Esc => false,
                _ => true,
            },
        }
    }

    fn update(&mut self) {
        // Tick down the player flash.
        if self.player_flash_remaining > 0 {
            self.player_flash_remaining -= 1;
            if self.player_flash_remaining == 0 && self.phase != Phase::GameOver {
                self.active_color = None;
            }
        }

        match self.phase {
            Phase::PreShow => {
                self.active_color = None;
                self.tick += 1;
                if self.tick >= PRE_SHOW_TICKS {
                    self.tick = 0;
                    self.seq_index = 0;
                    self.show_on = true;
                    self.phase = Phase::ShowSequence;
                }
            }
            Phase::ShowSequence => {
                self.tick += 1;

                let on_ticks = self.show_on_ticks();
                let off_ticks = self.show_off_ticks();

                if self.show_on {
                    // Currently lighting a color.
                    self.active_color = Some(self.sequence[self.seq_index]);
                    if self.tick >= on_ticks {
                        // Turn off, go to gap.
                        self.active_color = None;
                        self.tick = 0;
                        self.show_on = false;
                    }
                } else {
                    // Gap between colors.
                    self.active_color = None;
                    if self.tick >= off_ticks {
                        self.seq_index += 1;
                        self.tick = 0;
                        self.show_on = true;
                        if self.seq_index >= self.sequence.len() {
                            // Done showing sequence, player's turn.
                            self.seq_index = 0;
                            self.phase = Phase::PlayerInput;
                        }
                    }
                }
            }
            Phase::PlayerInput => {
                // Nothing to do here; handled in handle_event.
            }
            Phase::GameOver => {
                // Nothing to animate.
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Outer block.
        let title = format!(" Simon  |  Round: {} ", self.round);
        let outer_block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Fill background.
        let bg = Paragraph::new("").style(Style::default().bg(Color::Rgb(15, 15, 25)));
        frame.render_widget(bg, inner);

        // Layout: top info, the 2x2 grid, bottom help.
        let main_chunks = Layout::vertical([
            Constraint::Length(2), // status info
            Constraint::Min(8),   // the 4 panels
            Constraint::Length(2), // help bar
        ])
        .split(inner);

        // --- Status info ---
        self.render_status(frame, main_chunks[0]);

        // --- 2x2 color grid ---
        self.render_panels(frame, main_chunks[1]);

        // --- Help bar ---
        self.render_help(frame, main_chunks[2]);

        // --- Game Over overlay ---
        if self.phase == Phase::GameOver {
            self.render_game_over(frame, area);
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

impl SimonGame {
    /// Compute the "on" ticks for the current round (speed increases).
    fn show_on_ticks(&self) -> u32 {
        let reduction = (self.round.saturating_sub(1)) * SPEED_REDUCTION;
        BASE_SHOW_ON_TICKS.saturating_sub(reduction).max(MIN_SHOW_ON_TICKS)
    }

    /// Compute the "off" ticks for the current round.
    fn show_off_ticks(&self) -> u32 {
        let reduction = (self.round.saturating_sub(1)) * SPEED_REDUCTION;
        BASE_SHOW_OFF_TICKS.saturating_sub(reduction).max(MIN_SHOW_OFF_TICKS)
    }

    /// Render the status line showing the current phase.
    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let (text, color) = match self.phase {
            Phase::PreShow => ("Get ready...", Color::Cyan),
            Phase::ShowSequence => ("Watch the sequence!", Color::Yellow),
            Phase::PlayerInput => {
                let remaining = self.sequence.len() - self.seq_index;
                // We build the text inline below.
                let text = format!(
                    "Your turn! {}/{} remaining  [R] [G] [B] [Y]",
                    remaining,
                    self.sequence.len()
                );
                let line = Line::from(Span::styled(
                    text,
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ));
                let p = Paragraph::new(line).alignment(Alignment::Center);
                frame.render_widget(p, area);
                return;
            }
            Phase::GameOver => ("Game Over!", Color::Red),
        };

        let line = Line::from(Span::styled(
            text,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        let p = Paragraph::new(line).alignment(Alignment::Center);
        frame.render_widget(p, area);
    }

    /// Render the 2x2 color panel grid.
    fn render_panels(&self, frame: &mut Frame, area: Rect) {
        // Split vertically into two rows.
        let rows = Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(area);

        // Top row: Red (left), Green (right).
        let top_cols =
            Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(rows[0]);

        // Bottom row: Blue (left), Yellow (right).
        let bot_cols =
            Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(rows[1]);

        let panels = [
            (SimonColor::Red, top_cols[0]),
            (SimonColor::Green, top_cols[1]),
            (SimonColor::Blue, bot_cols[0]),
            (SimonColor::Yellow, bot_cols[1]),
        ];

        for (color, rect) in panels {
            let is_active = self.active_color == Some(color);
            self.render_single_panel(frame, rect, color, is_active);
        }
    }

    /// Render a single colored panel.
    fn render_single_panel(
        &self,
        frame: &mut Frame,
        area: Rect,
        color: SimonColor,
        active: bool,
    ) {
        let (bg, border_style) = if active {
            (
                color.bright(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (color.dim(), Style::default().fg(Color::DarkGray))
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ({}) ", color.name(), color.label()))
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Fill the panel background.
        let fill = Paragraph::new("").style(Style::default().bg(bg));
        frame.render_widget(fill, inner);

        // Draw a large centered label in the panel.
        if inner.height >= 2 && inner.width >= 3 {
            let label = color.label();
            let text_fg = if active {
                color.text_color()
            } else {
                Color::Rgb(60, 60, 60)
            };

            // Build a multi-line block text for a "large" label effect.
            let big_label = build_big_label(label);
            let big_height = big_label.len() as u16;

            // Center vertically.
            let y_offset = inner.height.saturating_sub(big_height) / 2;
            let label_area = Rect::new(
                inner.x,
                inner.y + y_offset,
                inner.width,
                big_height.min(inner.height),
            );

            let lines: Vec<Line> = big_label
                .into_iter()
                .map(|s| {
                    Line::from(Span::styled(
                        s,
                        Style::default()
                            .fg(text_fg)
                            .bg(bg)
                            .add_modifier(if active {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                    ))
                })
                .collect();

            let p = Paragraph::new(lines).alignment(Alignment::Center);
            frame.render_widget(p, label_area);
        }
    }

    /// Render the help bar at the bottom.
    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let spans = vec![
            Span::styled(
                " R ",
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " G ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " B ",
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Y ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(" Esc ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw("quit"),
        ];

        let p = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
        frame.render_widget(p, area);
    }

    /// Render the game-over overlay popup.
    fn render_game_over(&self, frame: &mut Frame, area: Rect) {
        let popup_w: u16 = 44.min(area.width.saturating_sub(4));
        let popup_h: u16 = 9.min(area.height.saturating_sub(4));
        let popup = centered_rect(popup_w, popup_h, area);

        frame.render_widget(Clear, popup);

        let rounds_completed = if self.best_round > 0 {
            self.best_round
        } else {
            0
        };

        let lines = vec![
            Line::from(Span::styled(
                "GAME OVER",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("You made it to round {}", self.round),
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::styled(
                format!("Rounds completed: {}", rounds_completed),
                Style::default().fg(Color::Cyan),
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
                    .border_style(Style::default().fg(Color::Red))
                    .title(" Game Over ")
                    .title_alignment(Alignment::Center),
            );

        frame.render_widget(paragraph, popup);
    }
}

// ---------------------------------------------------------------------------
// Standalone helpers
// ---------------------------------------------------------------------------

/// Build a "big" block-letter representation of a single character label.
/// Returns lines of text (using Unicode block characters).
fn build_big_label(ch: &str) -> Vec<String> {
    match ch {
        "R" => vec![
            "████".into(),
            "█  █".into(),
            "████".into(),
            "█ █ ".into(),
            "█  █".into(),
        ],
        "G" => vec![
            " ███".into(),
            "█   ".into(),
            "█ ██".into(),
            "█  █".into(),
            " ███".into(),
        ],
        "B" => vec![
            "████".into(),
            "█  █".into(),
            "████".into(),
            "█  █".into(),
            "████".into(),
        ],
        "Y" => vec![
            "█  █".into(),
            "█  █".into(),
            " ██ ".into(),
            " ██ ".into(),
            " ██ ".into(),
        ],
        _ => vec![ch.to_string()],
    }
}

/// Return a `Rect` of size `w x h` centred inside `area`.
fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}
