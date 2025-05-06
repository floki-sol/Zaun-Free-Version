use crate::cli::{
    menu::{MenuHandler, MenuPage, Page},
    options::PageOption,
};

use super::video::get_video_settings_page;


pub fn get_metadata_settings_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Use video for token media."),
            Some(get_video_settings_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Metadata & Media")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
