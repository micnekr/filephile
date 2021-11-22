use std::{borrow::Borrow, io};

use tui::{
    layout::{Constraint, Direction::Vertical},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::AppState;

pub(crate) fn ui<B: tui::backend::Backend>(
    f: &mut tui::Frame<B>,
    app_state: &mut AppState,
) -> io::Result<()> {
    let dir_path_string = app_state
        .current_dir
        .get_dir_path()
        .as_os_str()
        .to_string_lossy()
        .into_owned();

    let error_message_present = !app_state.error_message.is_empty();

    let chunks = tui::layout::Layout::default()
        .direction(Vertical)
        // .margin(0)
        .constraints(
            [
                Constraint::Min(1),
                Constraint::Length(if error_message_present { 3 } else { 0 }),
            ]
            .as_ref(),
        )
        .split(f.size());
    // error message
    if error_message_present {
        let block = Block::default().borders(Borders::ALL);
        let paragraph = Paragraph::new(app_state.error_message.borrow())
            .block(block)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, chunks[1]);
    }
    {
        let chunks = tui::layout::Layout::default()
            .direction(tui::layout::Direction::Horizontal)
            // .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[0]);

        let block = Block::default()
            .title(dir_path_string)
            .borders(Borders::ALL);

        let dir_items = app_state
            .current_dir
            .list_files()?
            .iter()
            .map(|e| {
                let e = e.to_string_lossy().into_owned();
                ListItem::new(e)
            })
            .collect::<Vec<_>>();
        let list = List::new(dir_items).block(block);

        f.render_widget(list, chunks[0]);

        let block = Block::default()
            .title(app_state.opened_file.clone().unwrap_or("".to_owned()))
            .borders(Borders::ALL);
        f.render_widget(block, chunks[1]);
    }
    Ok(())
}

// fn unwrap_or_print<T>(error: io::Result<T>, app_state: &mut AppState) -> Option<T> {
//     match error {
//         Err(err) => {
//             app_state.error_message = format!("{}", err);
//             None
//         }
//         Ok(out) => Some(out),
//     }
// }
