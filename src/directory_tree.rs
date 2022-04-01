use std::ffi::OsString;
use std::io::{self, stdin, BufRead, Stdout};
use std::path::{Component, Path, PathBuf};

use std::fs::{canonicalize, read_dir};
use std::time::Duration;

use crossbeam_channel::{select, tick, Receiver};
use fuzzy_matcher::skim::SkimMatcherV2;
use tui::backend::CrosstermBackend;
use tui::style::Style;
use tui::text::{Span, Spans};
use tui::widgets::ListItem;
use tui::Terminal;

use crate::helper_types::{MarkType, StyleSet};
use crate::{enter_captured_mode, exit_captured_mode};

#[derive(Clone)]
pub struct FileTreeNode {
    pub(self) path_buf: PathBuf,
    pub(self) simple_name: String,
}

// taken from here (I am assuming MIT license applies?):
// https://github.com/rust-lang/cargo/blob/master/crates/cargo-util/src/paths.rs
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

impl FileTreeNode {
    pub(crate) fn new(path: PathBuf) -> FileTreeNode {
        let path = normalize_path(&path);
        let simple_os_string_name = path.file_name().map(OsString::from);

        let simple_os_string_name = if let Some(mut simple_os_string_name) = simple_os_string_name {
            if path.is_dir() {
                simple_os_string_name.push("/");
            }

            simple_os_string_name
        } else {
            OsString::from("/")
        };

        // get the file name
        let simple_name = simple_os_string_name.to_string_lossy().into_owned();
        FileTreeNode {
            path_buf: path.to_path_buf(),
            simple_name,
        }
    }

    pub fn get_tui_representation(
        &self,
        cursor_styles: &StyleSet,
        default_styles: &StyleSet,
        is_cursor: bool,
        marks: &Vec<FileTreeNode>,
        mark_type: &MarkType,
    ) -> ListItem {
        let mark = if marks.contains(self) {
            Some(mark_type)
        } else {
            None
        };

        let mut spans_vec = match mark {
            None => vec![],
            Some(&MarkType::Delete) => vec![
                Span::styled("D", Style::default().fg(tui::style::Color::Red)),
                Span::from("|"),
            ],
        };

        spans_vec.push(Span::raw(self.get_simple_name().clone()));

        // choose the style based on whether it is a directory or a file and whether it is selected
        let styles_set = if is_cursor {
            &cursor_styles
        } else {
            &default_styles
        };
        let out = ListItem::new(Spans::from(spans_vec)).style(if self.is_dir() {
            styles_set.dir.clone()
        } else {
            styles_set.file.clone()
        });

        out
    }
    pub(crate) fn get_path_buf(&self) -> &PathBuf {
        &self.path_buf
    }

    pub(crate) fn get_simple_name(&self) -> &String {
        &self.simple_name
    }

    pub(crate) fn is_dir(&self) -> bool {
        self.path_buf.is_dir()
    }

    pub(crate) fn compute_score(&self, query: &str) -> i64 {
        let match_data =
            SkimMatcherV2::default()
                .smart_case()
                .fuzzy(&self.simple_name, query, true);
        match match_data {
            None => 0,
            Some(match_data) => match_data.0,
        }
    }

    pub(crate) fn list_files(&self) -> io::Result<Vec<FileTreeNode>> {
        let mut ret = Vec::new();
        for entry in read_dir(self.path_buf.clone())? {
            let resolved_entry = entry?;
            ret.push(FileTreeNode::new(resolved_entry.path()));
        }
        Ok(ret)
    }
}

impl PartialEq for FileTreeNode {
    fn eq(&self, other: &Self) -> bool {
        self.path_buf == other.path_buf
    }
}

pub(crate) fn get_file_cursor_index(
    selected_file: &Option<FileTreeNode>,
    items: &Vec<FileTreeNode>,
) -> Option<usize> {
    selected_file.as_ref().and_then(|selected_file| {
        items
            .iter()
            .position(|el| selected_file.path_buf == el.path_buf)
    })
}

pub(crate) fn run_command_in_foreground<I: Iterator<Item = String>>(
    mut options: I,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    relative_path_current_dir: &PathBuf,
    interrupt_signal_receiver: &Receiver<()>,
    command_status_refresh_secs: f64,
    pause_before_exiting: bool,
) {
    // open the file
    // move to a different screen
    exit_captured_mode(terminal).expect("Could not leave terminal capture");
    // NOTE: current_dir()'s behaviour is up to the implementation if the path is relative,
    // So we need to make it canonical
    let panic_message = format!(
        r#"Encountered an error while tried to convert "{}" to an absolute path"#,
        relative_path_current_dir.as_os_str().to_string_lossy()
    );
    let absolute_path_current_dir = canonicalize(relative_path_current_dir).expect(&panic_message);
    let mut command = std::process::Command::new(
        options
            .next()
            .expect("Found empty arguments for the default file editor"),
    );
    command.current_dir(absolute_path_current_dir);

    options.for_each(|o| {
        command.arg(o);
    });

    let mut handle = command.spawn().expect("Error: Failed to run the command");

    let ticks = tick(Duration::from_millis(
        (command_status_refresh_secs * 100f64) as u64,
    ));

    loop {
        match handle.try_wait() {
            Ok(Some(_)) => break, // it has exited, and so can we
            Ok(None) => {
                select! {
                    recv(ticks) -> _ => {
                    }
                    recv(interrupt_signal_receiver) -> _ => {
                        handle.kill().expect("Could not kill the command");
                        break; // terminate the child and exit
                    }
                }
            }
            Err(e) => panic!("Error attempting to wait for a command: {}", e),
        }
    }

    if pause_before_exiting {
        println!("The command has terminated. Press ENTER to continue.");
        let stdin = stdin();
        let mut buf = String::new();
        stdin
            .lock()
            .read_line(&mut buf)
            .expect("Could not read the input");
    }

    enter_captured_mode(terminal).expect("Could not re-enter the terminal capture");
}
