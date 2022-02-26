mod normal_mode;
mod search_mode;

use std::{cmp::Ordering, fs::File, io::Read};

use crate::{compile_time_settings::PREVIEW_TEXT_FETCH_LENGTH, directory_tree::FileTreeNode};

#[derive(Clone, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    Quitting,
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
