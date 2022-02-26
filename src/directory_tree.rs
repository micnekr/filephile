use std::ffi::OsString;
use std::fmt::Debug;
use std::io;
use std::path::{Component, Path, PathBuf};

use std::fs::read_dir;

use fuzzy_matcher::skim::SkimMatcherV2;

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
