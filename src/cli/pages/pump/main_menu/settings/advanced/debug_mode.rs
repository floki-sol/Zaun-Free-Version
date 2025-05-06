use crate::cli::{
    info::InfoSegment,
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

pub fn get_debug_mode_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(String::from("Enable debug mode"), None, Some(OptionCallback::ToggleDebugMode(true))),
        PageOption::new(String::from("Disable debug mode"), None, Some(OptionCallback::ToggleDebugMode(false))),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Debug mode config")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to toggle debug mode",
        )),
    ))
}
