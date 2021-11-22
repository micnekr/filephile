use crossterm::event::KeyEvent;
use std::io;

use crate::{AppMode, AppState};

pub(crate) fn handle_inputs(key: KeyEvent, app_state: &mut AppState) -> io::Result<()> {
    match key.code {
        crossterm::event::KeyCode::Char('q') => app_state.app_mode = AppMode::QUITTING,
        _ => {}
    }
    Ok(())
}
