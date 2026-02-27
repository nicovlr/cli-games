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
// Constants
// ---------------------------------------------------------------------------

const GRID_SIZE: usize = 4;
/// Width of each cell in characters (including internal padding).
const CELL_WIDTH: u16 = 8;
/// Height of each cell in rows.
const CELL_HEIGHT: u16 = 3;

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum GameState {
    Playing,
    Won,
    GameOver,
}

pub struct Twenty48Game {
    /// 4x4 grid. 0 means empty.
    grid: [[u32; GRID_SIZE]; GRID_SIZE],
    /// Current score (sum of all merged tile values).
    score: u32,
    /// Best score across restarts within the session.
    best: u32,
    /// Current game state.
    state: GameState,
    /// Whether the player has already seen the win message and chosen to continue.
    won_acknowledged: bool,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Return a color for a given tile value.
fn tile_color(value: u32) -> Color {
    match value {
        2 => Color::Rgb(238, 228, 218),
        4 => Color::Rgb(237, 224, 200),
        8 => Color::Rgb(242, 177, 121),
        16 => Color::Rgb(245, 149, 99),
        32 => Color::Rgb(246, 124, 95),
        64 => Color::Rgb(246, 94, 59),
        128 => Color::Rgb(237, 207, 114),
        256 => Color::Rgb(237, 204, 97),
        512 => Color::Rgb(237, 200, 80),
        1024 => Color::Rgb(237, 197, 63),
        2048 => Color::Rgb(237, 194, 46),
        4096 => Color::Rgb(60, 58, 50),
        8192 => Color::Rgb(50, 48, 40),
        _ => Color::Rgb(40, 38, 30),
    }
}

/// Text color contrasted against the tile background.
fn tile_fg(value: u32) -> Color {
    if value <= 4 {
        Color::Rgb(119, 110, 101)
    } else {
        Color::Rgb(249, 246, 242)
    }
}

/// Background color for the board.
fn board_bg() -> Color {
    Color::Rgb(187, 173, 160)
}

/// Background color for empty cells.
fn empty_cell_bg() -> Color {
    Color::Rgb(205, 193, 180)
}

/// Return a centred `Rect` of the given size inside `area`.
fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}

// ---------------------------------------------------------------------------
// Core game logic (pure, no UI)
// ---------------------------------------------------------------------------

impl Twenty48Game {
    /// Place a random tile (90 % chance of 2, 10 % chance of 4) on an empty cell.
    /// Returns false if there is no empty cell.
    fn spawn_tile(&mut self) -> bool {
        let mut empties = Vec::new();
        for r in 0..GRID_SIZE {
            for c in 0..GRID_SIZE {
                if self.grid[r][c] == 0 {
                    empties.push((r, c));
                }
            }
        }
        if empties.is_empty() {
            return false;
        }
        let mut rng = rand::thread_rng();
        let &(r, c) = &empties[rng.gen_range(0..empties.len())];
        self.grid[r][c] = if rng.gen::<f64>() < 0.9 { 2 } else { 4 };
        true
    }

    /// Slide and merge a single row to the left (in-place). Returns points earned.
    fn slide_row_left(row: &mut [u32; GRID_SIZE]) -> u32 {
        let mut points = 0u32;

        // 1. Compact non-zero values to the left.
        let mut compacted = [0u32; GRID_SIZE];
        let mut idx = 0;
        for i in 0..GRID_SIZE {
            if row[i] != 0 {
                compacted[idx] = row[i];
                idx += 1;
            }
        }

        // 2. Merge adjacent equal tiles (left to right, each tile merges at most once).
        let mut merged = [0u32; GRID_SIZE];
        let mut mi = 0;
        let mut i = 0;
        while i < GRID_SIZE {
            if compacted[i] == 0 {
                break;
            }
            if i + 1 < GRID_SIZE && compacted[i] == compacted[i + 1] {
                let val = compacted[i] * 2;
                merged[mi] = val;
                points += val;
                i += 2; // skip the merged partner
            } else {
                merged[mi] = compacted[i];
                i += 1;
            }
            mi += 1;
        }

        *row = merged;
        points
    }

    /// Perform a move in the given direction. Returns true if anything changed.
    fn do_move(&mut self, dir: Direction) -> bool {
        let old = self.grid;

        match dir {
            Direction::Left => {
                for r in 0..GRID_SIZE {
                    let pts = Self::slide_row_left(&mut self.grid[r]);
                    self.score += pts;
                }
            }
            Direction::Right => {
                for r in 0..GRID_SIZE {
                    self.grid[r].reverse();
                    let pts = Self::slide_row_left(&mut self.grid[r]);
                    self.score += pts;
                    self.grid[r].reverse();
                }
            }
            Direction::Up => {
                for c in 0..GRID_SIZE {
                    let mut col = Self::extract_col(&self.grid, c);
                    let pts = Self::slide_row_left(&mut col);
                    self.score += pts;
                    Self::put_col(&mut self.grid, c, &col);
                }
            }
            Direction::Down => {
                for c in 0..GRID_SIZE {
                    let mut col = Self::extract_col(&self.grid, c);
                    col.reverse();
                    let pts = Self::slide_row_left(&mut col);
                    self.score += pts;
                    col.reverse();
                    Self::put_col(&mut self.grid, c, &col);
                }
            }
        }

        self.grid != old
    }

    fn extract_col(grid: &[[u32; GRID_SIZE]; GRID_SIZE], c: usize) -> [u32; GRID_SIZE] {
        let mut col = [0u32; GRID_SIZE];
        for r in 0..GRID_SIZE {
            col[r] = grid[r][c];
        }
        col
    }

    fn put_col(grid: &mut [[u32; GRID_SIZE]; GRID_SIZE], c: usize, col: &[u32; GRID_SIZE]) {
        for r in 0..GRID_SIZE {
            grid[r][c] = col[r];
        }
    }

    /// Check whether any valid move exists.
    fn has_moves(&self) -> bool {
        for r in 0..GRID_SIZE {
            for c in 0..GRID_SIZE {
                if self.grid[r][c] == 0 {
                    return true;
                }
                // Check right neighbour.
                if c + 1 < GRID_SIZE && self.grid[r][c] == self.grid[r][c + 1] {
                    return true;
                }
                // Check bottom neighbour.
                if r + 1 < GRID_SIZE && self.grid[r][c] == self.grid[r + 1][c] {
                    return true;
                }
            }
        }
        false
    }

    /// Check whether any tile has reached 2048.
    fn has_won(&self) -> bool {
        for r in 0..GRID_SIZE {
            for c in 0..GRID_SIZE {
                if self.grid[r][c] >= 2048 {
                    return true;
                }
            }
        }
        false
    }

    /// After a move, update the game state accordingly.
    fn post_move(&mut self) {
        if self.score > self.best {
            self.best = self.score;
        }
        // Check win (only trigger overlay once).
        if !self.won_acknowledged && self.has_won() {
            self.state = GameState::Won;
            return;
        }
        // Check game over.
        if !self.has_moves() {
            self.state = GameState::GameOver;
        }
    }

    /// Reset the board for a new game, preserving the best score.
    fn restart(&mut self) {
        self.grid = [[0; GRID_SIZE]; GRID_SIZE];
        self.score = 0;
        self.state = GameState::Playing;
        self.won_acknowledged = false;
        self.spawn_tile();
        self.spawn_tile();
    }
}

// ---------------------------------------------------------------------------
// Direction enum
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

impl Game for Twenty48Game {
    fn new() -> Self {
        let mut game = Twenty48Game {
            grid: [[0; GRID_SIZE]; GRID_SIZE],
            score: 0,
            best: 0,
            state: GameState::Playing,
            won_acknowledged: false,
        };
        game.spawn_tile();
        game.spawn_tile();
        game
    }

    fn handle_event(&mut self, event: KeyEvent) -> bool {
        // --- Overlays first ---
        match self.state {
            GameState::Won => {
                match event.code {
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        // Continue playing past 2048.
                        self.won_acknowledged = true;
                        self.state = GameState::Playing;
                        return true;
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        self.restart();
                        return true;
                    }
                    KeyCode::Esc => return false,
                    _ => return true,
                }
            }
            GameState::GameOver => {
                match event.code {
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        self.restart();
                        return true;
                    }
                    KeyCode::Esc => return false,
                    _ => return true,
                }
            }
            GameState::Playing => {}
        }

        // --- Normal gameplay ---
        let dir = match event.code {
            KeyCode::Up | KeyCode::Char('w') => Some(Direction::Up),
            KeyCode::Down | KeyCode::Char('s') => Some(Direction::Down),
            KeyCode::Left | KeyCode::Char('a') => Some(Direction::Left),
            KeyCode::Right | KeyCode::Char('d') => Some(Direction::Right),
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.restart();
                return true;
            }
            KeyCode::Esc => return false,
            _ => None,
        };

        if let Some(d) = dir {
            let changed = self.do_move(d);
            if changed {
                self.spawn_tile();
                self.post_move();
            }
        }

        true
    }

    fn update(&mut self) {
        // 2048 is purely event-driven; nothing to do on tick.
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Outer chrome --------------------------------------------------
        let outer_block = Block::default()
            .title(" 2048 ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(187, 173, 160)));
        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Layout: header (score) | board | footer (help text).
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(inner);

        // --- Header: scores ---
        self.render_scores(frame, chunks[0]);

        // --- Board ---
        self.render_board(frame, chunks[1]);

        // --- Footer: help ---
        self.render_help(frame, chunks[2]);

        // --- Overlays ---
        match self.state {
            GameState::Won => self.render_overlay(
                frame,
                area,
                "YOU WIN!",
                Color::Rgb(237, 194, 46),
                &[
                    "You reached 2048!",
                    "",
                    "Press 'c' to continue playing",
                    "Press 'r' to restart",
                    "Press Esc to quit",
                ],
            ),
            GameState::GameOver => self.render_overlay(
                frame,
                area,
                "GAME OVER",
                Color::Red,
                &[
                    &format!("Final score: {}", self.score),
                    "",
                    "Press 'r' to restart",
                    "Press Esc to quit",
                ],
            ),
            GameState::Playing => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

impl Twenty48Game {
    fn render_scores(&self, frame: &mut Frame, area: Rect) {
        let p = Paragraph::new(Line::from(vec![
            Span::styled("  Score: ", Style::default().fg(Color::Rgb(187, 173, 160))),
            Span::styled(
                format!("{}", self.score),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled("Best: ", Style::default().fg(Color::Rgb(187, 173, 160))),
            Span::styled(
                format!("{}", self.best),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(143, 122, 102))),
        );
        frame.render_widget(p, area);
    }

    fn render_board(&self, frame: &mut Frame, area: Rect) {
        // Total board pixel size.
        let board_w = CELL_WIDTH * GRID_SIZE as u16 + (GRID_SIZE as u16 + 1); // +gaps
        let board_h = CELL_HEIGHT * GRID_SIZE as u16 + (GRID_SIZE as u16 + 1);

        let board_rect = centered_rect(board_w, board_h, area);

        // Fill board background.
        let bg = Paragraph::new("").style(Style::default().bg(board_bg()));
        frame.render_widget(bg, board_rect);

        // Draw each cell.
        for r in 0..GRID_SIZE {
            for c in 0..GRID_SIZE {
                let cell_x = board_rect.x + 1 + c as u16 * (CELL_WIDTH + 1);
                let cell_y = board_rect.y + 1 + r as u16 * (CELL_HEIGHT + 1);
                let cell_rect = Rect::new(cell_x, cell_y, CELL_WIDTH, CELL_HEIGHT);

                // Clip to board area to avoid drawing outside.
                if cell_rect.right() > board_rect.right() || cell_rect.bottom() > board_rect.bottom()
                {
                    continue;
                }

                let value = self.grid[r][c];
                if value == 0 {
                    let empty =
                        Paragraph::new("").style(Style::default().bg(empty_cell_bg()));
                    frame.render_widget(empty, cell_rect);
                } else {
                    let bg_color = tile_color(value);
                    let fg_color = tile_fg(value);
                    let label = format!("{}", value);

                    // Build lines: top padding, value centred, bottom padding.
                    let top_pad = (CELL_HEIGHT.saturating_sub(1)) / 2;
                    let mut lines: Vec<Line> = Vec::new();
                    for _ in 0..top_pad {
                        lines.push(Line::from(Span::styled(
                            " ".repeat(CELL_WIDTH as usize),
                            Style::default().bg(bg_color),
                        )));
                    }
                    // Centre the label within the cell width.
                    let pad_total = CELL_WIDTH as usize - label.len().min(CELL_WIDTH as usize);
                    let pad_left = pad_total / 2;
                    let pad_right = pad_total - pad_left;
                    let value_line = format!(
                        "{}{}{}",
                        " ".repeat(pad_left),
                        label,
                        " ".repeat(pad_right)
                    );
                    lines.push(Line::from(Span::styled(
                        value_line,
                        Style::default()
                            .fg(fg_color)
                            .bg(bg_color)
                            .add_modifier(Modifier::BOLD),
                    )));
                    // Fill remaining rows.
                    let remaining = CELL_HEIGHT as usize - lines.len();
                    for _ in 0..remaining {
                        lines.push(Line::from(Span::styled(
                            " ".repeat(CELL_WIDTH as usize),
                            Style::default().bg(bg_color),
                        )));
                    }

                    let p = Paragraph::new(lines);
                    frame.render_widget(p, cell_rect);
                }
            }
        }
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help = Paragraph::new(Line::from(vec![
            Span::styled(
                " Arrows ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("slide  "),
            Span::styled(
                "r ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("restart  "),
            Span::styled(
                "Esc ",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("quit"),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(help, area);
    }

    fn render_overlay(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        accent: Color,
        body_lines: &[&str],
    ) {
        let popup_w: u16 = 42.min(area.width.saturating_sub(4));
        let popup_h: u16 = (body_lines.len() as u16 + 4).min(area.height.saturating_sub(4));
        let popup = centered_rect(popup_w, popup_h, area);

        frame.render_widget(Clear, popup);

        let mut lines = vec![
            Line::from(Span::styled(
                title,
                Style::default()
                    .fg(accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];
        for &l in body_lines {
            lines.push(Line::from(Span::styled(
                l,
                Style::default().fg(Color::Gray),
            )));
        }

        let paragraph = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent))
                    .title(format!(" {} ", title))
                    .title_alignment(Alignment::Center),
            );

        frame.render_widget(paragraph, popup);
    }
}
