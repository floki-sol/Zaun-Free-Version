use crate::{
    cli::{
        info::{get_in_development_info_page, get_not_available_info_page},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::comments_manager::CommentType,
};

use super::quick_actions::get_quick_actions_option_page;

pub fn get_tracking_ui_option_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Normal (Pump)."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Bonded (Pump Amm)."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Quick Actions."),
            //Some(get_in_development_info_page(menu_handler)),
            Some(get_quick_actions_option_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Tracking Option")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select tracking option",
        )),
    ))
}
