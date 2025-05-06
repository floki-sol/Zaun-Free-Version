use crate::{
    cli::{
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::comments_manager::CommentType,
};

pub fn get_follow_mode_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Standard mode"),
            None,
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Follow mode")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select comment type",
        )),
    ))
}
