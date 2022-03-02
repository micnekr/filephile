mod actions;
mod compile_time_settings;
mod directory_tree;
mod helper_types;
mod modes;

use std::env;
use std::io::Stdout;
use std::{
    io,
    time::{Duration, Instant},
};

use actions::{ActionData, GLOBAL_ACTION_MAP, NORMAL_MODE_ACTION_MAP, SEARCH_MODE_ACTION_MAP};
use crossterm::event::KeyCode;
use crossterm::{event::EnableMouseCapture, terminal::EnterAlternateScreen};
use helper_types::{AppSettings, AppState, NormalModeState, SearchModeState, StyleSet};
use modes::{get_file_text_preview, Mode};
use tui::backend::{Backend, CrosstermBackend};
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::Style;
use tui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use tui::Terminal;

use crate::directory_tree::FileTreeNode;
use crate::helper_types::TrackedModifiable;
use crate::modes::cmp_by_dir_and_path;

pub type CustomTerminal = Terminal<CrosstermBackend<Stdout>>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppSettings::load_config(vec![
        "../example_config.toml",
        "/usr/share/fphile/global_config.toml",
    ])?;

    let current_dir = FileTreeNode::new(
        env::current_dir()
            .expect("Could not get the current directory")
            .to_path_buf(),
    );

    assert!(
        current_dir.is_dir(),
        "The current directory should be a directory"
    );

    // setup terminal
    let backend = tui::backend::CrosstermBackend::new(io::stdout());
    let mut terminal = tui::Terminal::new(backend)?;

    let app_state = TrackedModifiable::new(AppState::new(current_dir));
    enter_captured_mode(&mut terminal).expect("Could not capture the terminal");

    // create app and run it
    let res = run_loop(
        app_state,
        &mut terminal,
        Duration::from_millis(config.render_timeout.unwrap_or(250)),
        config,
    );

    exit_captured_mode(&mut terminal).expect("Could not capture the terminal");

    if let Err(err) = res {
        println!("Exiting because of an error: {:?}", err)
    }

    Ok(())
}

pub fn enter_captured_mode(terminal: &mut CustomTerminal) -> io::Result<()> {
    crossterm::execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    crossterm::terminal::enable_raw_mode()?;
    terminal.hide_cursor()?;
    // make the terminal redraw everything on the next draw to get rid of the artifacts
    terminal.clear()?;
    Ok(())
}

pub fn exit_captured_mode(terminal: &mut CustomTerminal) -> io::Result<()> {
    // restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_loop(
    mut app_state: TrackedModifiable<AppState>,
    terminal: &mut CustomTerminal,
    tick_rate: Duration,
    config: AppSettings,
) -> io::Result<()> {
    let mut last_tick = Instant::now();

    loop {
        if app_state.mode == Mode::Quitting {
            return Ok(());
        }
        // if an urgent update, fore it to update ASAP by reducing wait time to 0
        let timeout = if app_state.is_modified() {
            app_state.reset_modified_flag();
            Duration::from_secs(0)
        } else {
            tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0))
        };

        let mut dir_items = app_state.current_dir.list_files().unwrap_or_else(|err| {
            match err.kind() {
                io::ErrorKind::PermissionDenied => {
                    app_state.get_mut().error_popup(
                        String::from("Permission denied"),
                        String::from(
                            "You do not have the permissions to access this folder/directory",
                        ),
                    );
                }
                // unknown error
                _ => {
                    app_state.get_mut().error_popup(
                        String::from("Unknown error"),
                        String::from("An error occurred while reading the files"),
                    );
                }
            }
            vec![]
        });

        // sort
        let dir_items = match app_state.mode {
            Mode::Quitting => unreachable!(),
            Mode::Normal => {
                dir_items.sort_by(|a, b| cmp_by_dir_and_path(a, b));
                dir_items
            }
            Mode::Search => {
                struct FileTreeNodeWrapper {
                    item: FileTreeNode,
                    score: i64,
                }

                let search_string = &app_state.search_mode_state.search_string;

                if search_string.is_empty() {
                    dir_items.sort_by(|a, b| cmp_by_dir_and_path(a, b));
                    dir_items
                } else {
                    // get the scores
                    let mut dir_items: Vec<_> = dir_items
                        .into_iter()
                        .map(|el| FileTreeNodeWrapper {
                            score: el.compute_score(search_string),
                            item: el,
                        })
                        .filter(|el| el.score > 0)
                        .collect();

                    dir_items.sort_by(|a, b| {
                        if a.score != b.score {
                            return b.score.cmp(&a.score);
                        }
                        cmp_by_dir_and_path(&a.item, &b.item)
                    });

                    dir_items.into_iter().map(|el| el.item).collect()
                }
            }
        };

        if crossterm::event::poll(timeout)? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                // handle inputs
                inputs(key.code, dir_items, &config, &mut app_state, terminal);
            }
        } else {
            // Processes and draws the output
            terminal.draw(|f| draw(f, dir_items, &config, &mut app_state))?;
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

pub(self) fn inputs(
    k: KeyCode,
    dir_items: Vec<FileTreeNode>,
    config: &AppSettings,
    app_state: &mut TrackedModifiable<AppState>,
    terminal: &mut CustomTerminal,
) {
    // close the popup on a key press
    if let Some(_) = app_state.error_popup {
        app_state.get_mut().close_error_popup();
    }
    if let Some(_) = app_state.error_message_line {
        app_state.get_mut().error_message_line = None;
    }

    let is_search_mode = app_state.mode == Mode::Search;
    app_state
        .get_mut()
        .input_reader
        // if we are in the search mode, consider everything as verb text
        // (even digits, which would otherwise be considered as modifiers)
        // that is so that we remember it as normal text, not as commands
        .digest(k, is_search_mode);

    let modifier = app_state.input_reader.modifier_key_sequence.parse().ok();

    let mode_key_binding = match app_state.mode {
        Mode::Quitting => unreachable!(), // should have exited the program by now
        Mode::Normal => &config.normal_mode_key_bindings,
        Mode::Search => &config.search_mode_key_bindings,
    };
    let mode_actions = match app_state.mode {
        Mode::Quitting => unreachable!(), // should have exited the program by now
        Mode::Normal => &NORMAL_MODE_ACTION_MAP,
        Mode::Search => &SEARCH_MODE_ACTION_MAP,
    };

    let closure_option = app_state
        .input_reader
        .get_closure_by_key_bindings(mode_key_binding, mode_actions)
        .or_else(|| {
            // if we were not successful in finding a closure, look for a global key
            app_state
                .input_reader
                .get_closure_by_key_bindings(&config.global_key_bindings, &GLOBAL_ACTION_MAP)
        });

    if let Some(closure) = closure_option {
        let action_data = ActionData::new(config, terminal, app_state, modifier, &dir_items);
        let action_result = closure(action_data);

        // whether it was successful or not, clear the input state
        app_state.get_mut().input_reader.clear();
    } else {
        // look for the possible ways to continue the sequence. If there is one, do not do anything yet
        if !app_state
            .input_reader
            .check_incomplete_commands(vec![mode_key_binding, &config.global_key_bindings])
        {
            // if it is a search mode, enter the entire sequence
            if app_state.mode == Mode::Search {
                app_state.get_mut().copy_input_to_search_string();
            }
            // clear because that sequence is not valid
            app_state.get_mut().input_reader.clear();
        }
    }
}

pub(self) fn draw<B: Backend>(
    f: &mut tui::Frame<B>,
    dir_items: Vec<FileTreeNode>,
    config: &AppSettings,
    app_state: &mut TrackedModifiable<AppState>,
) {
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
        dir: Style::default()
            .bg(tui::style::Color::White)
            .fg(tui::style::Color::Rgb(50, 50, 200)),
    };
    let f_size = f.size();
    let bottom_text = app_state.error_message_line.clone().or_else(|| {
        if app_state.mode == Mode::Search {
            Some(format!("/ {}", &app_state.search_mode_state.search_string))
        } else {
            None
        }
    });

    // main division - main display vs the error line at the bottom
    let chunks = tui::layout::Layout::default()
        .direction(tui::layout::Direction::Vertical)
        .constraints(
            [
                Constraint::Min(1),
                Constraint::Length(if bottom_text.is_some() { 3 } else { 0 }),
            ]
            .as_ref(),
        )
        .split(f_size);

    // if the error line exists, write down the error text
    if let Some(bottom_text) = bottom_text {
        let block = Block::default().borders(Borders::ALL);
        f.render_widget(
            Paragraph::new(bottom_text.to_owned()).block(block),
            chunks[1],
        );
    }

    // main body
    {
        let chunks = tui::layout::Layout::default()
            .direction(tui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[0]);

        let dir_path_display_string = app_state
            .current_dir
            .get_path_buf()
            .as_os_str()
            .to_string_lossy()
            .into_owned();

        let left_chunk = chunks[0];
        let right_chunk = chunks[1];

        let block = Block::default().borders(Borders::ALL);
        let selected_file = match app_state.mode {
            Mode::Quitting => unreachable!(), // should have exited the program by now
            Mode::Normal => app_state.selected_file.as_ref(),
            Mode::Search => dir_items.get(0),
        };

        let file_text_preview = selected_file.and_then(|f| get_file_text_preview(&f));

        if let Some(text_preview) = file_text_preview {
            f.render_widget(Paragraph::new(text_preview).block(block), right_chunk);
        } else {
            f.render_widget(block, right_chunk);
        }

        let block = Block::default()
            .title(dir_path_display_string)
            .borders(Borders::ALL);

        let left_widget = match app_state.mode {
            Mode::Quitting => unreachable!(), // should have exited the program by now
            Mode::Search => {
                SearchModeState::get_left_ui(&dir_items, &cursor_styles, &default_styles)
            }
            Mode::Normal => NormalModeState::get_left_ui(
                app_state,
                &dir_items,
                config.min_distance_from_cursor_to_bottom,
                cursor_styles,
                default_styles,
                left_chunk.height as usize - 2, // -2 because one line from each side is used for the border
            ),
        };

        f.render_widget(left_widget.block(block), left_chunk);

        // error popup
        if let Some(error_popup) = &app_state.error_popup {
            let block = Block::default()
                .title(error_popup.title.clone())
                .borders(Borders::ALL);

            let paragraph = Paragraph::new(error_popup.desc.clone())
                .block(block)
                .alignment(tui::layout::Alignment::Center)
                .wrap(Wrap { trim: false });

            let area = centered_rect(60, 60, f_size);
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(paragraph, area);
        }
    }
}

// from https://github.com/fdehau/tui-rs/blob/master/examples/popup.rs
/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(layout[1])[1]
}
