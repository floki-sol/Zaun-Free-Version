use crate::cli::{
    menu::{MenuHandler, MenuPage, Page},
    options::PageOption,
};

use super::comment_intensity::get_comment_intensity_config_page;


pub fn get_comments_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Change Pump comment intensity."),
            Some(get_comment_intensity_config_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Comments Config")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
