mod directory_tree;
mod inputs;
mod ui;

use std::collections::BTreeSet;
use std::env;
use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{EnableMouseCapture, KeyEvent},
    terminal::EnterAlternateScreen,
};
use directory_tree::{FileSelection, FileTreeNodeSorter};
use tui::style::Style;

use crate::directory_tree::{FileSelectionSingle, FileTreeNode};

struct AppState {
    app_mode: AppMode,

    current_dir: directory_tree::FileTreeNode,
    opened_file: Option<String>,
    error_message: String,

    verb_key_sequence: String,
    modifier_key_sequence: String,

    file_cursor: FileSelectionSingle,
    file_tree_node_sorter: FileTreeNodeSorter,

    is_urgent_update: bool,
}

impl AppState {
    pub fn set_err(&mut self, err: io::Error) {
        self.error_message = err.to_string();
        self.is_urgent_update = true;
    }
}

#[derive(PartialEq)]
enum AppMode {
    NORMAL,
    // VISUAL,
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
        current_dir: FileTreeNode::new(env::current_dir()?.to_path_buf())?,
        opened_file: None,
        error_message: "".to_owned(),
        file_cursor: FileSelectionSingle {
            selected_file: None,
            style: Style::default()
                .bg(tui::style::Color::White)
                .fg(tui::style::Color::Black),
        },
        file_tree_node_sorter: FileTreeNodeSorter::NORMAL,

        verb_key_sequence: String::from(""),
        modifier_key_sequence: String::from(""),

        is_urgent_update: false,
    };

    // create app and run it
    let res = run_loop(
        &mut terminal,
        ui::ui,
        inputs::handle_inputs,
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
        let timeout = if app_state.is_urgent_update {
            app_state.is_urgent_update = false;
            Duration::from_secs(0)
        } else {
            tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0))
        };
        if crossterm::event::poll(timeout)? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                handle_inputs(key, app_state)?;
            }
        }

        // TODO: do not redraw as frequently
        terminal.draw(|f| {
            if let Err(err) = ui(f, app_state) {
                app_state.set_err(err);
            }
        })?;

        if last_tick.elapsed() >= tick_rate {
            // app.on_tick();
            last_tick = Instant::now();
        }
    }
}
