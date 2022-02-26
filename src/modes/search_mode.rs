impl SearchModeState {
    pub fn get_left_ui<'a>(
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
                let out = ListItem::new(el.get_simple_name().clone());

                // apply styles

                let styles_set = if el_index == 0 {
                    cursor_styles
                } else {
                    default_styles
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

use tui::widgets::{List, ListItem};

use crate::{
    directory_tree::FileTreeNode,
    helper_types::{SearchModeState, StyleSet},
};
