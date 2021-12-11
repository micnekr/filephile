mod actions;
mod directory_tree;
mod modes;
// mod ui;

use std::collections::BTreeMap;
use std::path::Path;
use std::{env, fs};
use std::{
    io,
    time::{Duration, Instant},
};

use actions::{NameMap, GLOBAL_ACTION_MAP, NORMAL_MODE_ACTION_MAP};
use crossterm::event::KeyCode;
use crossterm::{event::EnableMouseCapture, terminal::EnterAlternateScreen};
use modes::{Mode, ModeController, ModesManager, RecordedModifiable};
use serde::{Deserialize, Serialize};
use tui::layout::Constraint;
use tui::style::Style;

use tui::widgets::Borders;

use crate::directory_tree::FileTreeNode;

type StringMap = BTreeMap<String, String>;

#[derive(Serialize, Deserialize, Debug)]
struct AppSettings {
    global_key_bindings: StringMap,
    normal_mode_key_bindings: StringMap,
    search_mode_key_bindings: StringMap,
}
#[derive(Clone)]
pub struct StyleSet {
    pub file: Style,
    pub dir: Style,
}

struct InputReader {
    modifier_key_sequence: String,
    verb_key_sequence: String,
}

impl InputReader {
    fn clear(&mut self) {
        self.modifier_key_sequence.clear();
        self.verb_key_sequence.clear();
    }

    fn record_key_in_state(&mut self, key: KeyCode) {
        if let KeyCode::Char(character) = key {
            // if it is a modifier
            if character.is_digit(10) {
                self.modifier_key_sequence.push(character);

                // we can not add a movement after a verb, so fail in that case
                if !self.verb_key_sequence.is_empty() {
                    // TODO: maybe notify the user that this was an incorrect action
                    self.clear();
                }
            } else {
                self.verb_key_sequence.push(character);
            }
        }
        // TODO: keys like ESC and others
    }

    fn get_action_closure<'a, MapValueType>(
        &self,
        key_to_action_mapping: &'a BTreeMap<String, String>,
        action_to_closure_mapping: &'a NameMap<MapValueType>,
    ) -> Option<&'a MapValueType> {
        if let Some(action_name) = key_to_action_mapping.get(&self.verb_key_sequence) {
            return action_to_closure_mapping.get(action_name);
        }
        return None;
    }

    fn check_for_possible_extensions(&self, possiblities: Vec<&BTreeMap<String, String>>) -> bool {
        possiblities.iter().any(|map| {
            map.keys()
                .any(|string| string.starts_with(&self.verb_key_sequence))
        })
    }
}

#[derive(Clone)]
struct AppState {
    current_dir: directory_tree::FileTreeNode,
    // is_urgent_update: bool,
}

// TODO: check that it works on windows

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // setup terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = tui::backend::CrosstermBackend::new(stdout);
    let mut terminal = tui::Terminal::new(backend)?;

    let default_styles = StyleSet {
        file: Style::default()
            .bg(tui::style::Color::Black)
            .fg(tui::style::Color::White),
        dir: Style::default()
            .bg(tui::style::Color::Black)
            .fg(tui::style::Color::LightBlue),
    };

    let cursor_styles = StyleSet {
        file: Style::default()
            .bg(tui::style::Color::White)
            .fg(tui::style::Color::Black),
        // TODO: some way to show symlinks + where they are going
        dir: Style::default()
            .bg(tui::style::Color::White)
            .fg(tui::style::Color::Rgb(50, 50, 200)),
    };

    let modes_manager = ModesManager::new(default_styles, cursor_styles);
    let app_state = AppState {
        current_dir: FileTreeNode::new(env::current_dir()?.to_path_buf())?,
        // is_urgent_update: false,
        // NOTE: this would look good for multi-selection, maybe we should use it in the future
        // file: Style::default()
        //     .bg(tui::style::Color::DarkGray)
        //     .fg(tui::style::Color::White),
        // dir: Style::default()
        //     .bg(tui::style::Color::DarkGray)
        //     .fg(tui::style::Color::LightBlue),
    };

    let config = load_config(vec![
        "../example_config.toml",
        "/usr/share/fphile/global_config.toml",
    ])?;
    // create app and run it
    let res = run_loop(
        &mut terminal,
        app_state,
        modes_manager,
        config,
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
fn run_loop<B: tui::backend::Backend>(
    terminal: &mut tui::Terminal<B>,
    app_state: AppState,
    modes_manager: ModesManager,
    config: AppSettings,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();

    let mut app_state = app_state;

    let mut modes_manager = modes_manager;
    let mut input_reader = InputReader {
        modifier_key_sequence: String::new(),
        verb_key_sequence: String::new(),
    };
    loop {
        if modes_manager.get_current_mode() == &Mode::Quitting {
            return Ok(());
        }
        // if an urgent update, fore it to update ASAP by reducing wait time to 0
        let timeout = if modes_manager.has_been_modified() {
            Duration::from_secs(0)
        } else {
            tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0))
        };
        modes_manager.reset_modification_status();

        let poll_result = crossterm::event::poll(timeout)?;
        let mut dir_items = app_state.current_dir.clone().get_files()?;
        // collect the data once and then either use it for inputs or for outputs, but not both because of the potential to modify it
        // sort them

        modes_manager.sort(&mut dir_items);

        // TODO: redraw on change
        if poll_result {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                input_reader.record_key_in_state(key.code);
                // Look for key bindings in the current mode and execute them
                let have_executed_an_action = match modes_manager.get_current_mode() {
                    Mode::Quitting => false,
                    Mode::Normal => {
                        if let Some(closure) = input_reader.get_action_closure(
                            &config.normal_mode_key_bindings,
                            &NORMAL_MODE_ACTION_MAP,
                        ) {
                            closure(
                                &mut app_state,
                                &mut modes_manager.normal_mode_controller,
                                input_reader.modifier_key_sequence.parse().ok(),
                                &dir_items,
                            )?;
                            // TODO: maybe tell them if there was an error
                            true
                        } else {
                            false
                        }
                    }
                    Mode::Search => false,
                };
                // only look for the other keybinds if the first one has not been done
                let have_executed_an_action = have_executed_an_action
                    || if let Some(closure) = input_reader
                        .get_action_closure(&config.global_key_bindings, &GLOBAL_ACTION_MAP)
                    {
                        closure(
                            &mut app_state,
                            &mut modes_manager,
                            input_reader.modifier_key_sequence.parse().ok(),
                        )?;
                        // clear the sequence if it was successful or not
                        // TODO: maybe tell them if there was an error
                        true
                    } else {
                        false
                    };

                if have_executed_an_action {
                    input_reader.clear();
                } else {
                    // if there are no possible ways to continue the sequence
                    if !input_reader.check_for_possible_extensions(vec![
                        &config.global_key_bindings,
                        &config.normal_mode_key_bindings,
                        &config.search_mode_key_bindings,
                    ]) {
                        input_reader.clear();
                    }
                }
                // if let Err(err) = handle_inputs(key, app_state) {
                //     // TODO: see all places where there could be an error and set it
                //     // app_state.set_err(err);
                // }
            }
        } else {
            // Processes and draws the output
            // TODO: maybe make is_urgent_update not need to be set specifically and be inferred from changes
            terminal.draw(|f| {
                let bottom_text = modes_manager.get_bottom_text();

                let chunks = tui::layout::Layout::default()
                    .direction(tui::layout::Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Min(1),
                            Constraint::Length(if let Some(_) = bottom_text { 0 } else { 3 }),
                        ]
                        .as_ref(),
                    )
                    .split(f.size());

                if let Some(bottom_text) = bottom_text {
                    let block = tui::widgets::Block::default().borders(Borders::ALL);
                    bottom_text.block(block).render(f, chunks[1]);
                }

                {
                    let chunks = tui::layout::Layout::default()
                        .direction(tui::layout::Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
                        )
                        .split(chunks[0]);

                    let dir_path_string = app_state
                        .current_dir
                        .get_path()
                        .as_os_str()
                        .to_string_lossy()
                        .into_owned();

                    let block = tui::widgets::Block::default().borders(Borders::ALL);
                    let right_widget = modes_manager.get_right_ui(block, chunks[1]);
                    right_widget.render(f, chunks[1]);

                    let block = tui::widgets::Block::default()
                        .title(dir_path_string)
                        .borders(Borders::ALL);

                    let left_widget = modes_manager.get_left_ui(block, chunks[0], &dir_items);

                    left_widget.render(f, chunks[0]);
                }
            })?;
            // TODO: error management
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn load_config<P: AsRef<Path>>(paths: Vec<P>) -> io::Result<AppSettings> {
    let config = paths
        .iter()
        .find_map(|path| fs::read_to_string(path).ok())
        .ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "Could not find a config file",
        ))?;

    let config: AppSettings = toml::from_str(config.as_str())?;
    // app_state.global_key_sequence_to_action_mapping = config.global_key_bindings;
    Ok(config)
}
