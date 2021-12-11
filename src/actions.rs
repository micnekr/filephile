use lazy_static::lazy_static;
use std::{collections::BTreeMap, io, path::PathBuf};

use crate::{
    directory_tree::FileTreeNode,
    modes::{Mode, ModesManager, NormalModeController},
    AppState,
};

// // TODO: use phf instead
#[derive(PartialEq)]
pub(crate) enum ActionResult {
    VALID,
    INVALID,
}

pub(crate) type NameMap<Action> = BTreeMap<String, Action>;
type GlobalActionNameMap = NameMap<
    Box<
        dyn Fn(&mut AppState, &mut ModesManager, Option<usize>) -> io::Result<ActionResult>
            + Sync
            + 'static,
    >,
>;
type NormalModeActionNameMap = NameMap<
    Box<
        dyn Fn(
                &mut AppState,
                &mut NormalModeController,
                Option<usize>,
                &Vec<FileTreeNode>,
            ) -> io::Result<ActionResult>
            + Sync
            + 'static,
    >,
>;

lazy_static! {
    pub(crate) static ref GLOBAL_ACTION_MAP: GlobalActionNameMap = {
        let mut m: GlobalActionNameMap = BTreeMap::new();
        m.insert(
            String::from("quit"),
            Box::new(|_, mode_manager, _| {
                mode_manager.set_current_mode(Mode::Quitting);
                // app_state.is_urgent_update = true;
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            String::from("search"),
            Box::new(|_, mode_manager, _| {
                mode_manager.set_current_mode(Mode::Search);
                Ok(ActionResult::VALID)
            }),
        );
        m
    };
    pub(crate) static ref NORMAL_MODE_ACTION_MAP: NormalModeActionNameMap = {
        let mut m: NormalModeActionNameMap = BTreeMap::new();
        m.insert(
            String::from("down"),
            Box::new(|_, mode_manager, modifier, dir_items| {
                mode_manager.change_file_cursor_index(dir_items, |i, items| {
                    if let Some(i) = i{
                    Some((i + modifier.unwrap_or(1)).rem_euclid(items.len()))
                    }else {None}
                })?;
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            String::from("up"),
            Box::new(|_, mode_manager, modifier, dir_items|{
                // Add the length to make sure that there is no overflow
                mode_manager.change_file_cursor_index(dir_items, |i, items| {
                    if let Some(i) = i {
                    Some((items.len() + i - modifier.unwrap_or(1)).rem_euclid(items.len()))
                    }else {None}
                })?;
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            String::from("left"),
            Box::new(|app_state, mode_manager, modifier, dir_items|{
                let current_path = app_state.current_dir.get_path();
                let next_path = current_path.parent().unwrap_or(&current_path);
                app_state.current_dir = FileTreeNode::new(next_path.to_path_buf())?;
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            String::from("right"),
            Box::new(|app_state, mode_manager, modifier, dir_items| {
                let selected_file_tree_node = mode_manager
                    .get_file_cursor()
                    .get_selected_file()
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
            String::from("go_to_or_go_to_bottom"),
            Box::new(|_, mode_manager, modifier, dir_items| {
                // TODO: decouple key sequences from the app state?
                // if there is a specified line, go to it
                if let Some(modifier) = modifier {
                    mode_manager.change_file_cursor_index(dir_items, |i, items| {
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
                    // TODO: maybe show an error message, e.g. INVALID(String) ?
                    Ok(ActionResult::VALID)
                } else {
                    mode_manager.change_file_cursor_index(dir_items, |_, items| {
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
            String::from("go_to_top"),
            Box::new(|_, mode_manager, _, dir_items| {
                mode_manager.change_file_cursor_index(dir_items, |_, _| Some(0))?;
                Ok(ActionResult::VALID)
            }),
        );
        m
    };
}

// type Action =
//     Box<dyn Fn(&mut AppState, Option<usize>) -> io::Result<ActionResult> + Sync + 'static>;
// lazy_static! {
//     pub(crate) static ref ACTION_MAP: ActionNameMap = {
//         let mut m: ActionNameMap = BTreeMap::new();
//         m
//     };
// }
