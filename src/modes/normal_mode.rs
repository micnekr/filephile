use std::io;

use tui::{
    layout::Rect,
    widgets::{Block, List, ListItem, Paragraph, Wrap},
};

use crate::{
    directory_tree::{FileSelection, FileSelectionSingle, FileTreeNode},
    StyleSet,
};

use super::{cmp_by_dir_and_path, AllowedWidgets, ModeController, RecordedModifiable};

#[derive(Clone)]
pub struct NormalModeController<'a> {
    pub(self) error_message: Option<&'a String>,
    pub(self) file_cursor: FileSelectionSingle,
    pub(self) default_styles: StyleSet,
    pub(self) max_distance_from_cursor_to_bottom: usize,
    pub(self) has_been_modified: bool,
}

impl RecordedModifiable for NormalModeController<'_> {
    fn reset_modification_status(&mut self) {
        self.has_been_modified = false;
        self.file_cursor.reset_modification_status();
    }

    fn mark_as_modified(&mut self) {
        self.has_been_modified = true;
        self.file_cursor.mark_as_modified();
    }

    fn has_been_modified(&self) -> bool {
        self.has_been_modified || self.file_cursor.has_been_modified()
    }
}

impl<'a> NormalModeController<'a> {
    pub fn new(default_styles: StyleSet, cursor_styles: StyleSet) -> Self {
        NormalModeController {
            has_been_modified: false,
            file_cursor: FileSelectionSingle::new(cursor_styles),
            error_message: None,
            // TODO: make this configurable
            max_distance_from_cursor_to_bottom: 4,
            default_styles,
        }
    }

    pub(crate) fn change_file_cursor_index<
        F: Fn(Option<usize>, &Vec<FileTreeNode>) -> Option<usize>,
    >(
        &mut self,
        dir_items: &Vec<FileTreeNode>,
        update_function: F,
    ) -> io::Result<()> {
        let file_cursor_index = self.file_cursor.get_file_cursor_index_or_reset(dir_items);
        let new_file_cursor_index = update_function(file_cursor_index, dir_items);

        // Update
        // TODO: what do we do if the index is past its max value?: new_file_cursor_index < dir_items.len()
        if new_file_cursor_index != file_cursor_index {
            // if no index, return None
            self.file_cursor
                .set_selected_file(new_file_cursor_index.map_or(None, |index| {
                    dir_items
                        .get(index)
                        // if not found, return None
                        .map_or(None, |file| Some(file.get_path().as_os_str().to_owned()))
                }));
        }

        Ok(())
    }
    pub fn get_file_cursor_mut(&mut self) -> &mut FileSelectionSingle {
        &mut self.file_cursor
    }
    pub fn get_file_cursor(&self) -> &FileSelectionSingle {
        &self.file_cursor
    }
}

impl<'a> ModeController<'a> for NormalModeController<'a> {
    fn get_left_ui(
        &mut self,
        block: Block<'a>,
        size: Rect,
        dir_items: &Vec<FileTreeNode>,
    ) -> AllowedWidgets<'a> {
        let file_cursor_index = self.file_cursor.get_file_cursor_index_or_reset(dir_items);

        let height_of_list_available = size.height as usize - 2; // -2 because one line from each side is used for the border

        let num_to_skip = file_cursor_index.map_or(None, |index| {
            // how far is the index from its desired position?
            Some(
                // Do not do anything if it all fits in on one screen
                if dir_items.len() <= height_of_list_available {
                    0

                // Do not do anything if it can all be seen on one screen
                } else if self.max_distance_from_cursor_to_bottom + index < height_of_list_available
                {
                    0
                // if the viewport is full and the cursor is close to the bottom, but there are still concealed items later on in the list
                } else if dir_items.len() > index + self.max_distance_from_cursor_to_bottom {
                    index + self.max_distance_from_cursor_to_bottom - height_of_list_available
                } else {
                    dir_items.len() - height_of_list_available
                },
            )
        });

        let dir_items: Vec<_> = dir_items
            .iter()
            .enumerate()
            .filter_map(|el| {
                let el_index = el.0;
                let el = el.1;
                // skip if we are scrolling upwards
                if let Some(num_to_skip) = num_to_skip {
                    if el_index < num_to_skip {
                        return None;
                    }
                }
                let out = ListItem::new(el.get_simple_name().clone());

                // apply styles

                // different styles depending on whether it is selected or not and whether it si a dir or not
                // It is only None if the directory is empty, which would make the code below not be executed. Unwrap is safe.
                let styles_set = if el_index == file_cursor_index.unwrap() {
                    self.file_cursor.get_styles()
                } else {
                    &self.default_styles
                };
                let out = out.style(if el.is_dir() {
                    styles_set.dir.clone()
                } else {
                    styles_set.file.clone()
                });

                Some(out)
            })
            .collect();

        let list = List::new(dir_items);
        AllowedWidgets::ListWrapper(list.block(block))
    }

    fn get_right_ui(&self, block: Block<'a>, _: Rect) -> AllowedWidgets<'a> {
        AllowedWidgets::BlockWrapper(block)
    }

    fn get_bottom_text(&self) -> Option<AllowedWidgets<'a>> {
        if let Some(error_message) = self.error_message {
            Some(AllowedWidgets::ParagraphWrapper(
                Paragraph::new(error_message.as_str()).wrap(Wrap { trim: true }),
            ))
        } else {
            None
        }
    }

    fn transform_dir_items(&self, mut dir_items: Vec<FileTreeNode>) -> Vec<FileTreeNode> {
        dir_items.sort_by(|a, b| cmp_by_dir_and_path(a, b));
        dir_items
    }
}
