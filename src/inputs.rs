use crossterm::event::KeyEvent;
use std::io;

use crate::{directory_tree::FileTreeNode, AppMode, AppState};

const first_key_sequence_characters: [char; 1] = ['a'];

pub(crate) fn handle_inputs(key: KeyEvent, app_state: &mut AppState) -> io::Result<()> {
    // we do not need urgent updates because it all updates on input events automatically
    // if let crossterm::event::KeyCode::Char(character) = key.code {
    //     if first_key_sequence_characters.contains(&character) {
    //         app_state.last_key_sequence_char = Some(character);
    //     }
    // }

    // verbs
    match key.code {
        crossterm::event::KeyCode::Char('q') => app_state.app_mode = AppMode::QUITTING,
        // down
        crossterm::event::KeyCode::Char('j') => {
            // TODO: make sure it works in empty dirs
            change_file_cursor_index(app_state, |i, items| (i + 1).rem_euclid(items.len()))?;
        }
        // up
        crossterm::event::KeyCode::Char('k') => {
            // Add the length to make sure that there is no overflow
            change_file_cursor_index(app_state, |i, items| {
                (items.len() + i - 1).rem_euclid(items.len())
            })?;
        }
        _ => {}
    }

    Ok(())
}

fn change_file_cursor_index<F: Fn(usize, &Vec<FileTreeNode>) -> usize>(
    app_state: &mut AppState,
    modifier: F,
) -> io::Result<()> {
    let dir_items = app_state
        .current_dir
        .list_files(&app_state.file_tree_node_sorter)?;
    // TODO: work with empty dirs
    let file_cursor_index = app_state
        .file_cursor
        .get_file_cursor_index(&dir_items)
        .unwrap();

    let new_file_cursor_index = modifier(file_cursor_index, &dir_items);

    // Update
    if new_file_cursor_index != file_cursor_index && new_file_cursor_index < dir_items.len() {
        // TODO: do something about this unwrap
        app_state.file_cursor.selected_file = Some(
            dir_items
                .get(new_file_cursor_index)
                .unwrap()
                .get_path()
                .as_os_str()
                .to_owned(),
        );
    }

    Ok(())
}
