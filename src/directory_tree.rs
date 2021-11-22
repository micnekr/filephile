use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use std::fs::{read_dir, DirEntry, ReadDir};

pub(crate) struct FileTreeNode {
    // Make it impossible to modify it from the outside
    pub(self) path_buf: PathBuf,
    pub(self) is_dir: bool,
}

impl FileTreeNode {
    pub fn new(path: &Path) -> io::Result<FileTreeNode> {
        Ok(FileTreeNode {
            is_dir: path.is_dir(),
            path_buf: path.to_path_buf(),
        })
    }

    pub fn get_dir_path(&self) -> &PathBuf {
        &self.path_buf
    }
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn list_files(&self) -> io::Result<Vec<OsString>> {
        let mut ret = Vec::new();
        for entry in read_dir(self.path_buf.clone())? {
            let resolved_entry = entry?;
            ret.push(resolved_entry.file_name());
        }
        Ok(ret)
    }
}
