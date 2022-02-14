mod actions;
mod compile_time_settings;
mod directory_tree;
mod helper_types;
mod modes;

use std::env;
use std::{
    io,
    time::{Duration, Instant},
};

use actions::{GLOBAL_ACTION_MAP, NORMAL_MODE_ACTION_MAP, SEARCH_MODE_ACTION_MAP};
use crossterm::{event::EnableMouseCapture, terminal::EnterAlternateScreen};
use helper_types::{AppSettings, AppState, StyleSet};
use modes::{Mode, ModeController, ModesManager, RecordedModifiable};
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::Style;

use tui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::directory_tree::FileTreeNode;
use crate::helper_types::ErrorPopup;

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
        dir: Style::default()
            .bg(tui::style::Color::White)
            .fg(tui::style::Color::Rgb(50, 50, 200)),
    };

    let config = AppSettings::load_config(vec![
        "../example_config.toml",
        "/usr/share/fphile/global_config.toml",
    ])?;

    let modes_manager = ModesManager::new(default_styles, cursor_styles);
    let app_state = AppState::new(FileTreeNode::new(
        env::current_dir()
            .expect("Could not get the current directory")
            .to_path_buf(),
    ));
    // create app and run it
    let res = run_loop(
        &mut terminal,
        app_state,
        modes_manager,
        Duration::from_millis(config.render_timeout.unwrap_or(250)),
        config,
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
    mut app_state: AppState,
    mut modes_manager: ModesManager,
    tick_rate: Duration,
    config: AppSettings,
) -> io::Result<()> {
    let mut last_tick = Instant::now();

    // make sure to draw the first frame on startup
    modes_manager.mark_as_modified();

    loop {
        if modes_manager.get_current_mode() == &Mode::Quitting {
            return Ok(());
        }
        // if an urgent update, fore it to update ASAP by reducing wait time to 0
        let timeout = if modes_manager.has_been_modified() || app_state.has_been_modified() {
            Duration::from_secs(0)
        } else {
            tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0))
        };
        modes_manager.reset_modification_status();

        let poll_result = crossterm::event::poll(timeout)?;
        let dir_items = app_state
            .get_current_dir()
            .clone()
            .get_files()
            .unwrap_or_else(|err| {
                if let io::ErrorKind::PermissionDenied = err.kind() {
                    app_state.set_error_popup(Some(ErrorPopup::new(
                        String::from("Permission denied"),
                        String::from(
                            "You do not have the permissions to access this folder/directory",
                        ),
                    )));
                // unknown error
                } else {
                    app_state.set_error_popup(Some(ErrorPopup::new(
                        String::from("Unknown error"),
                        String::from("An error occured"),
                    )));
                }
                vec![]
            });
        // collect the data once and then either use it for inputs or for outputs, but not both because of the potential to modify it
        // sort them

        let dir_items = modes_manager.transform_dir_items(dir_items);

        if poll_result {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                // close the popup on a key press
                if let Some(_) = app_state.get_error_popup() {
                    app_state.set_error_popup(None);
                }

                app_state.get_input_reader().record_key_in_state(
                    key.code,
                    modes_manager.get_current_mode() != &Mode::Search,
                );
                // Look for key bindings in the current mode and execute them
                let have_executed_an_action = match modes_manager.get_current_mode() {
                    Mode::Quitting => false,
                    Mode::Normal => {
                        if let Some(closure) = app_state.get_input_reader().get_action_closure(
                            &config.normal_mode_key_bindings,
                            &NORMAL_MODE_ACTION_MAP,
                        ) {
                            let modifier_key_sequence: Option<usize> = app_state
                                .get_input_reader()
                                .get_modifier_key_sequence()
                                .parse()
                                .ok();
                            closure(
                                &mut app_state,
                                &mut modes_manager,
                                modifier_key_sequence,
                                &dir_items,
                            )?;
                            true
                        } else {
                            false
                        }
                    }
                    Mode::Search => {
                        if let Some(closure) = app_state.get_input_reader().get_action_closure(
                            &config.search_mode_key_bindings,
                            &SEARCH_MODE_ACTION_MAP,
                        ) {
                            closure(&mut app_state, &mut modes_manager, &dir_items)?;
                            true
                        } else {
                            false
                        }
                    }
                };
                // only look for the other keybinds if the first one has not been done
                let have_executed_an_action = have_executed_an_action
                    || if let Some(closure) = app_state
                        .get_input_reader()
                        .get_action_closure(&config.global_key_bindings, &GLOBAL_ACTION_MAP)
                    {
                        let modifier_key_sequence: Option<usize> = app_state
                            .get_input_reader()
                            .get_modifier_key_sequence()
                            .parse()
                            .ok();
                        closure(&mut app_state, &mut modes_manager, modifier_key_sequence)?;
                        // clear the sequence if it was successful or not
                        true
                    } else {
                        false
                    };

                if have_executed_an_action {
                    app_state.get_input_reader().clear();
                } else {
                    // if there are no possible ways to continue the sequence
                    if !app_state
                        .get_input_reader()
                        .check_for_possible_extensions(vec![
                            &config.global_key_bindings,
                            if let Mode::Search = modes_manager.get_current_mode() {
                                &config.search_mode_key_bindings
                            } else {
                                &config.normal_mode_key_bindings
                            },
                        ])
                    {
                        // if it is a search mode, try entering the exact character
                        if let Mode::Search = modes_manager.get_current_mode() {
                            modes_manager
                                .search_mode_controller
                                .move_input(&mut app_state);
                        } else {
                            app_state.get_input_reader().clear();
                        }
                    }
                }
            }
        } else {
            // Processes and draws the output
            terminal.draw(|f| {
                let f_size = f.size();
                let bottom_text = modes_manager.get_bottom_text();

                let chunks = tui::layout::Layout::default()
                    .direction(tui::layout::Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Min(1),
                            Constraint::Length(if let Some(_) = bottom_text { 3 } else { 0 }),
                        ]
                        .as_ref(),
                    )
                    .split(f_size);

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
                        .get_current_dir()
                        .get_path_buf()
                        .as_os_str()
                        .to_string_lossy()
                        .into_owned();

                    let block = tui::widgets::Block::default().borders(Borders::ALL);
                    let right_widget = modes_manager.get_right_ui(block, chunks[1], &dir_items);
                    right_widget.render(f, chunks[1]);

                    let block = tui::widgets::Block::default()
                        .title(dir_path_string)
                        .borders(Borders::ALL);

                    let left_widget = modes_manager.get_left_ui(block, chunks[0], &dir_items);

                    left_widget.render(f, chunks[0]);

                    // error popup
                    if let Some(error_popup) = &app_state.get_error_popup() {
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
            })?;
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
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
