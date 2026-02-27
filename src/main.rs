mod common;
mod games;

use std::io;

use clap::{Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Terminal;

use common::terminal::run_game;
use games::{GameChoice, Game};

#[derive(Parser)]
#[command(name = "cli-games", about = "10 terminal mini-games in Rust")]
struct Cli {
    #[command(subcommand)]
    game: Option<GameCommand>,
}

#[derive(Subcommand)]
enum GameCommand {
    Snake,
    Hangman,
    Wordle,
    #[command(name = "2048")]
    Twenty48,
    Blackjack,
    Minesweeper,
    Tetris,
    Pong,
    #[command(name = "typing-test")]
    TypingTest,
    Simon,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.game {
        Some(cmd) => {
            let choice = match cmd {
                GameCommand::Snake => GameChoice::Snake,
                GameCommand::Hangman => GameChoice::Hangman,
                GameCommand::Wordle => GameChoice::Wordle,
                GameCommand::Twenty48 => GameChoice::Twenty48,
                GameCommand::Blackjack => GameChoice::Blackjack,
                GameCommand::Minesweeper => GameChoice::Minesweeper,
                GameCommand::Tetris => GameChoice::Tetris,
                GameCommand::Pong => GameChoice::Pong,
                GameCommand::TypingTest => GameChoice::TypingTest,
                GameCommand::Simon => GameChoice::Simon,
            };
            launch_game(choice)
        }
        None => interactive_menu(),
    }
}

fn launch_game(choice: GameChoice) -> io::Result<()> {
    match choice {
        GameChoice::Snake => run_game(games::snake::SnakeGame::new()),
        GameChoice::Hangman => run_game(games::hangman::HangmanGame::new()),
        GameChoice::Wordle => run_game(games::wordle::WordleGame::new()),
        GameChoice::Twenty48 => run_game(games::twenty48::Twenty48Game::new()),
        GameChoice::Blackjack => run_game(games::blackjack::BlackjackGame::new()),
        GameChoice::Minesweeper => run_game(games::minesweeper::MinesweeperGame::new()),
        GameChoice::Tetris => run_game(games::tetris::TetrisGame::new()),
        GameChoice::Pong => run_game(games::pong::PongGame::new()),
        GameChoice::TypingTest => run_game(games::typing_test::TypingTestGame::new()),
        GameChoice::Simon => run_game(games::simon::SimonGame::new()),
    }
}

fn interactive_menu() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut selected: usize = 0;
    let choices = GameChoice::ALL;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let chunks = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

            let title = Paragraph::new(Line::from(vec![
                Span::styled(
                    " CLI Games ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
            frame.render_widget(title, chunks[0]);

            let items: Vec<ListItem> = choices
                .iter()
                .enumerate()
                .map(|(i, choice)| {
                    let style = if i == selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let prefix = if i == selected { " > " } else { "   " };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{}{}", prefix, choice.name()),
                            style,
                        ),
                        Span::styled(
                            format!("  {}", choice.description()),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]))
                })
                .collect();

            let list = List::new(items).block(
                Block::default()
                    .title(" Select a game ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            );
            frame.render_widget(list, chunks[1]);

            let help = Paragraph::new(Line::from(vec![
                Span::styled(" Up/Down ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("navigate  "),
                Span::styled(" Enter ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw("play  "),
                Span::styled(" Esc/q ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw("quit"),
            ]))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
            frame.render_widget(help, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => break,
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected > 0 {
                        selected -= 1;
                    } else {
                        selected = choices.len() - 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    selected = (selected + 1) % choices.len();
                }
                KeyCode::Enter => {
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;
                    return launch_game(choices[selected]);
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
