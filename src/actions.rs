use once_cell::sync::Lazy;
use std::{collections::BTreeMap, fs::canonicalize};

use crate::{
    directory_tree::FileTreeNode,
    enter_captured_mode, exit_captured_mode,
    helper_types::{AppSettings, NormalModeState, SearchModeState, TrackedModifiable},
    modes::Mode,
    AppState, CustomTerminal,
};

#[derive(PartialEq)]
pub enum ActionResult {
    VALID,
    INVALID(String),
}

pub struct ActionData<'a> {
    config: &'a AppSettings,
    terminal: &'a mut CustomTerminal,
    app_state: &'a mut TrackedModifiable<AppState>,
    modifier: Option<usize>,
    dir_items: &'a Vec<FileTreeNode>,
}

impl<'a> ActionData<'a> {
    pub fn new(
        config: &'a AppSettings,
        terminal: &'a mut CustomTerminal,
        app_state: &'a mut TrackedModifiable<AppState>,
        modifier: Option<usize>,
        dir_items: &'a Vec<FileTreeNode>,
    ) -> Self {
        ActionData {
            config,
            terminal,
            app_state,
            modifier,
            dir_items,
        }
    }
}

pub type ActionNameMap = BTreeMap<String, ActionClosure>;

pub type ActionClosure = Box<dyn Fn(ActionData) -> ActionResult + Sync + Send + 'static>;

pub(crate) static GLOBAL_ACTION_MAP: Lazy<ActionNameMap> = Lazy::new(|| {
    let mut m: ActionNameMap = BTreeMap::new();
    m.insert(
        String::from("quit"),
        Box::new(|v| {
            v.app_state.get_mut().mode = Mode::Quitting;
            ActionResult::VALID
        }),
    );
    m.insert(
        String::from("normal_mode"),
        Box::new(|v| {
            // reset the normal mode
            v.app_state.get_mut().normal_mode_state = NormalModeState::default();

            v.app_state.get_mut().mode = Mode::Normal;
            ActionResult::VALID
        }),
    );
    m.insert(
        String::from("search_mode"),
        Box::new(|v| {
            // reset the search mode
            v.app_state.get_mut().search_mode_state = SearchModeState::default();

            v.app_state.get_mut().mode = Mode::Search;
            ActionResult::VALID
        }),
    );
    m
});
pub(crate) static NORMAL_MODE_ACTION_MAP: Lazy<ActionNameMap> = Lazy::new(|| {
    let mut m: ActionNameMap = BTreeMap::new();
    m.insert(String::from("noop"), Box::new(|_| ActionResult::VALID));
    m.insert(
        String::from("down"),
        Box::new(|v| {
            v.app_state
                .get_mut()
                .set_file_cursor_highlight_index(v.dir_items, |i, _| i + v.modifier.unwrap_or(1));

            ActionResult::VALID
        }),
    );
    m.insert(
        String::from("up"),
        Box::new(|v| {
            v.app_state.get_mut().set_file_cursor_highlight_index(
                v.dir_items,
                |i, dir_items_num| {
                    // make sure that we do not go into the negatives because of overflow
                    // rem_euclid makes sure that we do not have a number less than or equal to dir_items_num - 1
                    dir_items_num + i - v.modifier.unwrap_or(1).rem_euclid(dir_items_num)
                },
            );

            ActionResult::VALID
        }),
    );
    m.insert(
        String::from("left"),
        Box::new(|v| {
            let current_path = v.app_state.current_dir.get_path_buf();
            let next_path = current_path.parent().unwrap_or(&current_path);
            let new_dir = next_path.to_path_buf();
            v.app_state.get_mut().current_dir = FileTreeNode::new(new_dir);
            ActionResult::VALID
        }),
    );
    m.insert(
        String::from("right"),
        Box::new(|v| {
            let selected_file_tree_node = &v.app_state.selected_file;
            if let Some(selected_file_tree_node) = selected_file_tree_node {
                if selected_file_tree_node.is_dir() {
                    // open the directory
                    v.app_state.get_mut().current_dir = selected_file_tree_node.clone();
                    ActionResult::VALID
                } else {
                    if let Some(file_editor_options) = &v.config.default_file_editor_command{

                    // open the file
                    // move to a different screen
                    exit_captured_mode(v.terminal).expect("Could not leave terminal capture");
                    // NOTE: current_dir()'s behaviour is up to the implementation if the path is relative,
                    // So we need to make it canonical
                    let relative_path_current_dir = v.app_state.current_dir.get_path_buf();
                    let panic_message = format!(
                        r#"Encountered an error while tried to convert "{}" to an absolute path"#,
                        relative_path_current_dir.as_os_str().to_string_lossy()
                    );
                    let absolute_path_current_dir =
                        canonicalize(relative_path_current_dir).expect(&panic_message);
                    let mut file_editor_options = file_editor_options.iter();
                    let mut command = std::process::Command::new(
                        file_editor_options
                            .next()
                            .expect("Found empty arguments for the default file editor"),
                    );
                    command.current_dir(absolute_path_current_dir);

                    let file_name = selected_file_tree_node.get_simple_name();
                    for file_editor_option in file_editor_options {
                        let file_editor_option = file_editor_option.replace("<FILE>", &file_name);
                        command.arg(file_editor_option);
                    }
                    command
                        .spawn()
                        .expect("Error: Failed to run an editor")
                        .wait()
                        .expect("Error: Editor returned a non-zero status");

                    enter_captured_mode(v.terminal)
                        .expect("Could not re-enter the terminal capture");
                    ActionResult::VALID
                    } else {
                        ActionResult::INVALID(String::from("Can not open the file because the config file does not contain a command to open files"))
                    }
                }
            } else {
                ActionResult::INVALID(String::from("No file selected"))
            }
        }),
    );
    m.insert(
        String::from("go_to_or_go_to_bottom"),
        Box::new(|v| {
            // if there is a specified line, go to it
            if let Some(modifier) = v.modifier {
                v.app_state.get_mut().set_file_cursor_highlight_index(
                    v.dir_items,
                    |_, num_items| {
                        (modifier - 1).clamp(0, num_items - 1) // to convert it into an index
                    },
                );
                ActionResult::VALID
            } else {
                v.app_state
                    .get_mut()
                    .set_file_cursor_highlight_index(v.dir_items, |_, num_items| num_items - 1);
                ActionResult::VALID
            }
        }),
    );
    m.insert(
        String::from("go_to_top"),
        Box::new(|v| {
            v.app_state
                .get_mut()
                .set_file_cursor_highlight_index(v.dir_items, |_, _| 0);
            ActionResult::VALID
        }),
    );
    m
});

pub(crate) static SEARCH_MODE_ACTION_MAP: Lazy<ActionNameMap> = Lazy::new(|| {
    let mut m: ActionNameMap = BTreeMap::new();
    m.insert(
        String::from("noop"),
        Box::new(|v| {
            // push the new typed characters
            v.app_state.get_mut().copy_input_to_search_string();
            v.app_state.get_mut().input_reader.clear();
            ActionResult::VALID
        }),
    );
    m.insert(
        String::from("delete_last_char"),
        Box::new(|v| {
            v.app_state.get_mut().search_mode_state.search_string.pop();

            ActionResult::VALID
        }),
    );
    m.insert(
        String::from("select"),
        Box::new(|v| {
            if let Some(first_item) = v.dir_items.get(0) {
                v.app_state.get_mut().selected_file = Some(first_item.to_owned());
            }

            v.app_state.get_mut().normal_mode_state = NormalModeState::default();
            v.app_state.get_mut().search_mode_state = SearchModeState::default();

            v.app_state.get_mut().mode = Mode::Normal;

            ActionResult::VALID
        }),
    );

    m
});
