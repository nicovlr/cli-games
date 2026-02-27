use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::games::Game;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FIELD_W: usize = 10;
const FIELD_H: usize = 20;

/// Base ticks-per-drop at level 0. Decreases as level rises.
const BASE_DROP_TICKS: u32 = 30;
/// Minimum ticks-per-drop (speed cap).
const MIN_DROP_TICKS: u32 = 2;
/// Soft-drop moves every this many ticks.
const SOFT_DROP_TICKS: u32 = 2;

/// Each cell in the playing field is rendered as 2 characters wide so the
/// board looks roughly square in the terminal (characters are taller than wide).
const CELL_WIDTH: u16 = 2;

// ---------------------------------------------------------------------------
// Tetromino definitions
// ---------------------------------------------------------------------------

/// The 7 standard Tetris pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PieceKind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl PieceKind {
    const ALL: [PieceKind; 7] = [
        PieceKind::I,
        PieceKind::O,
        PieceKind::T,
        PieceKind::S,
        PieceKind::Z,
        PieceKind::J,
        PieceKind::L,
    ];

    fn color(self) -> Color {
        match self {
            PieceKind::I => Color::Cyan,
            PieceKind::O => Color::Yellow,
            PieceKind::T => Color::Magenta,
            PieceKind::S => Color::Green,
            PieceKind::Z => Color::Red,
            PieceKind::J => Color::Blue,
            PieceKind::L => Color::Rgb(255, 165, 0), // orange
        }
    }

    /// Returns the cells for each rotation state (0..4).
    /// Each cell is (row, col) relative to the piece origin.
    fn cells(self, rotation: u8) -> [(i32, i32); 4] {
        match self {
            PieceKind::I => match rotation % 4 {
                0 => [(0, 0), (0, 1), (0, 2), (0, 3)],
                1 => [(0, 2), (1, 2), (2, 2), (3, 2)],
                2 => [(2, 0), (2, 1), (2, 2), (2, 3)],
                3 => [(0, 1), (1, 1), (2, 1), (3, 1)],
                _ => unreachable!(),
            },
            PieceKind::O => [(0, 0), (0, 1), (1, 0), (1, 1)],
            PieceKind::T => match rotation % 4 {
                0 => [(0, 1), (1, 0), (1, 1), (1, 2)],
                1 => [(0, 1), (1, 1), (1, 2), (2, 1)],
                2 => [(1, 0), (1, 1), (1, 2), (2, 1)],
                3 => [(0, 1), (1, 0), (1, 1), (2, 1)],
                _ => unreachable!(),
            },
            PieceKind::S => match rotation % 4 {
                0 => [(0, 1), (0, 2), (1, 0), (1, 1)],
                1 => [(0, 1), (1, 1), (1, 2), (2, 2)],
                2 => [(1, 1), (1, 2), (2, 0), (2, 1)],
                3 => [(0, 0), (1, 0), (1, 1), (2, 1)],
                _ => unreachable!(),
            },
            PieceKind::Z => match rotation % 4 {
                0 => [(0, 0), (0, 1), (1, 1), (1, 2)],
                1 => [(0, 2), (1, 1), (1, 2), (2, 1)],
                2 => [(1, 0), (1, 1), (2, 1), (2, 2)],
                3 => [(0, 1), (1, 0), (1, 1), (2, 0)],
                _ => unreachable!(),
            },
            PieceKind::J => match rotation % 4 {
                0 => [(0, 0), (1, 0), (1, 1), (1, 2)],
                1 => [(0, 1), (0, 2), (1, 1), (2, 1)],
                2 => [(1, 0), (1, 1), (1, 2), (2, 2)],
                3 => [(0, 1), (1, 1), (2, 0), (2, 1)],
                _ => unreachable!(),
            },
            PieceKind::L => match rotation % 4 {
                0 => [(0, 2), (1, 0), (1, 1), (1, 2)],
                1 => [(0, 1), (1, 1), (2, 1), (2, 2)],
                2 => [(1, 0), (1, 1), (1, 2), (2, 0)],
                3 => [(0, 0), (0, 1), (1, 1), (2, 1)],
                _ => unreachable!(),
            },
        }
    }
}

fn random_piece() -> PieceKind {
    let mut rng = rand::thread_rng();
    PieceKind::ALL[rng.gen_range(0..7)]
}

// ---------------------------------------------------------------------------
// Active piece state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct ActivePiece {
    kind: PieceKind,
    rotation: u8,
    /// Row of the piece origin on the field (can be negative = above visible area).
    row: i32,
    /// Column of the piece origin on the field.
    col: i32,
}

impl ActivePiece {
    fn new(kind: PieceKind) -> Self {
        ActivePiece {
            kind,
            rotation: 0,
            row: -1,
            col: (FIELD_W as i32 - 4) / 2, // centred horizontally
        }
    }

    fn cells(&self) -> [(i32, i32); 4] {
        let base = self.kind.cells(self.rotation);
        let mut out = [(0i32, 0i32); 4];
        for i in 0..4 {
            out[i] = (base[i].0 + self.row, base[i].1 + self.col);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Playing field
// ---------------------------------------------------------------------------

/// None = empty, Some(Color) = filled with that piece's colour.
type Field = [[Option<Color>; FIELD_W]; FIELD_H];

fn empty_field() -> Field {
    [[None; FIELD_W]; FIELD_H]
}

/// Check whether all cells of the piece are in-bounds and unoccupied.
fn piece_fits(piece: &ActivePiece, field: &Field) -> bool {
    for (r, c) in piece.cells() {
        if c < 0 || c >= FIELD_W as i32 {
            return false;
        }
        if r >= FIELD_H as i32 {
            return false;
        }
        // Rows above the top are fine (piece spawning).
        if r >= 0 && field[r as usize][c as usize].is_some() {
            return false;
        }
    }
    true
}

/// Lock the piece into the field.
fn lock_piece(piece: &ActivePiece, field: &mut Field) {
    let color = piece.kind.color();
    for (r, c) in piece.cells() {
        if r >= 0 && r < FIELD_H as i32 && c >= 0 && c < FIELD_W as i32 {
            field[r as usize][c as usize] = Some(color);
        }
    }
}

/// Clear completed lines and return how many were cleared.
fn clear_lines(field: &mut Field) -> u32 {
    let mut cleared = 0u32;
    let mut write = FIELD_H;
    // Scan from bottom to top; copy non-full rows downward.
    for read in (0..FIELD_H).rev() {
        let full = field[read].iter().all(|c| c.is_some());
        if full {
            cleared += 1;
        } else {
            write -= 1;
            if write != read {
                field[write] = field[read];
            }
        }
    }
    // Fill the remaining top rows with empties.
    for r in 0..write {
        field[r] = [None; FIELD_W];
    }
    cleared
}

/// Compute the ghost (hard-drop preview) row offset.
fn ghost_piece(piece: &ActivePiece, field: &Field) -> ActivePiece {
    let mut ghost = *piece;
    while {
        let mut test = ghost;
        test.row += 1;
        piece_fits(&test, field)
    } {
        ghost.row += 1;
    }
    ghost
}

// ---------------------------------------------------------------------------
// SRS-lite wall kicks
// ---------------------------------------------------------------------------

/// Simplified SRS wall-kick offsets. Try the basic rotation first, then
/// nudge left/right/up by small amounts.
const KICK_OFFSETS: [(i32, i32); 5] = [
    (0, 0),
    (0, -1),
    (0, 1),
    (-1, 0),
    (-1, -1),
];

const KICK_OFFSETS_I: [(i32, i32); 5] = [
    (0, 0),
    (0, -2),
    (0, 2),
    (-1, 0),
    (1, 0),
];

fn try_rotate(piece: &ActivePiece, field: &Field) -> Option<ActivePiece> {
    if piece.kind == PieceKind::O {
        return None; // O doesn't rotate
    }
    let new_rot = (piece.rotation + 1) % 4;
    let kicks = if piece.kind == PieceKind::I {
        &KICK_OFFSETS_I
    } else {
        &KICK_OFFSETS
    };
    for &(dr, dc) in kicks {
        let candidate = ActivePiece {
            kind: piece.kind,
            rotation: new_rot,
            row: piece.row + dr,
            col: piece.col + dc,
        };
        if piece_fits(&candidate, field) {
            return Some(candidate);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

fn score_for_lines(n: u32, level: u32) -> u32 {
    let base = match n {
        1 => 100,
        2 => 300,
        3 => 500,
        4 => 800,
        _ => 0,
    };
    base * (level + 1)
}

fn ticks_per_drop(level: u32) -> u32 {
    let t = BASE_DROP_TICKS.saturating_sub(level * 3);
    t.max(MIN_DROP_TICKS)
}

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

pub struct TetrisGame {
    field: Field,
    current: ActivePiece,
    next: PieceKind,
    score: u32,
    level: u32,
    lines: u32,
    game_over: bool,
    tick: u32,
    soft_drop: bool,
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

impl Game for TetrisGame {
    fn new() -> Self {
        let current_kind = random_piece();
        let next = random_piece();
        TetrisGame {
            field: empty_field(),
            current: ActivePiece::new(current_kind),
            next,
            score: 0,
            level: 0,
            lines: 0,
            game_over: false,
            tick: 0,
            soft_drop: false,
        }
    }

    fn handle_event(&mut self, event: KeyEvent) -> bool {
        if self.game_over {
            return match event.code {
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    *self = TetrisGame::new();
                    true
                }
                KeyCode::Esc => false,
                _ => true,
            };
        }

        match event.code {
            // Move left
            KeyCode::Left => {
                let mut moved = self.current;
                moved.col -= 1;
                if piece_fits(&moved, &self.field) {
                    self.current = moved;
                }
            }
            // Move right
            KeyCode::Right => {
                let mut moved = self.current;
                moved.col += 1;
                if piece_fits(&moved, &self.field) {
                    self.current = moved;
                }
            }
            // Rotate
            KeyCode::Up => {
                if let Some(rotated) = try_rotate(&self.current, &self.field) {
                    self.current = rotated;
                }
            }
            // Soft drop (hold)
            KeyCode::Down => {
                self.soft_drop = true;
                // Immediately move down one row for responsiveness.
                let mut moved = self.current;
                moved.row += 1;
                if piece_fits(&moved, &self.field) {
                    self.current = moved;
                    self.score += 1; // soft-drop bonus
                }
            }
            // Hard drop
            KeyCode::Char(' ') => {
                let ghost = ghost_piece(&self.current, &self.field);
                let rows_dropped = (ghost.row - self.current.row).max(0) as u32;
                self.score += rows_dropped * 2;
                self.current = ghost;
                self.place_and_spawn();
            }
            // Restart
            KeyCode::Char('r') | KeyCode::Char('R') => {
                *self = TetrisGame::new();
            }
            // Quit
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

        let drop_interval = if self.soft_drop {
            SOFT_DROP_TICKS
        } else {
            ticks_per_drop(self.level)
        };

        // Reset soft_drop flag each tick; it is re-set by handle_event if the
        // key is still held.
        self.soft_drop = false;

        if self.tick < drop_interval {
            return;
        }
        self.tick = 0;

        // Try to move the piece down.
        let mut moved = self.current;
        moved.row += 1;
        if piece_fits(&moved, &self.field) {
            self.current = moved;
        } else {
            self.place_and_spawn();
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // --- Layout: [sidebar left] [board] [sidebar right] ---
        // Board needs FIELD_W * CELL_WIDTH + 2 (borders) wide, FIELD_H + 2 tall.
        let board_w = (FIELD_W as u16) * CELL_WIDTH + 2;
        let board_h = (FIELD_H as u16) + 2;
        let side_w: u16 = 16;
        let total_w = side_w + board_w + side_w;
        let total_h = board_h;

        // Centre everything in the terminal.
        let outer = centered_rect(total_w, total_h, area);

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(side_w),
                Constraint::Length(board_w),
                Constraint::Length(side_w),
            ])
            .split(outer);

        self.render_board(frame, columns[1]);
        self.render_info(frame, columns[0]);
        self.render_next(frame, columns[2]);

        if self.game_over {
            self.render_game_over(frame, area);
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

impl TetrisGame {
    /// Lock the current piece, clear lines, and spawn the next piece.
    fn place_and_spawn(&mut self) {
        lock_piece(&self.current, &mut self.field);

        let cleared = clear_lines(&mut self.field);
        if cleared > 0 {
            self.lines += cleared;
            self.score += score_for_lines(cleared, self.level);
            self.level = self.lines / 10;
        }

        // Spawn next piece.
        let new_piece = ActivePiece::new(self.next);
        self.next = random_piece();
        if piece_fits(&new_piece, &self.field) {
            self.current = new_piece;
        } else {
            self.current = new_piece;
            self.game_over = true;
        }
        self.tick = 0;
    }

    // -- Rendering helpers --------------------------------------------------

    fn render_board(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Tetris ")
            .title_alignment(Alignment::Center);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Background
        let bg = Paragraph::new("")
            .style(Style::default().bg(Color::Rgb(15, 15, 25)));
        frame.render_widget(bg, inner);

        // Ghost piece
        let ghost = ghost_piece(&self.current, &self.field);
        for (r, c) in ghost.cells() {
            if r >= 0 && r < FIELD_H as i32 && c >= 0 && c < FIELD_W as i32 {
                let cell_rect = Rect::new(
                    inner.x + (c as u16) * CELL_WIDTH,
                    inner.y + r as u16,
                    CELL_WIDTH,
                    1,
                );
                if cell_rect.right() <= inner.right() && cell_rect.bottom() <= inner.bottom() {
                    let w = Paragraph::new("░░")
                        .style(Style::default().fg(Color::DarkGray).bg(Color::Rgb(15, 15, 25)));
                    frame.render_widget(w, cell_rect);
                }
            }
        }

        // Locked cells
        for r in 0..FIELD_H {
            for c in 0..FIELD_W {
                if let Some(color) = self.field[r][c] {
                    let cell_rect = Rect::new(
                        inner.x + (c as u16) * CELL_WIDTH,
                        inner.y + r as u16,
                        CELL_WIDTH,
                        1,
                    );
                    if cell_rect.right() <= inner.right() && cell_rect.bottom() <= inner.bottom() {
                        let w = Paragraph::new("██")
                            .style(Style::default().fg(color).bg(Color::Rgb(15, 15, 25)));
                        frame.render_widget(w, cell_rect);
                    }
                }
            }
        }

        // Current piece
        let color = self.current.kind.color();
        for (r, c) in self.current.cells() {
            if r >= 0 && r < FIELD_H as i32 && c >= 0 && c < FIELD_W as i32 {
                let cell_rect = Rect::new(
                    inner.x + (c as u16) * CELL_WIDTH,
                    inner.y + r as u16,
                    CELL_WIDTH,
                    1,
                );
                if cell_rect.right() <= inner.right() && cell_rect.bottom() <= inner.bottom() {
                    let w = Paragraph::new("██")
                        .style(Style::default().fg(color).bg(Color::Rgb(15, 15, 25)));
                    frame.render_widget(w, cell_rect);
                }
            }
        }
    }

    fn render_info(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Info ")
            .title_alignment(Alignment::Center);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::from(Span::styled(
                "SCORE",
                Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("{}", self.score),
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "LEVEL",
                Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("{}", self.level),
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "LINES",
                Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("{}", self.lines),
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "CONTROLS",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled("</>  Move", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled(" ^   Rotate", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled(" v   Soft", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("Spc  Hard", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled(" r   Reset", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("Esc  Quit", Style::default().fg(Color::DarkGray))),
        ];

        let p = Paragraph::new(lines).alignment(Alignment::Center);
        frame.render_widget(p, inner);
    }

    fn render_next(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Next ")
            .title_alignment(Alignment::Center);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Draw the next piece in a small preview area.
        let preview_cells = PieceKind::cells(self.next, 0);
        let color = self.next.color();

        // Find bounding box to centre the preview.
        let min_r = preview_cells.iter().map(|c| c.0).min().unwrap_or(0);
        let max_r = preview_cells.iter().map(|c| c.0).max().unwrap_or(0);
        let min_c = preview_cells.iter().map(|c| c.1).min().unwrap_or(0);
        let max_c = preview_cells.iter().map(|c| c.1).max().unwrap_or(0);

        let piece_h = (max_r - min_r + 1) as u16;
        let piece_w = ((max_c - min_c + 1) as u16) * CELL_WIDTH;

        let offset_y = inner.y + inner.height.saturating_sub(piece_h) / 2;
        let offset_x = inner.x + inner.width.saturating_sub(piece_w) / 2;

        for (r, c) in preview_cells {
            let py = offset_y + (r - min_r) as u16;
            let px = offset_x + ((c - min_c) as u16) * CELL_WIDTH;
            let cell_rect = Rect::new(px, py, CELL_WIDTH, 1);
            if cell_rect.right() <= inner.right() && cell_rect.bottom() <= inner.bottom() {
                let w = Paragraph::new("██")
                    .style(Style::default().fg(color));
                frame.render_widget(w, cell_rect);
            }
        }
    }

    fn render_game_over(&self, frame: &mut Frame, area: Rect) {
        let popup_w: u16 = 34.min(area.width.saturating_sub(4));
        let popup_h: u16 = 9.min(area.height.saturating_sub(4));
        let popup = centered_rect(popup_w, popup_h, area);

        frame.render_widget(Clear, popup);

        let lines = vec![
            Line::from(Span::styled(
                "GAME OVER",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("Score: {}", self.score),
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::styled(
                format!("Level: {}  Lines: {}", self.level, self.lines),
                Style::default().fg(Color::Yellow),
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
                    .border_style(Style::default().fg(Color::Red))
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
