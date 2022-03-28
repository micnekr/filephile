use crate::{
    actions::{ActionClosure, ActionMapper},
    directory_tree::{get_file_cursor_index, FileTreeNode},
    modes::{Mode, SimpleMode},
};
use crossterm::event::KeyCode;
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Error, ErrorKind},
    ops::Deref,
    path::Path,
};

use serde::{Deserialize, Serialize};
use tui::style::Style;

type StringMap = BTreeMap<String, String>;

pub struct TrackedModifiable<T> {
    pub(self) val: T,
    pub(self) is_modified: bool,
}

impl<T> TrackedModifiable<T> {
    pub fn new(val: T) -> Self {
        TrackedModifiable {
            val,
            is_modified: false,
        }
    }

    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    pub fn reset_modified_flag(&mut self) {
        self.is_modified = false;
    }
}

impl<T> TrackedModifiable<T> {
    pub fn get_mut(&mut self) -> &mut T {
        self.is_modified = true;
        &mut self.val
    }
}

impl<T> Deref for TrackedModifiable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

pub struct AppState {
    pub mode: Mode,
    pub current_dir: FileTreeNode,
    pub input_reader: InputReader,
    pub error_popup: Option<ErrorPopup>,
    pub error_message_line: Option<String>,
    pub selected_file: Option<FileTreeNode>,

    pub entered_text: String,

    pub marked_files: Vec<FileTreeNode>,
    pub mark_type: MarkType,
}

pub enum MarkType {
    Delete,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppSettings {
    pub render_timeout: Option<u64>,
    pub global_key_bindings: StringMap,
    pub normal_mode_key_bindings: StringMap,
    pub text_input_mode_key_bindings: StringMap,
    pub min_distance_from_cursor_to_bottom: usize,
    pub default_file_editor_command: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct StyleSet {
    pub file: Style,
    pub dir: Style,
}

pub enum InputReaderDigestResult {
    DigestSuccessful,
    DigestError(String),
}
pub struct InputReader {
    pub modifier_key_sequence: String,
    pub verb_key_sequence: Vec<String>,
}

pub struct ErrorPopup {
    pub title: String,
    pub desc: String,
}

pub trait FindKeyByActionName {
    fn find_key_by_action_name<'a>(&'a self, action_name: &'a str) -> Option<&'a str>;
}
impl FindKeyByActionName for StringMap {
    fn find_key_by_action_name<'a>(&'a self, action_name: &'a str) -> Option<&'a str> {
        self.iter()
            .find(|(_, v)| *v == action_name)
            .map(|(k, _)| k.as_str())
    }
}

impl AppSettings {
    pub fn load_config<P: AsRef<Path>>(paths: Vec<P>) -> io::Result<AppSettings> {
        let config = paths
            .iter()
            .find_map(|path| fs::read_to_string(path).ok())
            .ok_or(Error::new(
                ErrorKind::NotFound,
                "Could not find a config file",
            ))?;

        let config: AppSettings = toml::from_str(config.as_str())?;
        // app_state.global_key_sequence_to_action_mapping = config.global_key_bindings;
        Ok(config)
    }
}

impl AppState {
    pub fn new(current_dir: FileTreeNode) -> Self {
        Self {
            mode: Mode::SimpleMode(SimpleMode::Normal),
            current_dir,
            input_reader: InputReader {
                modifier_key_sequence: String::new(),
                verb_key_sequence: Vec::new(),
            },
            error_popup: None,
            error_message_line: None,
            selected_file: None,

            entered_text: String::new(),
            // NOTE: this would look good for multi-selection, maybe we should use it in the future
            // file: Style::default()
            //     .bg(tui::style::Color::DarkGray)
            //     .fg(tui::style::Color::White),
            // dir: Style::default()
            //     .bg(tui::style::Color::DarkGray)
            //     .fg(tui::style::Color::LightBlue),
            mark_type: MarkType::Delete,
            marked_files: vec![],
        }
    }

    /// Resets all the data (including prompts, error messages entered text and input manager) and changes into the normal mode
    pub fn reset_state(&mut self) {
        self.error_message_line = None;
        self.error_popup = None;

        self.entered_text = String::new();

        self.mode = Mode::SimpleMode(SimpleMode::Normal);
    }
    pub fn copy_input_manager_verbs_to_entered_text(&mut self) {
        let input_verbs_string = &self.input_reader.verb_key_sequence.concat();
        self.entered_text.push_str(input_verbs_string);
    }
    pub fn set_file_cursor_highlight_index<F: FnOnce(usize, usize) -> usize>(
        &mut self,
        dir_items: &Vec<FileTreeNode>,
        get_new_index: F,
    ) {
        let items_num = dir_items.len();
        // avoid something % 0
        let file_cursor_highlight_index = if items_num == 0 {
            0
        } else {
            let file_cursor_highlight_index =
                get_file_cursor_index(&self.selected_file, dir_items).unwrap_or(0);
            // wrapping around
            get_new_index(file_cursor_highlight_index, items_num).rem_euclid(items_num)
        };

        self.selected_file = dir_items
            .get(file_cursor_highlight_index)
            .map(|e| e.to_owned());
    }
    pub fn error_popup(&mut self, title: String, body: String) {
        self.error_popup = Some(ErrorPopup::new(title, body));
    }
}

impl InputReader {
    pub fn get_human_friendly_verb_key_sequence(&self) -> String {
        self.verb_key_sequence
            .iter()
            .fold(String::from(""), |acc, el| {
                if acc.is_empty() {
                    acc + el.to_string().as_str()
                } else {
                    acc + format!(" {}", el).as_str()
                }
            })
    }
    pub fn clear(&mut self) {
        self.modifier_key_sequence.clear();
        self.verb_key_sequence.clear();
    }

    pub fn digest(&mut self, key: KeyCode, force_pushing_as_verb: bool) -> InputReaderDigestResult {
        if let KeyCode::Char(character) = key {
            // if it is a modifier
            if !force_pushing_as_verb && character.is_digit(10) {
                self.modifier_key_sequence.push(character);

                // we can not add a movement after a verb, so fail in that case
                if !self.verb_key_sequence.is_empty() {
                    self.clear();
                    return InputReaderDigestResult::DigestError(String::from(
                        "Can not have a verb modifier after an verb",
                    ));
                }
            } else {
                self.verb_key_sequence.push(character.to_string());
            }
        } else if let KeyCode::Esc = key {
            self.verb_key_sequence.push("ESC".to_string());
        } else if let KeyCode::Backspace = key {
            self.verb_key_sequence.push("BACKSPACE".to_string());
        } else if let KeyCode::Enter = key {
            self.verb_key_sequence.push("ENTER".to_string());
        }
        InputReaderDigestResult::DigestSuccessful
    }

    pub fn get_closure_by_key_bindings<'a>(
        &self,
        key_to_action_mapping: &'a BTreeMap<String, String>,
        action_to_closure_mapping: &'a ActionMapper,
    ) -> Option<&'a ActionClosure> {
        if let Some(action_name) =
            key_to_action_mapping.get(&self.get_human_friendly_verb_key_sequence())
        {
            return action_to_closure_mapping.find_action(action_name);
        }
        return None;
    }

    pub fn check_incomplete_commands(
        &self,
        current_sequence: &str,
        possiblities: Vec<&BTreeMap<String, String>>,
    ) -> bool {
        possiblities.iter().any(|map| {
            map.keys()
                .any(|string| string.starts_with(current_sequence))
        })
    }
}

impl ErrorPopup {
    pub fn new(title: String, desc: String) -> Self {
        ErrorPopup { title, desc }
    }
}
