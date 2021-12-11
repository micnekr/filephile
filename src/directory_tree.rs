use std::ffi::OsString;
use std::io;
use std::path::{Component, Path, PathBuf};

use std::fs::read_dir;

use fuzzy_matcher::skim::SkimMatcherV2;

use crate::modes::RecordedModifiable;
use crate::StyleSet;

#[derive(Clone)]
pub(crate) struct FileTreeNode {
    // Make it impossible to modify it from the outside
    pub(self) path_buf: PathBuf,
    pub(self) is_dir: bool,
    // pub(self) simple_os_string_name: OsString,
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
    pub(crate) fn new(path: PathBuf) -> io::Result<FileTreeNode> {
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
        Ok(FileTreeNode {
            is_dir: path.is_dir(),
            path_buf: path.to_path_buf(),
            // simple_os_string_name,
            simple_name,
        })
    }

    pub(crate) fn get_simple_name(&self) -> &String {
        &self.simple_name
    }

    pub(crate) fn get_score(&self, query: &str) -> i64 {
        let match_data =
            SkimMatcherV2::default()
                .smart_case()
                .fuzzy(&self.simple_name, query, true);
        match match_data {
            None => 0,
            Some(match_data) => match_data.0,
        }
    }

    pub(crate) fn get_path(&self) -> &PathBuf {
        &self.path_buf
    }
    pub(crate) fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub(crate) fn get_files(&self) -> io::Result<Vec<FileTreeNode>> {
        let mut ret = Vec::new();
        for entry in read_dir(self.path_buf.clone())? {
            let resolved_entry = entry?;
            ret.push(FileTreeNode::new(resolved_entry.path())?);
        }
        Ok(ret)
    }
}

// #[derive(Clone)]
// pub(crate) struct FileSelectionMultiple {
//     pub selected_files: BTreeSet<OsString>,
//     pub styles: StyleSet,
// }

#[derive(Clone)]
pub struct FileSelectionSingle {
    pub(self) has_been_modified: bool,
    pub(self) selected_file: Option<OsString>,
    pub(self) styles: StyleSet,
}

pub(crate) trait FileSelection {
    fn is_selected(&self, node: &FileTreeNode) -> bool;
    fn get_styles(&self) -> &StyleSet;
}

// impl FileSelection for FileSelectionMultiple {
//     fn is_selected(&self, node: &FileTreeNode) -> bool {
//         self.selected_files.contains(node.get_path().as_os_str())
//     }
//     fn get_styles(&self) -> &StyleSet {
//         &self.styles
//     }
// }

impl FileSelection for FileSelectionSingle {
    fn is_selected(&self, node: &FileTreeNode) -> bool {
        if let Some(selected_file) = &self.selected_file {
            selected_file == node.get_path().as_os_str()
        } else {
            false
        }
    }
    fn get_styles(&self) -> &StyleSet {
        &self.styles
    }
}

impl RecordedModifiable for FileSelectionSingle {
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

impl FileSelectionSingle {
    pub(crate) fn new(styles: StyleSet) -> Self {
        FileSelectionSingle {
            has_been_modified: false,
            selected_file: None,
            styles,
        }
    }
    pub(crate) fn set_selected_file(&mut self, selected_file: Option<OsString>) {
        self.mark_as_modified();
        self.selected_file = selected_file;
    }
    pub(crate) fn get_file_cursor_index_or_reset<'a>(
        &'a mut self,
        items: &Vec<FileTreeNode>,
    ) -> Option<usize> {
        items
            .iter()
            .position(|el| self.is_selected(&el))
            .or_else(|| {
                // if the cursor can not be placed:
                // TODO: do this
                // app_state.is_urgent_update = true;
                self.mark_as_modified();
                self.selected_file = items
                    // get the first index
                    .first()
                    .map_or(None, |el| Some(el.get_path().as_os_str().to_owned()));
                // if the directory is empty, skip it. Otherwise, go to the first element
                if self.selected_file.is_some() {
                    Some(0)
                } else {
                    None
                }
            })
    }
    pub(crate) fn get_selected_file(&self) -> &Option<OsString> {
        &self.selected_file
    }
}
