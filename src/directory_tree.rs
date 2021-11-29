use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use std::fs::read_dir;

use crate::ui::StyleSet;

pub(crate) struct FileTreeNode {
    // Make it impossible to modify it from the outside
    pub(self) path_buf: PathBuf,
    pub(self) is_dir: bool,
    pub(self) simple_name: Option<OsString>, // pub(self) dir_entry: DirEntry,
}

impl FileTreeNode {
    pub(crate) fn new(path: PathBuf) -> io::Result<FileTreeNode> {
        let path = path.canonicalize()?;
        let mut simple_name = path.file_name().map(OsString::from);
        if path.is_dir() {
            if let Some(mut el) = simple_name {
                el.push("/");
                simple_name = Some(el);
            }
        }
        Ok(FileTreeNode {
            is_dir: path.is_dir(),
            path_buf: path.to_path_buf(),
            simple_name,
        })
    }

    pub(crate) fn get_simple_name(&self) -> io::Result<OsString> {
        if let Some(simple_name) = &self.simple_name {
            Ok(simple_name.to_owned())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Can not display a name for this file or directory",
            ))
        }
    }

    pub(crate) fn get_path(&self) -> &PathBuf {
        &self.path_buf
    }
    pub(crate) fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub(crate) fn list_files(
        &self,
        file_tree_node_sorter: &FileTreeNodeSorter,
    ) -> io::Result<Vec<FileTreeNode>> {
        let mut ret = Vec::new();
        for entry in read_dir(self.path_buf.clone())? {
            let resolved_entry = entry?;
            ret.push(FileTreeNode::new(resolved_entry.path())?);
        }
        ret.sort_by(|el1, el2| file_tree_node_sorter.cmp(el1, el2));
        Ok(ret)
    }
}

pub(crate) enum FileTreeNodeSorter {
    NORMAL,
}

impl FileTreeNodeSorter {
    pub(crate) fn cmp(&self, a: &FileTreeNode, b: &FileTreeNode) -> Ordering {
        match self {
            &FileTreeNodeSorter::NORMAL => {
                let is_a_dir = a.is_dir();
                let is_b_dir = b.is_dir();
                if is_a_dir ^ is_b_dir {
                    if is_a_dir {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    }
                } else {
                    a.get_path().cmp(b.get_path())
                }
            }
        }
    }
}

pub(crate) struct FileSelectionMultiple {
    pub selected_files: BTreeSet<OsString>,
    pub styles: StyleSet,
}

pub(crate) struct FileSelectionSingle {
    pub selected_file: Option<OsString>,
    pub styles: StyleSet,
}

pub(crate) trait FileSelection {
    fn is_selected(&self, node: &FileTreeNode) -> bool;
    fn get_styles(&self) -> &StyleSet;
}

impl FileSelection for FileSelectionMultiple {
    fn is_selected(&self, node: &FileTreeNode) -> bool {
        self.selected_files.contains(node.get_path().as_os_str())
    }
    fn get_styles(&self) -> &StyleSet {
        &self.styles
    }
}

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

impl FileSelectionSingle {
    pub(crate) fn get_file_cursor_index(&self, items: &Vec<FileTreeNode>) -> Option<usize> {
        items.iter().position(|el| self.is_selected(&el))
    }
}
