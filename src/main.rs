mod directory_tree;
mod logic;
mod ui;

use std::env;
use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{EnableMouseCapture, KeyEvent},
    terminal::EnterAlternateScreen,
};

use crate::directory_tree::FileTreeNode;

struct AppState {
    app_mode: AppMode,
    current_dir: directory_tree::FileTreeNode,
    opened_file: Option<String>,
    error_message: String,
}

#[derive(PartialEq)]
enum AppMode {
    NORMAL,
    VISUAL,
    QUITTING,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // setup terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = tui::backend::CrosstermBackend::new(stdout);
    let mut terminal = tui::Terminal::new(backend)?;

    let mut app_state = AppState {
        app_mode: AppMode::NORMAL,
        current_dir: FileTreeNode::new(env::current_dir()?.as_path())?,
        error_message: "".to_owned(),
        opened_file: None,
    };
    // create app and run it
    let res = run_loop(
        &mut terminal,
        ui::ui,
        logic::handle_inputs,
        &mut app_state,
        Duration::from_millis(250),
    );

    // restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}
fn run_loop<
    B: tui::backend::Backend,
    F1: Fn(&mut tui::Frame<B>, &mut AppState) -> io::Result<()>,
    F2: Fn(KeyEvent, &mut AppState) -> io::Result<()>,
>(
    terminal: &mut tui::Terminal<B>,
    ui: F1,
    handle_inputs: F2,
    app_state: &mut AppState,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        if app_state.app_mode == AppMode::QUITTING {
            return Ok(());
        }
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                handle_inputs(key, app_state)?;
            }
        }

        // TODO: do not redraw as frequently
        terminal.draw(|f| {
            if let Err(err) = ui(f, app_state) {
                app_state.error_message = err.to_string();
            }
        })?;

        if last_tick.elapsed() >= tick_rate {
            // app.on_tick();
            last_tick = Instant::now();
        }
    }
}
