mod normal_mode;
mod search_mode;

use std::cmp::Ordering;

use tui::{
    backend::Backend,
    layout::Rect,
    widgets::{Block, List, Paragraph},
};

use crate::{directory_tree::FileTreeNode, StyleSet};

pub use self::{normal_mode::NormalModeController, search_mode::SearchModeController};

// widgets that can be displayed from within the main loop
pub(crate) enum AllowedWidgets<'a> {
    BlockWrapper(Block<'a>),
    ListWrapper(List<'a>),
    ParagraphWrapper(Paragraph<'a>),
}

impl<'a> AllowedWidgets<'a> {
    pub(crate) fn render<B: Backend>(self, f: &mut tui::Frame<B>, size: Rect) {
        match self {
            AllowedWidgets::BlockWrapper(block) => f.render_widget(block, size),
            AllowedWidgets::ListWrapper(list) => f.render_widget(list, size),
            AllowedWidgets::ParagraphWrapper(paragraph) => f.render_widget(paragraph, size),
        };
    }
    pub(crate) fn block(self, block: Block<'a>) -> Self {
        match self {
            AllowedWidgets::BlockWrapper(_) => {}
            AllowedWidgets::ListWrapper(list) => {
                return AllowedWidgets::ListWrapper(list.block(block));
            }
            AllowedWidgets::ParagraphWrapper(paragraph) => {
                return AllowedWidgets::ParagraphWrapper(paragraph.block(block));
            }
        };
        self
    }
}

#[derive(Clone, PartialEq)]
pub(crate) enum Mode {
    Normal,
    Search,
    Quitting,
}

pub(crate) trait RecordedModifiable {
    fn reset_modification_status(&mut self);
    fn mark_as_modified(&mut self);
    fn has_been_modified(&self) -> bool;
}
pub(crate) trait ModeController<'a> {
    fn get_left_ui(
        &mut self,
        block: Block<'a>,
        size: Rect,
        dir_items: &Vec<FileTreeNode>,
    ) -> AllowedWidgets<'a>;
    fn get_right_ui(&'a self, block: Block<'a>, size: Rect) -> AllowedWidgets<'a>;
    fn get_bottom_text(&'a self) -> Option<AllowedWidgets<'a>>;
    fn sort(&'a self, dir_items: &mut Vec<FileTreeNode>);
}

fn cmp_by_dir_and_path(a: &FileTreeNode, b: &FileTreeNode) -> Ordering {
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

#[derive(Clone)]
pub(crate) struct ModesManager<'a> {
    pub(self) has_been_modified: bool,
    pub(self) current_mode: Mode,
    pub(crate) normal_mode_controller: NormalModeController<'a>,
    pub(crate) search_mode_controller: SearchModeController<'a>,
}

impl<'a> ModesManager<'a> {
    pub(crate) fn new(default_styles: StyleSet, cursor_styles: StyleSet) -> ModesManager<'a> {
        ModesManager {
            has_been_modified: false,
            current_mode: Mode::Normal,
            normal_mode_controller: NormalModeController::new(default_styles, cursor_styles),
            search_mode_controller: SearchModeController::new(),
        }
    }
    pub(crate) fn get_current_mode(&mut self) -> &Mode {
        &self.current_mode
    }
    pub(crate) fn set_current_mode(&mut self, new_mode: Mode) {
        self.mark_as_modified();
        self.current_mode = new_mode;
    }
}

impl<'a> ModeController<'a> for ModesManager<'a> {
    fn get_left_ui(
        &mut self,
        block: Block<'a>,
        size: Rect,
        dir_items: &Vec<FileTreeNode>,
    ) -> AllowedWidgets<'a> {
        match self.current_mode {
            Mode::Normal => self
                .normal_mode_controller
                .get_left_ui(block, size, dir_items),
            Mode::Search => self
                .search_mode_controller
                .get_left_ui(block, size, dir_items),
            Mode::Quitting => panic!("Quitting mode has been used without quitting"),
        }
    }

    fn get_right_ui(&'a self, block: Block<'a>, size: Rect) -> AllowedWidgets<'a> {
        match self.current_mode {
            Mode::Normal => self.normal_mode_controller.get_right_ui(block, size),
            Mode::Search => self.search_mode_controller.get_right_ui(block, size),
            Mode::Quitting => panic!("Quitting mode has been used without quitting"),
        }
    }

    fn get_bottom_text(&'a self) -> Option<AllowedWidgets<'a>> {
        match self.current_mode {
            Mode::Normal => self.normal_mode_controller.get_bottom_text(),
            Mode::Search => self.search_mode_controller.get_bottom_text(),
            Mode::Quitting => panic!("Quitting mode has been used without quitting"),
        }
    }

    fn sort(&self, dir_items: &mut Vec<FileTreeNode>) {
        match self.current_mode {
            Mode::Normal => self.normal_mode_controller.sort(dir_items),
            Mode::Search => self.search_mode_controller.sort(dir_items),
            Mode::Quitting => panic!("Quitting mode has been used without quitting"),
        }
    }
}

impl RecordedModifiable for ModesManager<'_> {
    fn reset_modification_status(&mut self) {
        self.has_been_modified = false;
        self.normal_mode_controller.reset_modification_status();
        self.search_mode_controller.reset_modification_status();
    }

    fn mark_as_modified(&mut self) {
        self.has_been_modified = true;
        self.normal_mode_controller.mark_as_modified();
        self.search_mode_controller.mark_as_modified();
    }
    fn has_been_modified(&self) -> bool {
        self.has_been_modified
            || self.normal_mode_controller.has_been_modified()
            || self.search_mode_controller.has_been_modified()
    }
}