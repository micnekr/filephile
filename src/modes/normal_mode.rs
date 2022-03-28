use tui::widgets::{List, ListItem};

use crate::{
    directory_tree::{get_file_cursor_index, FileTreeNode},
    helper_types::{AppState, StyleSet, TrackedModifiable},
};

pub fn get_normal_mode_left_ui<'a>(
    app_state: &mut TrackedModifiable<AppState>,
    dir_items: &'a Vec<FileTreeNode>,
    min_distance_from_cursor_to_bottom: usize,
    cursor_styles: StyleSet,
    default_styles: StyleSet,
    height_of_list_available: usize,
) -> List<'a> {
    let file_cursor_highlight_index = get_file_cursor_index(&app_state.selected_file, dir_items);

    // reset the selected file to the first element if it was not found
    let file_cursor_highlight_index = file_cursor_highlight_index.unwrap_or_else(|| {
        app_state.get_mut().selected_file = dir_items.get(0).map(|e| e.to_owned());
        0
    });

    // how many list elements to skip to give the appearance of scrolling
    let num_to_skip =
            // Do not do anything if it all fits in on one screen
            if dir_items.len() <= height_of_list_available {
                0

            // Do not do anything if it can all be seen on one screen
            } else if min_distance_from_cursor_to_bottom + file_cursor_highlight_index < height_of_list_available {
                0
            // if the viewport is full and the cursor is close to the bottom, but there are still concealed items later on in the list
            } else if dir_items.len() > file_cursor_highlight_index + min_distance_from_cursor_to_bottom {
                file_cursor_highlight_index + min_distance_from_cursor_to_bottom - height_of_list_available
            // show the last page
            } else {
                dir_items.len() - height_of_list_available
            };

    let dir_items: Vec<_> = dir_items
        .iter()
        .enumerate()
        .filter_map(|el| {
            let el_index = el.0;
            let el = el.1;
            // skip if we are scrolling upwards
            if el_index < num_to_skip {
                return None;
            }

            Some(el.get_tui_representation(
                &cursor_styles,
                &default_styles,
                el_index == file_cursor_highlight_index,
                &app_state.marked_files,
                &app_state.mark_type,
            ))
        })
        .collect();

    List::new(dir_items)
}
