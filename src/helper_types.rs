use crate::{actions::NameMap, directory_tree::FileTreeNode};
use crossterm::event::KeyCode;
use std::{collections::BTreeMap, fs, io, path::Path};

use serde::{Deserialize, Serialize};
use tui::style::Style;

use crate::modes::RecordedModifiable;

type StringMap = BTreeMap<String, String>;

pub struct AppState {
    pub(self) has_been_modified: bool,
    pub(self) current_dir: FileTreeNode,
    pub(self) input_reader: InputReader, // is_urgent_update: bool,

    pub(self) error_popup: Option<ErrorPopup>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppSettings {
    pub render_timeout: Option<u64>,
    pub global_key_bindings: StringMap,
    pub normal_mode_key_bindings: StringMap,
    pub search_mode_key_bindings: StringMap,
}

#[derive(Clone)]
pub struct StyleSet {
    pub file: Style,
    pub dir: Style,
}

pub struct InputReader {
    modifier_key_sequence: String,
    verb_key_sequence: Vec<String>,
}

pub struct ErrorPopup {
    pub title: String,
    pub desc: String,
}

impl AppSettings {
    pub fn load_config<P: AsRef<Path>>(paths: Vec<P>) -> io::Result<AppSettings> {
        let config = paths
            .iter()
            .find_map(|path| fs::read_to_string(path).ok())
            .ok_or(io::Error::new(
                io::ErrorKind::NotFound,
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
            current_dir,
            input_reader: InputReader {
                modifier_key_sequence: String::new(),
                verb_key_sequence: Vec::new(),
            },
            error_popup: None,
            has_been_modified: false,
            // NOTE: this would look good for multi-selection, maybe we should use it in the future
            // file: Style::default()
            //     .bg(tui::style::Color::DarkGray)
            //     .fg(tui::style::Color::White),
            // dir: Style::default()
            //     .bg(tui::style::Color::DarkGray)
            //     .fg(tui::style::Color::LightBlue),
        }
    }
    pub fn set_current_dir(&mut self, n: FileTreeNode) {
        self.mark_as_modified();
        self.current_dir = n;
    }
    pub fn get_current_dir(&self) -> &FileTreeNode {
        &self.current_dir
    }
    pub fn set_error_popup(&mut self, e: Option<ErrorPopup>) {
        self.mark_as_modified();
        self.error_popup = e;
    }
    pub fn get_error_popup(&self) -> &Option<ErrorPopup> {
        &self.error_popup
    }
    pub fn get_input_reader(&mut self) -> &mut InputReader {
        &mut self.input_reader
    }
}

impl RecordedModifiable for AppState {
    fn reset_modification_status(&mut self) {
        self.has_been_modified = false;
    }

    fn mark_as_modified(&mut self) {
        self.has_been_modified = true;
    }

    fn has_been_modified(&self) -> bool {
        self.has_been_modified
    }
}

impl InputReader {
    pub fn get_verb_key_sequence(&self) -> &Vec<String> {
        &self.verb_key_sequence
    }

    pub fn get_modifier_key_sequence(&self) -> &String {
        &self.modifier_key_sequence
    }

    fn get_human_friendly_verb_key_sequence(&self) -> String {
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

    pub fn record_key_in_state(&mut self, key: KeyCode, allow_modifiers: bool) {
        if let KeyCode::Char(character) = key {
            // if it is a modifier
            if allow_modifiers && character.is_digit(10) {
                self.modifier_key_sequence.push(character);

                // we can not add a movement after a verb, so fail in that case
                if !self.verb_key_sequence.is_empty() {
                    self.clear();
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
    }

    pub fn get_action_closure<'a, MapValueType>(
        &self,
        key_to_action_mapping: &'a BTreeMap<String, String>,
        action_to_closure_mapping: &'a NameMap<MapValueType>,
    ) -> Option<&'a MapValueType> {
        if let Some(action_name) =
            key_to_action_mapping.get(&self.get_human_friendly_verb_key_sequence())
        {
            return action_to_closure_mapping.get(action_name);
        }
        return None;
    }

    pub fn check_for_possible_extensions(
        &self,
        possiblities: Vec<&BTreeMap<String, String>>,
    ) -> bool {
        possiblities.iter().any(|map| {
            map.keys()
                .any(|string| string.starts_with(&self.get_human_friendly_verb_key_sequence()))
        })
    }
}

impl ErrorPopup {
    pub fn new(title: String, desc: String) -> Self {
        ErrorPopup { title, desc }
    }
}
