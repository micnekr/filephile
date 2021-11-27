use std::{borrow::Borrow, io, ops::RangeBounds};

use tui::{
    layout::{Constraint, Direction::Vertical},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::{directory_tree::FileSelection, AppState};

pub(crate) fn ui<B: tui::backend::Backend>(
    f: &mut tui::Frame<B>,
    app_state: &mut AppState,
) -> io::Result<()> {
    let dir_path_string = app_state
        .current_dir
        .get_path()
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

        let mut dir_items = app_state
            .current_dir
            .list_files(&app_state.file_tree_node_sorter)?;

        let file_cursor_index = app_state
            .file_cursor
            .get_file_cursor_index(&dir_items)
            .unwrap_or_else(|| {
                // if the cursor can not be placed:
                app_state.is_urgent_update = true;
                // TODO: make sure this does not break with empty dirs
                app_state.file_cursor.selected_file =
                    Some(dir_items.first().unwrap().get_path().as_os_str().to_owned());
                0
            });
        let dir_items: Vec<_> = dir_items
            .iter()
            .enumerate()
            .map(|el| {
                let file_name = match el.1.get_simple_name() {
                    Ok(simple_name) => simple_name.to_string_lossy().into_owned(),
                    Err(err) => {
                        app_state.set_err(err);
                        String::from("<Could not get file name>")
                    }
                };
                let mut out = ListItem::new(file_name.clone());
                if el.0 == file_cursor_index {
                    out = app_state.file_cursor.update_styles(out);
                }
                out
            })
            .collect();

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
