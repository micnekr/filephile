use std::fs;

use std::io::Result;

use crate::directory_tree::FileTreeNode;

pub fn delete_file_tree_node(file_tree_node: &FileTreeNode) -> Result<()> {
    if file_tree_node.is_dir() {
        fs::remove_dir_all(file_tree_node.get_path_buf())
    } else {
        fs::remove_file(file_tree_node.get_path_buf())
    }
}
