use tui::{
    layout::Rect,
    widgets::{Block, Paragraph, Wrap},
};

use crate::directory_tree::FileTreeNode;

use super::{cmp_by_dir_and_path, AllowedWidgets, ModeController, RecordedModifiable};

#[derive(Clone)]
pub struct SearchModeController<'a> {
    pub(self) has_been_modified: bool,
    pub(self) search_string: String,
    pub(self) error_message: Option<&'a String>,
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
    pub fn new() -> Self {
        SearchModeController {
            error_message: None,
            search_string: String::new(),
            has_been_modified: false,
        }
    }
}

impl<'a> ModeController<'a> for SearchModeController<'a> {
    fn get_left_ui(
        &mut self,
        block: Block<'a>,
        _: Rect,
        _: &Vec<FileTreeNode>,
    ) -> AllowedWidgets<'a> {
        AllowedWidgets::BlockWrapper(block)
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

    fn sort(&self, dir_items: &mut Vec<FileTreeNode>) {
        if self.search_string.is_empty() {
            dir_items.sort_by(|a, b| cmp_by_dir_and_path(a, b));
            return;
        }
        // TODO: properly sort these ones, maybe with caching
        dir_items.sort_by(|a, b| {
            let a_score = a.get_score(&self.search_string);
            let b_score = b.get_score(&self.search_string);

            if a_score != b_score {
                return a_score.cmp(&b_score);
            }
            cmp_by_dir_and_path(a, b)
        });
    }
}
