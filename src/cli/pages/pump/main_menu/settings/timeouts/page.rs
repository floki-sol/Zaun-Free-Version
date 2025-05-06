use crate::cli::{
    menu::{MenuHandler, MenuPage, Page},
    options::PageOption,
};

use super::bundle_timeouts::get_bundle_timeouts_input_page;


pub fn get_timeouts_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Bundle timeouts."),
            Some(get_bundle_timeouts_input_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Timeouts")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
