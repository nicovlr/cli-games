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
use std::time::Instant;

use crate::games::Game;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_ROWS: usize = 16;
const DEFAULT_COLS: usize = 16;
const DEFAULT_MINES: usize = 40;

/// Each cell occupies 3 characters wide so that the grid looks square-ish in
/// a terminal (characters are roughly 2x taller than they are wide).
const CELL_WIDTH: u16 = 3;
const CELL_HEIGHT: u16 = 1;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellState {
    Hidden,
    Revealed,
    Flagged,
}

#[derive(Debug, Clone, Copy)]
struct Cell {
    mine: bool,
    state: CellState,
    adjacent: u8,
}

impl Cell {
    fn new() -> Self {
        Cell {
            mine: false,
            state: CellState::Hidden,
            adjacent: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameState {
    /// Before the first reveal.
    Ready,
    /// Playing.
    Playing,
    /// Hit a mine.
    Lost,
    /// All non-mine cells revealed.
    Won,
}

// ---------------------------------------------------------------------------
// Game struct
// ---------------------------------------------------------------------------

pub struct MinesweeperGame {
    rows: usize,
    cols: usize,
    total_mines: usize,
    grid: Vec<Vec<Cell>>,
    cursor_row: usize,
    cursor_col: usize,
    state: GameState,
    start_time: Option<Instant>,
    elapsed_secs: u64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the 8-connected neighbours of (r, c) that lie within the grid.
fn neighbours(r: usize, c: usize, rows: usize, cols: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(8);
    for dr in [-1i32, 0, 1] {
        for dc in [-1i32, 0, 1] {
            if dr == 0 && dc == 0 {
                continue;
            }
            let nr = r as i32 + dr;
            let nc = c as i32 + dc;
            if nr >= 0 && nr < rows as i32 && nc >= 0 && nc < cols as i32 {
                out.push((nr as usize, nc as usize));
            }
        }
    }
    out
}

/// Colour used for the number labels 1-8.
fn number_color(n: u8) -> Color {
    match n {
        1 => Color::Blue,
        2 => Color::Green,
        3 => Color::Red,
        4 => Color::Magenta,
        5 => Color::Rgb(128, 0, 0),   // dark red / maroon
        6 => Color::Cyan,
        7 => Color::White,
        8 => Color::DarkGray,
        _ => Color::White,
    }
}

/// Return a `Rect` of size `w x h` centred inside `area`.
fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}

// ---------------------------------------------------------------------------
// Core logic
// ---------------------------------------------------------------------------

impl MinesweeperGame {
    /// Build a fresh board with no mines placed yet (mines are placed on the
    /// first reveal so that the first click is always safe).
    fn create_board(rows: usize, cols: usize) -> Vec<Vec<Cell>> {
        vec![vec![Cell::new(); cols]; rows]
    }

    /// Place mines randomly, avoiding `safe_r, safe_c` and its neighbours
    /// so the first reveal always opens a region.
    fn place_mines(&mut self, safe_r: usize, safe_c: usize) {
        let mut rng = rand::thread_rng();
        let safe_set: Vec<(usize, usize)> = {
            let mut s = vec![(safe_r, safe_c)];
            s.extend(neighbours(safe_r, safe_c, self.rows, self.cols));
            s
        };

        let mut placed = 0;
        while placed < self.total_mines {
            let r = rng.gen_range(0..self.rows);
            let c = rng.gen_range(0..self.cols);
            if self.grid[r][c].mine || safe_set.contains(&(r, c)) {
                continue;
            }
            self.grid[r][c].mine = true;
            placed += 1;
        }

        // Compute adjacency counts.
        for r in 0..self.rows {
            for c in 0..self.cols {
                if self.grid[r][c].mine {
                    continue;
                }
                let count = neighbours(r, c, self.rows, self.cols)
                    .iter()
                    .filter(|&&(nr, nc)| self.grid[nr][nc].mine)
                    .count() as u8;
                self.grid[r][c].adjacent = count;
            }
        }
    }

    /// Reveal a cell. If it is a zero, flood-fill to reveal the connected
    /// region of zeroes and their numbered border.
    fn reveal(&mut self, r: usize, c: usize) {
        if self.grid[r][c].state != CellState::Hidden {
            return;
        }

        // First reveal: place mines.
        if self.state == GameState::Ready {
            self.place_mines(r, c);
            self.state = GameState::Playing;
            self.start_time = Some(Instant::now());
        }

        // Hit a mine.
        if self.grid[r][c].mine {
            self.grid[r][c].state = CellState::Revealed;
            self.state = GameState::Lost;
            // Reveal all mines.
            for row in self.grid.iter_mut() {
                for cell in row.iter_mut() {
                    if cell.mine {
                        cell.state = CellState::Revealed;
                    }
                }
            }
            return;
        }

        // BFS flood fill.
        let mut queue = VecDeque::new();
        queue.push_back((r, c));
        self.grid[r][c].state = CellState::Revealed;

        while let Some((cr, cc)) = queue.pop_front() {
            if self.grid[cr][cc].adjacent == 0 {
                for (nr, nc) in neighbours(cr, cc, self.rows, self.cols) {
                    if self.grid[nr][nc].state == CellState::Hidden && !self.grid[nr][nc].mine {
                        self.grid[nr][nc].state = CellState::Revealed;
                        queue.push_back((nr, nc));
                    }
                }
            }
        }

        // Check win condition.
        self.check_win();
    }

    /// Toggle a flag on a hidden cell.
    fn toggle_flag(&mut self, r: usize, c: usize) {
        match self.grid[r][c].state {
            CellState::Hidden => self.grid[r][c].state = CellState::Flagged,
            CellState::Flagged => self.grid[r][c].state = CellState::Hidden,
            CellState::Revealed => {}
        }
    }

    /// Check whether the player has won.
    fn check_win(&mut self) {
        let total_cells = self.rows * self.cols;
        let revealed = self
            .grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|c| c.state == CellState::Revealed)
            .count();
        if revealed == total_cells - self.total_mines {
            self.state = GameState::Won;
            // Auto-flag remaining mines.
            for row in self.grid.iter_mut() {
                for cell in row.iter_mut() {
                    if cell.mine && cell.state == CellState::Hidden {
                        cell.state = CellState::Flagged;
                    }
                }
            }
        }
    }

    /// Count of currently placed flags.
    fn flag_count(&self) -> usize {
        self.grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|c| c.state == CellState::Flagged)
            .count()
    }

    /// Render the end-game overlay (win or lose).
    fn render_overlay(&self, frame: &mut Frame, area: Rect) {
        let (title, message, color) = match self.state {
            GameState::Won => (" You Win! ", "All mines cleared!", Color::Green),
            GameState::Lost => (" Game Over ", "You hit a mine!", Color::Red),
            _ => return,
        };

        let popup_w: u16 = 42.min(area.width.saturating_sub(4));
        let popup_h: u16 = 7.min(area.height.saturating_sub(4));
        let popup = centered_rect(popup_w, popup_h, area);

        frame.render_widget(Clear, popup);

        let lines = vec![
            Line::from(Span::styled(
                message,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("Time: {}s", self.elapsed_secs),
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
                    .border_style(Style::default().fg(color))
                    .title(title)
                    .title_alignment(Alignment::Center),
            );

        frame.render_widget(paragraph, popup);
    }
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

impl Game for MinesweeperGame {
    fn new() -> Self {
        MinesweeperGame {
            rows: DEFAULT_ROWS,
            cols: DEFAULT_COLS,
            total_mines: DEFAULT_MINES,
            grid: MinesweeperGame::create_board(DEFAULT_ROWS, DEFAULT_COLS),
            cursor_row: DEFAULT_ROWS / 2,
            cursor_col: DEFAULT_COLS / 2,
            state: GameState::Ready,
            start_time: None,
            elapsed_secs: 0,
        }
    }

    fn handle_event(&mut self, event: KeyEvent) -> bool {
        // If the game is over, only accept restart / quit.
        if self.state == GameState::Won || self.state == GameState::Lost {
            match event.code {
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    *self = MinesweeperGame::new();
                    return true;
                }
                KeyCode::Esc => return false,
                _ => return true,
            }
        }

        match event.code {
            // Movement
            KeyCode::Up => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                }
            }
            KeyCode::Down => {
                if self.cursor_row + 1 < self.rows {
                    self.cursor_row += 1;
                }
            }
            KeyCode::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_col + 1 < self.cols {
                    self.cursor_col += 1;
                }
            }

            // Reveal
            KeyCode::Enter | KeyCode::Char(' ') => {
                let r = self.cursor_row;
                let c = self.cursor_col;
                if self.grid[r][c].state != CellState::Flagged {
                    self.reveal(r, c);
                }
            }

            // Flag
            KeyCode::Char('f') | KeyCode::Char('F') => {
                let r = self.cursor_row;
                let c = self.cursor_col;
                self.toggle_flag(r, c);
            }

            // Restart
            KeyCode::Char('r') | KeyCode::Char('R') => {
                *self = MinesweeperGame::new();
            }

            // Quit
            KeyCode::Esc => return false,

            _ => {}
        }
        true
    }

    fn update(&mut self) {
        // Update the elapsed timer while playing.
        if self.state == GameState::Playing {
            if let Some(start) = self.start_time {
                self.elapsed_secs = start.elapsed().as_secs();
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // -----------------------------------------------------------------
        // Status bar height: 1 line for info.
        // -----------------------------------------------------------------
        let status_height: u16 = 1;
        let grid_pixel_w = self.cols as u16 * CELL_WIDTH;
        let grid_pixel_h = self.rows as u16 * CELL_HEIGHT;

        // Total inner area needed (grid + status + a bit of padding).
        let needed_inner_w = grid_pixel_w;
        let _needed_inner_h = grid_pixel_h + status_height + 1; // +1 for spacing

        // Outer block.
        let title = " Minesweeper ";
        let outer_block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Dark background.
        let bg = Paragraph::new("").style(Style::default().bg(Color::Rgb(20, 20, 30)));
        frame.render_widget(bg, inner);

        // -----------------------------------------------------------------
        // Status line: mines remaining, timer, controls hint.
        // -----------------------------------------------------------------
        let mines_remaining = self.total_mines as i32 - self.flag_count() as i32;
        let timer_display = match self.state {
            GameState::Ready => 0,
            _ => self.elapsed_secs,
        };
        let state_indicator = match self.state {
            GameState::Ready => ":-)",
            GameState::Playing => ":-)",
            GameState::Won => "B-)",
            GameState::Lost => "X-(",
        };

        let status_line = Line::from(vec![
            Span::styled(
                format!(" Mines: {:>3} ", mines_remaining),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} ", state_indicator),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(" Time: {:>4}s ", timer_display),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                " Arrows:move  Enter:reveal  f:flag  r:restart  Esc:quit ",
                Style::default().fg(Color::DarkGray),
            ),
        ]);

        let status_rect = Rect::new(inner.x, inner.y, inner.width, status_height);
        frame.render_widget(Paragraph::new(status_line), status_rect);

        // -----------------------------------------------------------------
        // Grid area: centre the grid within the remaining space.
        // -----------------------------------------------------------------
        let grid_area_y = inner.y + status_height + 1;
        let grid_area_h = inner.height.saturating_sub(status_height + 1);

        // Centre horizontally.
        let offset_x = if inner.width > needed_inner_w {
            (inner.width - needed_inner_w) / 2
        } else {
            0
        };
        // Centre vertically.
        let offset_y = if grid_area_h > grid_pixel_h {
            (grid_area_h - grid_pixel_h) / 2
        } else {
            0
        };

        let grid_origin_x = inner.x + offset_x;
        let grid_origin_y = grid_area_y + offset_y;

        // Draw cells.
        for r in 0..self.rows {
            let py = grid_origin_y + r as u16 * CELL_HEIGHT;
            if py >= inner.y + inner.height {
                break;
            }

            let mut spans: Vec<Span> = Vec::with_capacity(self.cols);

            for c in 0..self.cols {
                let cell = &self.grid[r][c];
                let is_cursor = r == self.cursor_row && c == self.cursor_col;

                let (text, fg, bg_color) = match cell.state {
                    CellState::Hidden => {
                        if is_cursor {
                            (" # ", Color::White, Color::Rgb(80, 80, 120))
                        } else {
                            (" # ", Color::Rgb(140, 140, 160), Color::Rgb(50, 50, 70))
                        }
                    }
                    CellState::Flagged => {
                        if is_cursor {
                            (" F ", Color::Red, Color::Rgb(80, 80, 120))
                        } else {
                            (" F ", Color::Red, Color::Rgb(50, 50, 70))
                        }
                    }
                    CellState::Revealed => {
                        if cell.mine {
                            // Mine -- show differently if it was the one the
                            // player hit (cursor) vs revealed on game over.
                            if is_cursor {
                                (" * ", Color::White, Color::Rgb(180, 30, 30))
                            } else {
                                (" * ", Color::Red, Color::Rgb(40, 40, 50))
                            }
                        } else if cell.adjacent == 0 {
                            if is_cursor {
                                ("   ", Color::White, Color::Rgb(60, 60, 90))
                            } else {
                                ("   ", Color::White, Color::Rgb(30, 30, 40))
                            }
                        } else {
                            // Number.
                            let fg_col = number_color(cell.adjacent);
                            let bg_col = if is_cursor {
                                Color::Rgb(60, 60, 90)
                            } else {
                                Color::Rgb(30, 30, 40)
                            };
                            // We need an owned string for the number, but
                            // Span can hold &str. We'll build the span
                            // below instead.
                            let _ = (fg_col, bg_col);
                            // Push directly and continue.
                            let num_str = format!(" {} ", cell.adjacent);
                            spans.push(Span::styled(
                                num_str,
                                Style::default()
                                    .fg(fg_col)
                                    .bg(bg_col)
                                    .add_modifier(Modifier::BOLD),
                            ));
                            continue;
                        }
                    }
                };

                let mut style = Style::default().fg(fg).bg(bg_color);
                if is_cursor {
                    style = style.add_modifier(Modifier::BOLD);
                }
                spans.push(Span::styled(text, style));
            }

            let line = Line::from(spans);
            let row_rect = Rect::new(
                grid_origin_x,
                py,
                (self.cols as u16 * CELL_WIDTH).min(inner.width),
                CELL_HEIGHT,
            );
            frame.render_widget(Paragraph::new(line), row_rect);
        }

        // -----------------------------------------------------------------
        // End-game overlay.
        // -----------------------------------------------------------------
        if self.state == GameState::Won || self.state == GameState::Lost {
            self.render_overlay(frame, area);
        }
    }
}
