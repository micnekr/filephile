use std::{cmp::Ordering, io};

use tui::{
    backend::Backend,
    layout::Rect,
    widgets::{Block, List, ListItem, Paragraph, Wrap},
};

use crate::{
    directory_tree::{FileSelection, FileSelectionSingle, FileTreeNode},
    StyleSet,
};

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

#[derive(Clone)]
pub(crate) struct NormalModeController<'a> {
    pub(self) error_message: Option<&'a String>,
    pub(self) file_cursor: FileSelectionSingle,
    pub(self) default_styles: StyleSet,
    pub(self) max_distance_from_cursor_to_bottom: usize,
    pub(self) has_been_modified: bool,
}

#[derive(Clone)]
pub(crate) struct SearchModeController<'a> {
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

impl<'a> SearchModeController<'a> {
    fn new() -> Self {
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
impl<'a> NormalModeController<'a> {
    pub(crate) fn new(default_styles: StyleSet, cursor_styles: StyleSet) -> Self {
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

    fn sort(&self, dir_items: &mut Vec<FileTreeNode>) {
        dir_items.sort_by(|a, b| cmp_by_dir_and_path(a, b));
    }
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
