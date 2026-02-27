use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::collections::VecDeque;

use crate::games::Game;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pos {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

pub struct SnakeGame {
    /// Snake body segments (front = head).
    body: VecDeque<Pos>,
    /// Current travel direction.
    direction: Direction,
    /// Queued direction change (applied on next move tick).
    next_direction: Direction,
    /// Position of the apple.
    apple: Pos,
    /// Play-area width in cells (set on first render).
    width: u16,
    /// Play-area height in cells (set on first render).
    height: u16,
    /// Number of apples eaten.
    score: u32,
    /// Whether the game is over.
    game_over: bool,
    /// Tick counter used to throttle movement speed.
    tick: u32,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// How many `update()` calls between each movement step.
const TICKS_PER_MOVE: u32 = 8;
/// Default grid size until the first render tells us the real terminal size.
const DEFAULT_W: u16 = 40;
const DEFAULT_H: u16 = 20;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_pos(width: u16, height: u16) -> Pos {
    let mut rng = rand::thread_rng();
    Pos {
        x: rng.gen_range(0..width as i32),
        y: rng.gen_range(0..height as i32),
    }
}

fn spawn_apple(body: &VecDeque<Pos>, width: u16, height: u16) -> Pos {
    loop {
        let p = random_pos(width, height);
        if !body.contains(&p) {
            return p;
        }
    }
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

impl Game for SnakeGame {
    fn new() -> Self {
        let w = DEFAULT_W;
        let h = DEFAULT_H;
        let start = Pos {
            x: w as i32 / 2,
            y: h as i32 / 2,
        };
        let mut body = VecDeque::new();
        body.push_back(start);
        body.push_back(Pos {
            x: start.x - 1,
            y: start.y,
        });
        body.push_back(Pos {
            x: start.x - 2,
            y: start.y,
        });

        let apple = spawn_apple(&body, w, h);

        SnakeGame {
            body,
            direction: Direction::Right,
            next_direction: Direction::Right,
            apple,
            width: w,
            height: h,
            score: 0,
            game_over: false,
            tick: 0,
        }
    }

    fn handle_event(&mut self, event: KeyEvent) -> bool {
        if self.game_over {
            match event.code {
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    *self = SnakeGame::new_with_size(self.width, self.height);
                    return true;
                }
                KeyCode::Esc => return false,
                _ => return true,
            }
        }

        match event.code {
            KeyCode::Up | KeyCode::Char('w') => {
                if self.direction != Direction::Down {
                    self.next_direction = Direction::Up;
                }
            }
            KeyCode::Down | KeyCode::Char('s') => {
                if self.direction != Direction::Up {
                    self.next_direction = Direction::Down;
                }
            }
            KeyCode::Left | KeyCode::Char('a') => {
                if self.direction != Direction::Right {
                    self.next_direction = Direction::Left;
                }
            }
            KeyCode::Right | KeyCode::Char('d') => {
                if self.direction != Direction::Left {
                    self.next_direction = Direction::Right;
                }
            }
            KeyCode::Esc => return false,
            _ => {}
        }
        true
    }

    fn update(&mut self) {
        if self.game_over {
            return;
        }

        self.tick += 1;
        if self.tick < TICKS_PER_MOVE {
            return;
        }
        self.tick = 0;

        // Apply queued direction.
        if self.next_direction.opposite() != self.direction {
            self.direction = self.next_direction;
        }

        // Compute new head position.
        let head = self.body.front().expect("snake has no body");
        let new_head = match self.direction {
            Direction::Up => Pos {
                x: head.x,
                y: head.y - 1,
            },
            Direction::Down => Pos {
                x: head.x,
                y: head.y + 1,
            },
            Direction::Left => Pos {
                x: head.x - 1,
                y: head.y,
            },
            Direction::Right => Pos {
                x: head.x + 1,
                y: head.y,
            },
        };

        // Wall collision.
        if new_head.x < 0
            || new_head.y < 0
            || new_head.x >= self.width as i32
            || new_head.y >= self.height as i32
        {
            self.game_over = true;
            return;
        }

        // Self collision (check against all body segments except the tail,
        // which will move away unless the snake just ate).
        if self.body.contains(&new_head) {
            self.game_over = true;
            return;
        }

        // Move the snake.
        self.body.push_front(new_head);

        if new_head == self.apple {
            // Grow: don't remove the tail.
            self.score += 1;
            self.apple = spawn_apple(&self.body, self.width, self.height);
        } else {
            // Normal move: remove the tail.
            self.body.pop_back();
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Outer block with score in title.
        let title = format!(" Snake  |  Score: {} ", self.score);
        let block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        // Render the border block first so we get its inner rect.
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Fill the inner area with a dark background.
        let bg = Paragraph::new("")
            .style(Style::default().bg(Color::Rgb(20, 20, 30)));
        frame.render_widget(bg, inner);

        // Draw each cell of the grid.
        // We iterate over only the cells that actually fit.
        let cols = inner.width.min(self.width);
        let rows = inner.height.min(self.height);

        // -- Apple --
        if (self.apple.x as u16) < cols && (self.apple.y as u16) < rows {
            let apple_rect = Rect::new(
                inner.x + self.apple.x as u16,
                inner.y + self.apple.y as u16,
                1,
                1,
            );
            let apple_widget = Paragraph::new("●")
                .style(Style::default().fg(Color::Red).bg(Color::Rgb(20, 20, 30)));
            frame.render_widget(apple_widget, apple_rect);
        }

        // -- Snake body --
        for (i, seg) in self.body.iter().enumerate() {
            if (seg.x as u16) < cols && (seg.y as u16) < rows {
                let cell = Rect::new(
                    inner.x + seg.x as u16,
                    inner.y + seg.y as u16,
                    1,
                    1,
                );
                let color = if i == 0 {
                    Color::Rgb(100, 255, 100) // brighter green head
                } else {
                    Color::Green
                };
                let ch = if i == 0 { "█" } else { "▓" };
                let widget = Paragraph::new(ch)
                    .style(Style::default().fg(color).bg(Color::Rgb(20, 20, 30)));
                frame.render_widget(widget, cell);
            }
        }

        // -- Game Over overlay --
        if self.game_over {
            self.render_game_over(frame, area);
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

impl SnakeGame {
    /// Create a new game with an explicit grid size.
    fn new_with_size(width: u16, height: u16) -> Self {
        let w = if width > 4 { width } else { DEFAULT_W };
        let h = if height > 4 { height } else { DEFAULT_H };

        let start = Pos {
            x: w as i32 / 2,
            y: h as i32 / 2,
        };
        let mut body = VecDeque::new();
        body.push_back(start);
        body.push_back(Pos {
            x: start.x - 1,
            y: start.y,
        });
        body.push_back(Pos {
            x: start.x - 2,
            y: start.y,
        });

        let apple = spawn_apple(&body, w, h);

        SnakeGame {
            body,
            direction: Direction::Right,
            next_direction: Direction::Right,
            apple,
            width: w,
            height: h,
            score: 0,
            game_over: false,
            tick: 0,
        }
    }

    /// Render a centred "GAME OVER" popup.
    fn render_game_over(&self, frame: &mut Frame, area: Rect) {
        let popup_w: u16 = 40.min(area.width.saturating_sub(4));
        let popup_h: u16 = 7.min(area.height.saturating_sub(4));
        let popup = centered_rect(popup_w, popup_h, area);

        // Clear the area behind the popup.
        frame.render_widget(Clear, popup);

        let lines = vec![
            Line::from(Span::styled(
                "GAME OVER",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("Final score: {}", self.score),
                Style::default().fg(Color::Yellow),
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

/// Return a `Rect` of size `w x h` centred inside `area`.
fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}
