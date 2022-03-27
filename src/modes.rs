pub mod normal_mode;
pub mod search_mode;

use std::{
    cmp::Ordering,
    fs::{self, File},
    io::Read,
};

use tui::{
    style::Style,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{
    actions::{ActionMapper, ActionResult, NORMAL_MODE_ACTION_MAP},
    compile_time_settings::PREVIEW_TEXT_FETCH_LENGTH,
    directory_tree::FileTreeNode,
};

pub enum Mode {
    SimpleMode(SimpleMode),
    OverlayMode {
        background_mode: SimpleMode,
        overlay_mode: OverlayMode,
    },
    TextInputMode {
        text_input_type: TextInput,
    }, // a mode which acts like a prompt, but can also present its own view
}

pub enum SimpleMode {
    Normal,
    Quitting,
}

pub enum TextInput {
    Search,
}

pub enum OverlayMode {
    Rename { old_file: FileTreeNode },
}

impl Mode {
    pub fn get_action_map(&self) -> ActionMapper {
        match self {
            Mode::SimpleMode(SimpleMode::Quitting) => unreachable!(), // should have exited the program by now
            Mode::SimpleMode(SimpleMode::Normal) => {
                ActionMapper::StaticActionMap(&NORMAL_MODE_ACTION_MAP)
            }
            Mode::TextInputMode {
                text_input_type: TextInput::Search,
            } => ActionMapper::new_dynamic(
                String::from("select"),
                Box::new(|v| {
                    if let Some(first_item) = v.dir_items.get(0) {
                        v.app_state.get_mut().selected_file = Some(first_item.to_owned());
                    }

                    v.app_state.get_mut().reset_state();

                    ActionResult::Valid
                }),
            ),
            Mode::OverlayMode {
                overlay_mode: OverlayMode::Rename { old_file },
                ..
            } => {
                let old_file = old_file.to_owned();
                ActionMapper::new_dynamic(
                    String::from("select"),
                    Box::new(move |v| {
                        // TODO: do we want to check if the new name is available?
                        // NOTE: this check is not 100% reliable because of the race condition.

                        let new_name = &v.app_state.entered_text;
                        let mut new_path = v.app_state.current_dir.get_path_buf().clone();
                        new_path.push(new_name);

                        // reset the mode
                        v.app_state.get_mut().reset_state();

                        match fs::rename(old_file.get_path_buf(), new_path) {
                            Ok(_) => ActionResult::Valid,
                            Err(err) => {
                                ActionResult::Invalid(format!("Error while renaming: {}", err))
                            }
                        }
                    }),
                )
            }
        }
    }
}

impl OverlayMode {
    pub fn get_popup_text(&self, typed_text: String) -> Paragraph {
        let title = match &self {
            OverlayMode::Rename { old_file } => {
                format!("Renaming '{}'", old_file.get_simple_name())
            }
        };
        let block = Block::default().title(title).borders(Borders::ALL);

        Paragraph::new(vec![Spans::from(vec![
            Span::raw("New name: '"),
            Span::styled(typed_text, Style::default().fg(tui::style::Color::Blue)),
            Span::raw("'"),
        ])])
        .block(block)
        .alignment(tui::layout::Alignment::Center)
        .wrap(Wrap { trim: false })
    }
}

impl TextInput {
    pub fn represent_text_line(&self, text_line: &str) -> String {
        match &self {
            TextInput::Search => format!("/{}", text_line),
        }
    }
}

impl Mode {
    pub fn is_text_mode(&self) -> bool {
        match &self {
            Mode::SimpleMode(_) => false,
            Mode::OverlayMode { .. } | Mode::TextInputMode { .. } => true,
        }
    }
}

// misc functions used by multiple modes
pub fn cmp_by_dir_and_path(a: &FileTreeNode, b: &FileTreeNode) -> Ordering {
    let is_a_dir = a.is_dir();
    let is_b_dir = b.is_dir();
    if is_a_dir ^ is_b_dir {
        if is_a_dir {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    } else {
        a.get_path_buf().cmp(b.get_path_buf())
    }
}
pub fn get_file_text_preview(f: &FileTreeNode) -> Option<String> {
    // let extension = f.get_path_buf().extension().unwrap_or(OsStr::new(""));

    let mut buffer = [0; PREVIEW_TEXT_FETCH_LENGTH];
    let opened_file = File::open(f.get_path_buf()).ok();

    opened_file
        .and_then(|mut opened_file| opened_file.read(&mut buffer).ok())
        .map(|n| String::from_utf8_lossy(&buffer[..n]).into_owned())
}
