mod app;
mod config;
mod ui;

use std::{error::Error, io, time::Duration};

use app::{App, CommandAction, Mode, expand_user_path};
use config::load_from_default_location;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size},
};
use ratatui::{Terminal, backend::CrosstermBackend};

fn main() -> Result<(), Box<dyn Error>> {
    let mut app = bootstrap_app()?;
    run_tui(&mut app)
}

fn bootstrap_app() -> Result<App, Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut app = match args.next() {
        Some(first) if first == "--help" || first == "-h" => {
            println!("usage: tellus_42 [level.tlvl]");
            println!("without a path, the editor starts with a blank 32x18 level");
            std::process::exit(0);
        }
        Some(first) => App::from_path(expand_user_path(first))?,
        None => App::blank(32, 18, None)?,
    };

    match load_from_default_location() {
        Ok(Some(config)) => {
            if let Err(err) = app.apply_config(config) {
                app.set_status(format!("Error: {err}"));
            }
        }
        Ok(None) => {}
        Err(err) => app.set_status(format!("Error: {err}")),
    }

    Ok(app)
}

fn run_tui(app: &mut App) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = event_loop(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if handle_key(app, key)? {
            break;
        }
    }

    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool, Box<dyn Error>> {
    let result = match app.mode() {
        Mode::Command => handle_command_key(app, key),
        Mode::Insert => handle_insert_key(app, key),
        Mode::Normal => handle_normal_key(app, key),
    };

    match result {
        Ok(should_quit) => Ok(should_quit),
        Err(err) => {
            app.set_status(format!("Error: {err}"));
            Ok(false)
        }
    }
}

fn handle_normal_key(app: &mut App, key: KeyEvent) -> Result<bool, String> {
    let viewport_tiles = viewport_tiles(app);
    match key.code {
        KeyCode::Char(':') => app.begin_command(),
        KeyCode::Char('i') => app.enter_insert_mode(),
        KeyCode::Char('u') if !key.modifiers.contains(KeyModifiers::CONTROL) => app.undo()?,
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => app.redo()?,
        KeyCode::Char('h') | KeyCode::Left => app.move_cursor(-1, 0, viewport_tiles),
        KeyCode::Char('j') | KeyCode::Down => app.move_cursor(0, 1, viewport_tiles),
        KeyCode::Char('k') | KeyCode::Up => app.move_cursor(0, -1, viewport_tiles),
        KeyCode::Char('l') | KeyCode::Right => app.move_cursor(1, 0, viewport_tiles),
        KeyCode::Char('J') => app.cycle_layer(1),
        KeyCode::Char('K') => app.cycle_layer(-1),
        KeyCode::Char('+') | KeyCode::Char('=') => app.adjust_zoom(1, viewport_tiles),
        KeyCode::Char('-') => app.adjust_zoom(-1, viewport_tiles),
        KeyCode::Esc => app.enter_normal_mode(),
        _ => {}
    }
    Ok(false)
}

fn handle_insert_key(app: &mut App, key: KeyEvent) -> Result<bool, String> {
    let viewport_tiles = viewport_tiles(app);
    match key.code {
        KeyCode::Esc => app.enter_normal_mode(),
        KeyCode::Char('h') | KeyCode::Left => app.move_cursor(-1, 0, viewport_tiles),
        KeyCode::Char('j') | KeyCode::Down => app.move_cursor(0, 1, viewport_tiles),
        KeyCode::Char('k') | KeyCode::Up => app.move_cursor(0, -1, viewport_tiles),
        KeyCode::Char('l') | KeyCode::Right => app.move_cursor(1, 0, viewport_tiles),
        KeyCode::Char(ch) if ('1'..='9').contains(&ch) => {
            app.paint_digit(ch.to_digit(10).unwrap_or_default() as u16)?;
        }
        _ => {}
    }
    Ok(false)
}

fn handle_command_key(app: &mut App, key: KeyEvent) -> Result<bool, String> {
    match key.code {
        KeyCode::Esc => app.cancel_command(),
        KeyCode::Enter => {
            if let CommandAction::Quit = app.submit_command()? {
                return Ok(true);
            }
        }
        KeyCode::Backspace => app.command_backspace(),
        KeyCode::Char(ch) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                app.command_push(ch);
            }
        }
        _ => {}
    }
    Ok(false)
}

fn viewport_tiles(app: &App) -> (u16, u16) {
    let (term_w, term_h) = size().unwrap_or((120, 40));
    let width = term_w.saturating_sub(app.sidebar_width() + 2);
    let height = term_h.saturating_sub(app::COMMAND_HEIGHT + 2);
    let (tile_w, tile_h) = app.tile_size();
    ((width / tile_w.max(1)).max(1), (height / tile_h.max(1)).max(1))
}
