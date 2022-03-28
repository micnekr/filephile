use tui::widgets::{List, ListItem};

use crate::{
    directory_tree::FileTreeNode,
    helper_types::{AppState, StyleSet, TrackedModifiable},
};

pub fn get_search_mode_left_ui<'a>(
    app_state: &mut TrackedModifiable<AppState>,
    dir_items: &'a Vec<FileTreeNode>,
    cursor_styles: &StyleSet,
    default_styles: &StyleSet,
) -> List<'a> {
    let dir_items: Vec<_> = dir_items
        .iter()
        .enumerate()
        .filter_map(|el| {
            let el_index = el.0;
            let el = el.1;
            Some(el.get_tui_representation(
                &cursor_styles,
                &default_styles,
                el_index == 0,
                &app_state.marked_files,
                &app_state.mark_type,
            ))
        })
        .collect();

    List::new(dir_items)
}
