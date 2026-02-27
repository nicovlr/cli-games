use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::games::Game;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Paddle height in cells.
const PADDLE_H: u16 = 5;
/// Paddle width in cells.
const PADDLE_W: u16 = 1;
/// Distance of paddles from the edge of the playing field.
const PADDLE_MARGIN: u16 = 2;

/// Initial ball speed (cells per tick).
const BALL_SPEED_INIT: f64 = 0.35;
/// Speed increase each time a point is scored.
const BALL_SPEED_INCREMENT: f64 = 0.02;
/// Maximum ball speed.
const BALL_SPEED_MAX: f64 = 0.8;

/// Paddle movement speed (cells per tick when key is held).
const PADDLE_SPEED: f64 = 0.6;

/// CPU paddle tracking speed factor (0..1). Lower = more imperfect.
const CPU_SPEED: f64 = 0.38;
/// CPU reaction dead-zone: CPU won't move if the ball is within this many cells
/// of its paddle centre, adding some human-like imprecision.
const CPU_DEAD_ZONE: f64 = 1.2;

/// Points needed to win.
const WIN_SCORE: u32 = 5;

/// How many ticks the countdown lasts per digit (3, 2, 1).
const COUNTDOWN_TICKS_PER_STEP: u32 = 40;

/// Default playing field size (used until first render provides real size).
const DEFAULT_W: u16 = 60;
const DEFAULT_H: u16 = 24;

/// The ball character.
const BALL_CHAR: &str = "\u{25CF}"; // filled circle

// ---------------------------------------------------------------------------
// Game states
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Countdown before serve (value = remaining ticks).
    Countdown(u32),
    /// Ball is in play.
    Playing,
    /// Someone won.
    GameOver,
}

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

pub struct PongGame {
    // Playing-field dimensions (inner area, excluding borders).
    field_w: u16,
    field_h: u16,

    // Paddle positions (floating-point y of the paddle's top edge).
    player_y: f64,
    cpu_y: f64,

    // Ball position and velocity.
    ball_x: f64,
    ball_y: f64,
    ball_vx: f64,
    ball_vy: f64,

    /// Current ball speed magnitude.
    ball_speed: f64,

    // Scores.
    player_score: u32,
    cpu_score: u32,

    // Input state: which direction the player is pressing (or 0).
    player_dy: f64,

    phase: Phase,

    /// Who served last / who should receive next serve.
    /// false = serve towards player, true = serve towards CPU.
    serve_towards_cpu: bool,
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

impl Game for PongGame {
    fn new() -> Self {
        let mut game = PongGame {
            field_w: DEFAULT_W,
            field_h: DEFAULT_H,
            player_y: 0.0,
            cpu_y: 0.0,
            ball_x: 0.0,
            ball_y: 0.0,
            ball_vx: 0.0,
            ball_vy: 0.0,
            ball_speed: BALL_SPEED_INIT,
            player_score: 0,
            cpu_score: 0,
            player_dy: 0.0,
            phase: Phase::Countdown(COUNTDOWN_TICKS_PER_STEP * 3),
            serve_towards_cpu: true,
        };
        game.reset_positions();
        game
    }

    fn handle_event(&mut self, event: KeyEvent) -> bool {
        match self.phase {
            Phase::GameOver => match event.code {
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    *self = PongGame::new_with_size(self.field_w, self.field_h);
                    true
                }
                KeyCode::Esc => false,
                _ => true,
            },
            _ => {
                match event.code {
                    KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                        self.player_dy = -PADDLE_SPEED;
                    }
                    KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                        self.player_dy = PADDLE_SPEED;
                    }
                    KeyCode::Esc => return false,
                    _ => {}
                }
                true
            }
        }
    }

    fn update(&mut self) {
        match self.phase {
            Phase::Countdown(remaining) => {
                // Move player paddle during countdown too.
                self.move_player();
                if remaining == 0 {
                    self.phase = Phase::Playing;
                } else {
                    self.phase = Phase::Countdown(remaining - 1);
                }
            }
            Phase::Playing => {
                self.move_player();
                self.move_cpu();
                self.move_ball();
            }
            Phase::GameOver => {}
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Outer block.
        let block = Block::default()
            .title(" Pong ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Dark background.
        let bg = Paragraph::new("").style(Style::default().bg(Color::Rgb(10, 10, 20)));
        frame.render_widget(bg, inner);

        // If the terminal was resized, we note it for next update
        // (field_w/field_h are mutable through interior-mutability tricks, but
        // since render takes &self we just accept the slight lag).
        // The field dimensions are implicitly `inner`.

        // Score display at top.
        self.render_scores(frame, inner);

        // Dashed centre line.
        self.render_centre_line(frame, inner);

        // Paddles.
        self.render_paddles(frame, inner);

        // Ball (only when playing or countdown about to end).
        if self.phase == Phase::Playing
            || matches!(self.phase, Phase::Countdown(t) if t < COUNTDOWN_TICKS_PER_STEP)
        {
            self.render_ball(frame, inner);
        }

        // Countdown overlay.
        if let Phase::Countdown(t) = self.phase {
            self.render_countdown(frame, inner, t);
        }

        // Game-over overlay.
        if self.phase == Phase::GameOver {
            self.render_game_over(frame, area);
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

impl PongGame {
    /// Create a new game with explicit field dimensions.
    fn new_with_size(w: u16, h: u16) -> Self {
        let mut game = PongGame {
            field_w: w.max(20),
            field_h: h.max(10),
            player_y: 0.0,
            cpu_y: 0.0,
            ball_x: 0.0,
            ball_y: 0.0,
            ball_vx: 0.0,
            ball_vy: 0.0,
            ball_speed: BALL_SPEED_INIT,
            player_score: 0,
            cpu_score: 0,
            player_dy: 0.0,
            phase: Phase::Countdown(COUNTDOWN_TICKS_PER_STEP * 3),
            serve_towards_cpu: true,
        };
        game.reset_positions();
        game
    }

    /// Centre the ball and paddles; set initial ball velocity based on
    /// `serve_towards_cpu`.
    fn reset_positions(&mut self) {
        let fh = self.field_h as f64;
        let fw = self.field_w as f64;

        // Centre paddles vertically.
        self.player_y = (fh - PADDLE_H as f64) / 2.0;
        self.cpu_y = (fh - PADDLE_H as f64) / 2.0;

        // Centre ball.
        self.ball_x = fw / 2.0;
        self.ball_y = fh / 2.0;

        // Random initial angle between -30 and 30 degrees from horizontal.
        let mut rng = rand::thread_rng();
        let angle: f64 = rng.gen_range(-0.5..0.5); // radians, roughly +/-28 degrees
        let dir = if self.serve_towards_cpu { 1.0 } else { -1.0 };
        self.ball_vx = dir * self.ball_speed * angle.cos();
        self.ball_vy = self.ball_speed * angle.sin();
    }

    /// Start a new point (after someone scores).
    fn start_new_point(&mut self) {
        self.serve_towards_cpu = !self.serve_towards_cpu;
        self.ball_speed = (self.ball_speed + BALL_SPEED_INCREMENT).min(BALL_SPEED_MAX);
        self.phase = Phase::Countdown(COUNTDOWN_TICKS_PER_STEP * 3);
        self.reset_positions();
    }

    // -- Movement -----------------------------------------------------------

    fn move_player(&mut self) {
        let max_y = self.field_h as f64 - PADDLE_H as f64;
        self.player_y = (self.player_y + self.player_dy).clamp(0.0, max_y);
        // Reset input: the game loop is tick-based so we consume the input
        // each tick; the player must keep pressing to keep moving.
        self.player_dy = 0.0;
    }

    fn move_cpu(&mut self) {
        let max_y = self.field_h as f64 - PADDLE_H as f64;
        let cpu_centre = self.cpu_y + PADDLE_H as f64 / 2.0;
        let diff = self.ball_y - cpu_centre;

        if diff.abs() > CPU_DEAD_ZONE {
            let step = diff.signum() * CPU_SPEED;
            self.cpu_y = (self.cpu_y + step).clamp(0.0, max_y);
        }
    }

    fn move_ball(&mut self) {
        let fw = self.field_w as f64;
        let fh = self.field_h as f64;

        // Advance ball.
        self.ball_x += self.ball_vx;
        self.ball_y += self.ball_vy;

        // Top/bottom wall bounce.
        if self.ball_y <= 0.0 {
            self.ball_y = -self.ball_y;
            self.ball_vy = self.ball_vy.abs();
        } else if self.ball_y >= fh - 1.0 {
            self.ball_y = 2.0 * (fh - 1.0) - self.ball_y;
            self.ball_vy = -self.ball_vy.abs();
        }

        // Player paddle collision (left side).
        let paddle_left_x = PADDLE_MARGIN as f64;
        let paddle_right_x = paddle_left_x + PADDLE_W as f64;
        if self.ball_vx < 0.0
            && self.ball_x <= paddle_right_x
            && self.ball_x >= paddle_left_x - 0.5
            && self.ball_y >= self.player_y - 0.5
            && self.ball_y <= self.player_y + PADDLE_H as f64 + 0.5
        {
            self.ball_x = paddle_right_x;
            self.paddle_bounce(self.player_y, true);
        }

        // CPU paddle collision (right side).
        let cpu_paddle_x = fw - PADDLE_MARGIN as f64 - PADDLE_W as f64;
        if self.ball_vx > 0.0
            && self.ball_x >= cpu_paddle_x
            && self.ball_x <= cpu_paddle_x + PADDLE_W as f64 + 0.5
            && self.ball_y >= self.cpu_y - 0.5
            && self.ball_y <= self.cpu_y + PADDLE_H as f64 + 0.5
        {
            self.ball_x = cpu_paddle_x;
            self.paddle_bounce(self.cpu_y, false);
        }

        // Score detection: ball passed left edge.
        if self.ball_x < 0.0 {
            self.cpu_score += 1;
            if self.cpu_score >= WIN_SCORE {
                self.phase = Phase::GameOver;
            } else {
                self.start_new_point();
            }
            return;
        }

        // Score detection: ball passed right edge.
        if self.ball_x >= fw {
            self.player_score += 1;
            if self.player_score >= WIN_SCORE {
                self.phase = Phase::GameOver;
            } else {
                self.start_new_point();
            }
        }
    }

    /// Bounce the ball off a paddle. `going_right` is true when the ball
    /// should now travel to the right (i.e., it hit the player's paddle).
    fn paddle_bounce(&mut self, paddle_y: f64, going_right: bool) {
        // Normalised hit position: -1.0 (top edge) to 1.0 (bottom edge).
        let paddle_centre = paddle_y + PADDLE_H as f64 / 2.0;
        let offset = (self.ball_y - paddle_centre) / (PADDLE_H as f64 / 2.0);
        let offset = offset.clamp(-1.0, 1.0);

        // Map offset to angle: centre = 0, edges = +/- ~60 degrees.
        let max_angle: f64 = std::f64::consts::FRAC_PI_3; // 60 degrees
        let angle = offset * max_angle;

        let dir = if going_right { 1.0 } else { -1.0 };
        self.ball_vx = dir * self.ball_speed * angle.cos();
        self.ball_vy = self.ball_speed * angle.sin();
    }

    // -- Rendering ----------------------------------------------------------

    fn render_scores(&self, frame: &mut Frame, inner: Rect) {
        if inner.width < 20 || inner.height < 3 {
            return;
        }

        // Player score on left quarter, CPU score on right quarter.
        let score_y = inner.y;
        let quarter_w = inner.width / 4;

        // Player score.
        let player_text = format!("{}", self.player_score);
        let player_rect = Rect::new(inner.x + quarter_w.saturating_sub(1), score_y, 3, 1);
        let player_widget = Paragraph::new(Line::from(Span::styled(
            player_text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Rgb(10, 10, 20)));
        frame.render_widget(player_widget, player_rect);

        // CPU score.
        let cpu_text = format!("{}", self.cpu_score);
        let cpu_rect = Rect::new(
            inner.x + inner.width - quarter_w - 1,
            score_y,
            3,
            1,
        );
        let cpu_widget = Paragraph::new(Line::from(Span::styled(
            cpu_text,
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Rgb(10, 10, 20)));
        frame.render_widget(cpu_widget, cpu_rect);
    }

    fn render_centre_line(&self, frame: &mut Frame, inner: Rect) {
        let cx = inner.x + inner.width / 2;
        for row in 0..inner.height {
            // Dashed pattern: draw every other row.
            if row % 2 == 0 {
                let rect = Rect::new(cx, inner.y + row, 1, 1);
                let dash = Paragraph::new("\u{2502}") // thin vertical line
                    .style(Style::default().fg(Color::DarkGray).bg(Color::Rgb(10, 10, 20)));
                frame.render_widget(dash, rect);
            }
        }
    }

    fn render_paddles(&self, frame: &mut Frame, inner: Rect) {
        // Player paddle (left).
        let px = inner.x + PADDLE_MARGIN;
        let py = inner.y + (self.player_y.round() as u16).min(inner.height.saturating_sub(PADDLE_H));
        for i in 0..PADDLE_H {
            if py + i < inner.y + inner.height {
                let rect = Rect::new(px, py + i, PADDLE_W, 1);
                let w = Paragraph::new("\u{2588}") // full block
                    .style(Style::default().fg(Color::Cyan).bg(Color::Rgb(10, 10, 20)));
                frame.render_widget(w, rect);
            }
        }

        // CPU paddle (right).
        let cpu_px = inner.x + inner.width - PADDLE_MARGIN - PADDLE_W;
        let cpu_py =
            inner.y + (self.cpu_y.round() as u16).min(inner.height.saturating_sub(PADDLE_H));
        for i in 0..PADDLE_H {
            if cpu_py + i < inner.y + inner.height {
                let rect = Rect::new(cpu_px, cpu_py + i, PADDLE_W, 1);
                let w = Paragraph::new("\u{2588}")
                    .style(Style::default().fg(Color::Red).bg(Color::Rgb(10, 10, 20)));
                frame.render_widget(w, rect);
            }
        }
    }

    fn render_ball(&self, frame: &mut Frame, inner: Rect) {
        let bx = self.ball_x.round() as u16;
        let by = self.ball_y.round() as u16;
        if bx < inner.width && by < inner.height {
            let rect = Rect::new(inner.x + bx, inner.y + by, 1, 1);
            let w = Paragraph::new(BALL_CHAR).style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Rgb(10, 10, 20)),
            );
            frame.render_widget(w, rect);
        }
    }

    fn render_countdown(&self, frame: &mut Frame, inner: Rect, ticks: u32) {
        let digit = ticks / COUNTDOWN_TICKS_PER_STEP + 1;
        let text = format!("{}", digit.min(3));

        let cx = inner.x + inner.width / 2;
        let cy = inner.y + inner.height / 2;
        let rect = Rect::new(cx.saturating_sub(1), cy, 3, 1);

        let w = Paragraph::new(Line::from(Span::styled(
            text,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Rgb(10, 10, 20)));
        frame.render_widget(w, rect);
    }

    fn render_game_over(&self, frame: &mut Frame, area: Rect) {
        let popup_w: u16 = 36.min(area.width.saturating_sub(4));
        let popup_h: u16 = 9.min(area.height.saturating_sub(4));
        let popup = centered_rect(popup_w, popup_h, area);

        frame.render_widget(Clear, popup);

        let winner = if self.player_score >= WIN_SCORE {
            "YOU WIN!"
        } else {
            "CPU WINS!"
        };
        let winner_color = if self.player_score >= WIN_SCORE {
            Color::Green
        } else {
            Color::Red
        };

        let lines = vec![
            Line::from(Span::styled(
                winner,
                Style::default()
                    .fg(winner_color)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{} - {}", self.player_score, self.cpu_score),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "'r' restart  |  Esc quit",
                Style::default().fg(Color::Gray),
            )),
        ];

        let paragraph = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(winner_color))
                    .title(" Game Over ")
                    .title_alignment(Alignment::Center),
            );
        frame.render_widget(paragraph, popup);
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}
