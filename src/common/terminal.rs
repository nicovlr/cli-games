use std::io::{self, stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::games::Game;

pub fn run_game<G: Game>(mut game: G) -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let result = game_loop(&mut terminal, &mut game);
    restore_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn game_loop<G: Game>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    game: &mut G,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| game.render(frame))?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.code == KeyCode::Esc {
                    break;
                }
                if !game.handle_event(key) {
                    break;
                }
            }
        }

        game.update();
    }
    Ok(())
}
