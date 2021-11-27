use crossterm::event::{KeyCode, KeyEvent};
use std::io;

use crate::{directory_tree::FileTreeNode, AppMode, AppState};

#[derive(PartialEq)]
enum KeySequenceState {
    COMPLETE,
    INCOMPLETE,
    INVALID,
}

pub(crate) fn handle_inputs(key: KeyEvent, app_state: &mut AppState) -> io::Result<()> {
    // we do not need urgent updates because it all updates on input events automatically
    // if let crossterm::event::KeyCode::Char(character) = key.code {
    //     if first_key_sequence_characters.contains(&character) {
    //         app_state.last_key_sequence_char = Some(character);
    //     }
    // }

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
                clear_key_sequences(app_state);
            }
            return Ok(());
        }

        // verbs
        let modifier: usize = app_state.modifier_key_sequence.parse().ok().unwrap_or(1);
        let key_sequence_state = match key.code {
            crossterm::event::KeyCode::Char('q') => {
                app_state.app_mode = AppMode::QUITTING;
                KeySequenceState::COMPLETE
            }
            // down
            crossterm::event::KeyCode::Char('j') => {
                // TODO: make sure it works in empty dirs
                change_file_cursor_index(app_state, |i, items| {
                    (i + modifier).rem_euclid(items.len())
                })?;
                KeySequenceState::COMPLETE
            }
            // up
            crossterm::event::KeyCode::Char('k') => {
                // Add the length to make sure that there is no overflow
                change_file_cursor_index(app_state, |i, items| {
                    (items.len() + i - modifier).rem_euclid(items.len())
                })?;
                KeySequenceState::COMPLETE
            }
            crossterm::event::KeyCode::Char('g') => {
                if app_state.verb_key_sequence.is_empty() {
                    KeySequenceState::INCOMPLETE
                } else {
                    if app_state.verb_key_sequence == "g" {
                        change_file_cursor_index(app_state, |_, _| 0)?;
                        KeySequenceState::COMPLETE
                    } else {
                        KeySequenceState::INVALID
                    }
                }
            }
            crossterm::event::KeyCode::Char('G') => {
                // if there is a specified line, go to it
                if !app_state.modifier_key_sequence.is_empty() {
                    change_file_cursor_index(app_state, |i, items| {
                        if modifier > items.len() {
                            i
                        } else {
                            modifier - 1 // to convert it into an index
                        }
                    })?;
                    // TODO: return KeySequenceState::INVALID when needed
                    // TODO: maybe show an error message, e.g. INVALID(String) ?
                    KeySequenceState::COMPLETE
                } else {
                    change_file_cursor_index(app_state, |_, items| items.len() - 1)?;
                    KeySequenceState::COMPLETE
                }
            }
            // It does not make sense, so skip it
            _ => KeySequenceState::INCOMPLETE,
        };

        if key_sequence_state == KeySequenceState::INVALID
            || key_sequence_state == KeySequenceState::COMPLETE
        {
            clear_key_sequences(app_state);
        } else {
            app_state.verb_key_sequence.push(character);
        }
    }
    Ok(())
}

fn clear_key_sequences(app_state: &mut AppState) {
    app_state.modifier_key_sequence.clear();
    app_state.verb_key_sequence.clear();
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
