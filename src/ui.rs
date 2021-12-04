use std::{borrow::Borrow, io};

use fuzzy_matcher::skim::SkimMatcherV2;
use tui::{
    layout::{Constraint, Direction::Vertical},
    style::Style,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::{
    directory_tree::{FileSelection, FileTreeNode},
    AppState, Mode,
};

pub(crate) struct StyleSet {
    pub file: Style,
    pub dir: Style,
}

struct DisplayListItem<'a> {
    file_tree_node: &'a FileTreeNode,
    file_name: String,
    search_score: i64,
}

pub(crate) fn ui<B: tui::backend::Backend>(
    f: &mut tui::Frame<B>,
    app_state: &mut AppState,
) -> io::Result<()> {
    let dir_path_string = app_state
        .current_dir
        .get_path()
        .as_os_str()
        .to_string_lossy()
        .into_owned();

    let error_message_present = !app_state.error_message.is_empty();
    let search_string_present = app_state.app_mode == crate::Mode::SEARCH;

    let chunks = tui::layout::Layout::default()
        .direction(Vertical)
        // .margin(0)
        .constraints(
            [
                Constraint::Min(1),
                Constraint::Length(if error_message_present { 3 } else { 0 }),
                Constraint::Length(if search_string_present { 3 } else { 0 }),
            ]
            .as_ref(),
        )
        .split(f.size());
    // error message
    let block = Block::default().borders(Borders::ALL);
    let paragraph = Paragraph::new(app_state.error_message.borrow())
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, chunks[1]);
    // search string
    let block = Block::default().borders(Borders::ALL);
    let paragraph = Paragraph::new(String::from("/") + app_state.search_string.borrow())
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, chunks[2]);
    {
        let chunks = tui::layout::Layout::default()
            .direction(tui::layout::Direction::Horizontal)
            // .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[0]);

        let block = Block::default()
            .title(dir_path_string)
            .borders(Borders::ALL);

        let is_searching =
            app_state.app_mode == Mode::SEARCH && !app_state.search_string.is_empty();
        let dir_items = app_state
            .current_dir
            .clone()
            .list_files(&app_state.file_tree_node_sorter)?;
        let dir_items: Vec<_> = dir_items
            .iter()
            .enumerate()
            .filter_map(|el| {
                // get the file name
                let file_name = match el.1.get_simple_name() {
                    Ok(simple_name) => simple_name.to_string_lossy().into_owned(),
                    Err(err) => {
                        app_state.set_err(err);
                        String::from("<Could not get file name>")
                    }
                };
                // if searching, skip irrelevant results
                let search_score = if is_searching {
                    let match_data = SkimMatcherV2::default().smart_case().fuzzy(
                        &file_name,
                        &app_state.search_string,
                        true,
                    );
                    match match_data {
                        None => return None,
                        // TODO: sort them as well
                        Some(match_data) => match_data.0,
                    }
                } else {
                    0
                };
                Some(DisplayListItem {
                    file_tree_node: el.1,
                    file_name,
                    search_score,
                })
            })
            .collect();

        let file_cursor_index = app_state
            .file_cursor
            .get_file_cursor_index(&mut dir_items.iter().map(|el| el.file_tree_node))
            .or_else(|| {
                // if the cursor can not be placed:
                app_state.is_urgent_update = true;
                app_state.file_cursor.selected_file = dir_items
                    // get the first index
                    .first()
                    .map_or(None, |el| {
                        Some(el.file_tree_node.get_path().as_os_str().to_owned())
                    });
                // if the directory is empty, skip it. Otherwise, go to the first element
                if app_state.file_cursor.selected_file.is_some() {
                    Some(0)
                } else {
                    None
                }
            });

        let height_of_list_available = chunks[0].height as usize - 2; // -2 because one line from each side is used for the border

        let num_to_skip = file_cursor_index.map_or(None, |index| {
            // how far is the index from its desired position?
            Some(
                // Do not do anything
                if app_state.max_distance_from_cursor_to_bottom + index < height_of_list_available {
                    0
                // if the viewport is full and the cursor is close to the bottom, but there are still concealed items later on in the list
                } else if dir_items.len() > index + app_state.max_distance_from_cursor_to_bottom {
                    index + app_state.max_distance_from_cursor_to_bottom - height_of_list_available
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
                let out = ListItem::new(el.file_name.clone());

                // apply styles

                // different styles depending on whether it is selected or not and whether it si a dir or not
                // It is only None if the directory is empty, which would make the code below not be executed. Unwrap is safe.
                let styles_set = if el_index == file_cursor_index.unwrap() {
                    app_state.file_cursor.get_styles()
                } else {
                    &app_state.default_style_set
                };
                let out = out.style(if el.file_tree_node.is_dir() {
                    styles_set.dir.clone()
                } else {
                    styles_set.file.clone()
                });

                Some(out)
            })
            .collect();

        let list = List::new(dir_items).block(block);

        f.render_widget(list, chunks[0]);

        let block = Block::default()
            // .title(app_state.opened_file.clone().unwrap_or("".to_owned()))
            .borders(Borders::ALL);
        f.render_widget(block, chunks[1]);
    }
    Ok(())
}
