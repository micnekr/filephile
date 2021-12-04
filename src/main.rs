mod actions;
mod directory_tree;
mod inputs;
mod ui;

use std::collections::BTreeMap;
use std::path::Path;
use std::{env, fs};
use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{EnableMouseCapture, KeyEvent},
    terminal::EnterAlternateScreen,
};
use directory_tree::FileTreeNodeSorter;
use serde::{Deserialize, Serialize};
use tui::style::Style;

use ui::StyleSet;

use crate::directory_tree::{FileSelectionSingle, FileTreeNode};

#[derive(Serialize, Deserialize, Debug)]
struct AppSettings {
    key_bindings: BTreeMap<String, String>,
}

struct AppState {
    app_mode: Mode,

    current_dir: directory_tree::FileTreeNode,
    // opened_file: Option<String>,
    error_message: String,

    verb_key_sequence: String,
    modifier_key_sequence: String,

    file_cursor: FileSelectionSingle,
    file_tree_node_sorter: FileTreeNodeSorter,

    default_style_set: StyleSet,

    key_sequence_to_action_mapping: BTreeMap<String, String>,

    max_distance_from_cursor_to_bottom: usize,

    is_urgent_update: bool,
}

impl AppState {
    pub(crate) fn set_err(&mut self, err: io::Error) {
        self.error_message = err.to_string();
        self.is_urgent_update = true;
    }
}

#[derive(PartialEq)]
enum Mode {
    NORMAL,
    SEARCH,
    QUITTING,
}

// TODO: check that it works on windows

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // setup terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = tui::backend::CrosstermBackend::new(stdout);
    let mut terminal = tui::Terminal::new(backend)?;

    let mut app_state = AppState {
        app_mode: Mode::NORMAL,
        current_dir: FileTreeNode::new(env::current_dir()?.to_path_buf())?,
        // opened_file: None,
        error_message: "".to_owned(),
        file_cursor: FileSelectionSingle {
            selected_file: None,
            styles: StyleSet {
                file: Style::default()
                    .bg(tui::style::Color::White)
                    .fg(tui::style::Color::Black),
                // TODO: some way to show symlinks + where they are going
                dir: Style::default()
                    .bg(tui::style::Color::White)
                    .fg(tui::style::Color::Rgb(50, 50, 200)),
            },
        },
        file_tree_node_sorter: FileTreeNodeSorter::NORMAL,

        default_style_set: StyleSet {
            file: Style::default()
                .bg(tui::style::Color::Black)
                .fg(tui::style::Color::White),
            dir: Style::default()
                .bg(tui::style::Color::Black)
                .fg(tui::style::Color::LightBlue),
        },

        verb_key_sequence: String::from(""),
        modifier_key_sequence: String::from(""),

        key_sequence_to_action_mapping: BTreeMap::new(),

        max_distance_from_cursor_to_bottom: 4,

        is_urgent_update: false,
        // NOTE: this would look good for multi-selection, maybe we should use it in the future
        // file: Style::default()
        //     .bg(tui::style::Color::DarkGray)
        //     .fg(tui::style::Color::White),
        // dir: Style::default()
        //     .bg(tui::style::Color::DarkGray)
        //     .fg(tui::style::Color::LightBlue),
    };

    load_config(
        &mut app_state,
        vec![
            "/usr/share/fphile/global_config.toml",
            "../example_config.toml",
        ],
    )?;
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
        println!("Exiting because of an error: {:?}", err)
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
        if app_state.app_mode == Mode::QUITTING {
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

fn load_config<P: AsRef<Path>>(app_state: &mut AppState, paths: Vec<P>) -> io::Result<()> {
    let config = paths
        .iter()
        .find_map(|path| fs::read_to_string(path).ok())
        .ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "Could not find a config file",
        ))?;

    let config: AppSettings = toml::from_str(config.as_str())?;
    app_state.key_sequence_to_action_mapping = config.key_bindings;
    Ok(())
}
