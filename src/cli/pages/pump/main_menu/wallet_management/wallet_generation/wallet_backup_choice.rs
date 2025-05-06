use std::path::PathBuf;

use crate::{
    cli::{
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::backups::format_backup_filename,
};

pub fn get_choose_backup_page(menu_handler: &mut MenuHandler, backups: Vec<PathBuf>) -> Page {
    let mut options: Vec<PageOption> = backups
        .iter()
        .take(10)
        .map(|path_buf| {
            PageOption::new(
                format_backup_filename(&path_buf),
                None,
                Some(OptionCallback::ConfirmRetrieveBackup(path_buf.clone())),
            )
        })
        .collect();

    options.push(PageOption::new(String::from("Return."), None, None));

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Choose backup")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
