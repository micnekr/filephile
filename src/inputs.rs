use crossterm::event::{KeyCode, KeyEvent};
use std::io;

use crate::{
    actions::{ActionResult, ACTION_MAP},
    directory_tree::FileTreeNode,
    AppState,
};

pub(crate) fn handle_inputs(key: KeyEvent, app_state: &mut AppState) -> io::Result<()> {
    // we do not need urgent updates because it all updates on input events automatically
    // clear all the errors?
    app_state.error_message = String::from("");

    if key.code == KeyCode::Esc {
        clear_key_sequences(app_state);
        return Ok(());
    }

    if let crossterm::event::KeyCode::Char(character) = key.code {
        // modifiers
        if character.is_digit(10) {
            app_state.modifier_key_sequence.push(character);

            // we can not add a movement after a verb, so fail in that case
            if !app_state.verb_key_sequence.is_empty() {
                // TODO: maybe notify the user that this was incorrect
                clear_key_sequences(app_state);
            }
            return Ok(());
        }
        app_state.verb_key_sequence.push(character);

        // verbs
        let modifier: usize = app_state.modifier_key_sequence.parse().ok().unwrap_or(1);
        // check if there is an existing one
        if let Some(command_name) = app_state
            .key_sequence_to_action_mapping
            .get(app_state.verb_key_sequence.as_str())
        {
            // TODO: show an error message if that is not found
            if let Some(command) = ACTION_MAP.get(command_name) {
                // TODO: attach a message to ActionResult::INVALID and display it to hint at what is wrong
                let command_result = command(app_state, modifier)?;
            }

            clear_key_sequences(app_state);
        } else {
            // no commands were found, see if it is possible that a command will be matched in the future:
            let was_found = app_state
                .key_sequence_to_action_mapping
                .iter()
                .any(|(k, _)| k.starts_with(app_state.verb_key_sequence.as_str()));
            // TODO: if not found, tell us
            if !was_found {
                clear_key_sequences(app_state);
            }
        }
    }
    Ok(())
}

pub(crate) fn clear_key_sequences(app_state: &mut AppState) {
    app_state.modifier_key_sequence.clear();
    app_state.verb_key_sequence.clear();
}

pub(crate) fn change_file_cursor_index<
    F: Fn(Option<usize>, &Vec<FileTreeNode>) -> Option<usize>,
>(
    app_state: &mut AppState,
    update_function: F,
) -> io::Result<()> {
    let dir_items = app_state
        .current_dir
        .list_files(&app_state.file_tree_node_sorter)?;
    let file_cursor_index = app_state.file_cursor.get_file_cursor_index(&dir_items);

    let new_file_cursor_index = update_function(file_cursor_index, &dir_items);

    // Update
    // TODO: what do we do if the index is past its max value?: new_file_cursor_index < dir_items.len()
    if new_file_cursor_index != file_cursor_index {
        // if no index, return None
        app_state.file_cursor.selected_file = new_file_cursor_index.map_or(None, |index| {
            dir_items
                .get(index)
                // if not found, return None
                .map_or(None, |file| Some(file.get_path().as_os_str().to_owned()))
        });
    }

    Ok(())
}
