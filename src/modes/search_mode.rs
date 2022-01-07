use std::ffi::OsString;

use tui::{
    layout::Rect,
    widgets::{Block, List, ListItem, Paragraph, Wrap},
};

use crate::{directory_tree::FileTreeNode, AppState, StyleSet};

use super::{cmp_by_dir_and_path, AllowedWidgets, ModeController, RecordedModifiable};

#[derive(Clone)]
pub struct SearchModeController<'a> {
    pub(self) has_been_modified: bool,
    pub(self) search_string: String,
    pub(self) error_message: Option<&'a String>,
    pub(self) default_styles: StyleSet,
    pub(self) cursor_styles: StyleSet,
}

impl RecordedModifiable for SearchModeController<'_> {
    fn reset_modification_status(&mut self) {
        self.has_been_modified = false;
    }

    fn mark_as_modified(&mut self) {
        self.has_been_modified = true;
    }

    fn has_been_modified(&self) -> bool {
        self.has_been_modified
    }
}

impl<'a> SearchModeController<'a> {
    pub fn new(default_styles: StyleSet, cursor_styles: StyleSet) -> Self {
        SearchModeController {
            error_message: None,
            search_string: String::new(),
            has_been_modified: false,
            default_styles,
            cursor_styles,
        }
    }

    pub fn move_input(&mut self, app_state: &mut AppState) {
        self.mark_as_modified();
        self.search_string
            .push_str(&app_state.input_reader.verb_key_sequence.concat());
        app_state.input_reader.clear();
    }

    pub fn delete_last_char(&mut self) {
        self.mark_as_modified();
        self.search_string.pop();
    }

    pub fn clear(&mut self) {
        self.mark_as_modified();
        self.search_string.clear();
    }
}

impl<'a> ModeController<'a> for SearchModeController<'a> {
    fn get_left_ui(
        &mut self,
        block: Block<'a>,
        _: Rect,
        dir_items: &Vec<FileTreeNode>,
    ) -> AllowedWidgets<'a> {
        let dir_items: Vec<_> = dir_items
            .iter()
            .enumerate()
            .filter_map(|el| {
                let el_index = el.0;
                let el = el.1;
                let out = ListItem::new(el.get_simple_name().clone());

                // apply styles

                let styles_set = if el_index == 0 {
                    &self.cursor_styles
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
        let contents = if let Some(error_message) = self.error_message {
            error_message.to_owned()
        } else {
            String::from("/") + &self.search_string
        };
        Some(AllowedWidgets::ParagraphWrapper(
            Paragraph::new(contents).wrap(Wrap { trim: true }),
        ))
    }

    fn transform_dir_items(&self, mut dir_items: Vec<FileTreeNode>) -> Vec<FileTreeNode> {
        struct FileTreeNodeWrapper {
            item: FileTreeNode,
            score: i64,
        }

        if self.search_string.is_empty() {
            dir_items.sort_by(|a, b| cmp_by_dir_and_path(a, b));
            return dir_items;
        }
        let mut dir_items: Vec<_> = dir_items
            .into_iter()
            .map(|el| FileTreeNodeWrapper {
                score: el.get_score(&self.search_string),
                item: el,
            })
            .filter(|el| el.score > 0)
            .collect();

        // TODO: properly sort these ones, maybe with caching
        dir_items.sort_by(|a, b| {
            if a.score != b.score {
                return b.score.cmp(&a.score);
            }
            cmp_by_dir_and_path(&a.item, &b.item)
        });

        dir_items.into_iter().map(|el| el.item).collect()
    }
}
