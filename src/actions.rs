use lazy_static::lazy_static;
use std::{collections::BTreeMap, ffi::OsString, fs::File, io, path::PathBuf};

use crate::{
    directory_tree::FileTreeNode,
    modes::{Mode, ModesManager},
    AppState,
};

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
                &mut ModesManager,
                Option<usize>,
                &Vec<FileTreeNode>,
            ) -> io::Result<ActionResult>
            + Sync
            + 'static,
    >,
>;

type SearchModeActionNameMap = NameMap<
    Box<
        dyn Fn(&mut AppState, &mut ModesManager, &Vec<FileTreeNode>) -> io::Result<ActionResult>
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
            String::from("normal_mode"),
            Box::new(|_, mode_manager, _| {
                mode_manager.search_mode_controller.clear();
                mode_manager.set_current_mode(Mode::Normal);
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            String::from("search_mode"),
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
            String::from("noop"),
            Box::new(|_, _, _, _| {
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            String::from("down"),
            Box::new(|_, mode_manager, modifier, dir_items| {
                mode_manager.normal_mode_controller.change_file_cursor_index(dir_items, |i, items| {
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
                mode_manager.normal_mode_controller.change_file_cursor_index(dir_items, |i, items| {
                    if let Some(i) = i {
                    Some((items.len() + i - modifier.unwrap_or(1)).rem_euclid(items.len()))
                    }else {None}
                })?;
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            String::from("left"),
            Box::new(|app_state, _, _, _|{
                let current_path = app_state.get_current_dir().get_path_buf();
                let next_path = current_path.parent().unwrap_or(&current_path);
                app_state.set_current_dir(FileTreeNode::new(next_path.to_path_buf())?);
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            String::from("right"),
            Box::new(|app_state, mode_manager, _, _| {
                let selected_file_tree_node = mode_manager.normal_mode_controller
                    .get_file_cursor()
                    .get_selected_file()
                    .as_ref()
                    .map(|el| el.to_owned());
                if let Some(selected_file_tree_node) = selected_file_tree_node {
                    // if selected_file_tree_node.
                    if selected_file_tree_node.is_dir() {
                        app_state.set_current_dir(selected_file_tree_node);
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
                // if there is a specified line, go to it
                if let Some(modifier) = modifier {
                    mode_manager.normal_mode_controller.change_file_cursor_index(dir_items, |i, items| {
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
                    Ok(ActionResult::VALID)
                } else {
                    mode_manager.normal_mode_controller.change_file_cursor_index(dir_items, |_, items| {
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
                mode_manager.normal_mode_controller.change_file_cursor_index(dir_items, |_, _| Some(0))?;
                Ok(ActionResult::VALID)
            }),
        );
        m
    };

    pub(crate) static ref SEARCH_MODE_ACTION_MAP: SearchModeActionNameMap = {
        let mut m: SearchModeActionNameMap = BTreeMap::new();
        m.insert(
            String::from("noop"),
            Box::new(|app_state, mode_controller, _| {
                mode_controller.search_mode_controller.move_input(app_state);
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            String::from("delete_last_char"),
            Box::new(|_, mode_controller, _| {
                mode_controller.search_mode_controller.delete_last_char();
                Ok(ActionResult::VALID)
            }),
        );
        m.insert(
            String::from("select"),
            Box::new(|_, mode_controller, dir_items |{
                if let Some(first_item_name) = dir_items.get(0) {
                mode_controller.normal_mode_controller.get_file_cursor_mut().set_selected_file(Some(FileTreeNode::new(first_item_name.get_path_buf().to_path_buf())?));
                }
                mode_controller.search_mode_controller.clear();
                mode_controller.set_current_mode(Mode::Normal);
                Ok(ActionResult::VALID)
            }),
        );

        m
    };
}
