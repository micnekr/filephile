use tui::widgets::{List, ListItem};

use crate::{
    directory_tree::{get_file_cursor_index, FileTreeNode},
    helper_types::{AppState, NormalModeState, StyleSet, TrackedModifiable},
};

impl NormalModeState {
    pub fn get_left_ui<'a>(
        app_state: &mut TrackedModifiable<AppState>,
        dir_items: &'a Vec<FileTreeNode>,
        min_distance_from_cursor_to_bottom: usize,
        cursor_styles: StyleSet,
        default_styles: StyleSet,
        height_of_list_available: usize,
    ) -> List<'a> {
        let file_cursor_highlight_index =
            get_file_cursor_index(&app_state.selected_file, dir_items);

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
                let out = ListItem::new(el.get_simple_name().clone());

                // apply styles

                // different styles depending on whether it is selected or not and whether it si a dir or not
                // It is only None if the directory is empty, which would make the code below not be executed. Unwrap is safe.
                let styles_set = if el_index == file_cursor_highlight_index {
                    &cursor_styles
                } else {
                    &default_styles
                };
                let out = out.style(if el.is_dir() {
                    styles_set.dir.clone()
                } else {
                    styles_set.file.clone()
                });

                Some(out)
            })
            .collect();

        List::new(dir_items)
    }
}
