use crate::cli::{
    menu::{MenuHandler, MenuPage, Page},
    options::PageOption,
};

use super::coins_monitor_channel::get_channel_option_page;


pub fn get_extras_option_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("New coins monitor."),
            Some(get_channel_option_page(menu_handler, "new coins")),
            None,
        ),
        PageOption::new(
            String::from("King of the hill monitor."),
            Some(get_channel_option_page(menu_handler, "koth")),
            None,
        ),
        PageOption::new(
            String::from("Migration monitor."),
            Some(get_channel_option_page(menu_handler, "migration")),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Misc Utils")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select comment type",
        )),
    ))
}
