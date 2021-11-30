use crate::{directory_tree::FileTreeNode, inputs::change_file_cursor_index, AppMode, AppState};
use lazy_static::lazy_static;
use std::{collections::BTreeMap, io, path::PathBuf};

// TODO: use phf instead
pub(crate) enum ActionResult {
    VALID,
    INVALID,
}

type Action = Box<dyn Fn(&mut AppState, usize) -> io::Result<ActionResult> + Sync + 'static>;
type ActionNameMap = BTreeMap<&'static str, Action>;
lazy_static! {
    pub(crate) static ref ACTION_MAP: ActionNameMap = {
        let mut m: ActionNameMap = BTreeMap::new();
        m.insert(
            "quit",
            Box::new(|app_state, _| {
                app_state.app_mode = AppMode::QUITTING;
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            "down",
            Box::new(|app_state, modifier| {
                // TODO: make sure it works in empty dirs
                change_file_cursor_index(app_state, |i, items| {
                    if let Some(i) = i{
                    Some((i + modifier).rem_euclid(items.len()))
                    }else {None}
                })?;
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            "up",
            Box::new(|app_state, modifier|{
                // Add the length to make sure that there is no overflow
                change_file_cursor_index(app_state, |i, items| {
                    if let Some(i) = i{
                    Some((items.len() + i - modifier).rem_euclid(items.len()))
                    }else {None}
                })?;
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            "left",
            Box::new(|app_state, _|{
                let current_path = app_state.current_dir.get_path();
                let next_path = current_path.parent().unwrap_or(&current_path);
                app_state.current_dir = FileTreeNode::new(next_path.to_path_buf())?;
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            "right",
            Box::new(|app_state, _|{
                let selected_file_tree_node = app_state
                    .file_cursor
                    .selected_file
                    .as_ref()
                    .map(|el| FileTreeNode::new(PathBuf::from(el)));
                if let Some(Ok(selected_file_tree_node)) = selected_file_tree_node {
                    // if selected_file_tree_node.
                    if selected_file_tree_node.is_dir() {
                        app_state.current_dir = selected_file_tree_node;
                        Ok(ActionResult::VALID)
                    } else {
                        Ok(ActionResult::INVALID)
                    }
                } else {
                        Ok(ActionResult::INVALID)
                }
            }),
        );
        m.insert(
            "go_to_or_go_to_bottom",
            Box::new(|app_state, modifier| {
                // if there is a specified line, go to it
                // TODO: decouple key sequences from the app state?
                if !app_state.modifier_key_sequence.is_empty() {
                    change_file_cursor_index(app_state, |i, items| {
                        if let Some(i) = i {
                        Some(if modifier > items.len() {
                            i
                        } else {
                            modifier - 1 // to convert it into an index
                        })
                        } else {
                            None
                        }
                    })?;
                    // TODO: return KeySequenceState::INVALID when needed
                    // TODO: maybe show an error message, e.g. INVALID(String) ?
                    Ok(ActionResult::VALID)
                } else {
                    change_file_cursor_index(app_state, |_, items|
                                             {
                                                 match items.len() {
                                                     0 => None,

                                                 _ => Some(items.len() - 1)
                                                 }
                                             })?;
                    Ok(ActionResult::VALID)
                }
            }),
        );
        m.insert(
            "go_to_top",
            Box::new(|app_state, _| {
                change_file_cursor_index(app_state, |_, _| Some(0))?;
                Ok(ActionResult::VALID)
            }),
        );
        m
    };
}
