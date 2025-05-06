use crate::cli::{
    info::InfoSegment,
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

use super::{
    debug_mode::get_debug_mode_config_page, rpc_health_check::get_rpc_health_check_page,
    zaun_account::page::get_zaun_account_option_page,
};

pub fn get_advanced_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Zaun account."),
            Some(get_zaun_account_option_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Rpc health check."),
            Some(get_rpc_health_check_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Debug mode."),
            Some(get_debug_mode_config_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Advanced config")),
        Some(String::from("[⇑⇓] keys to navigate and ⏎ to choose option")),
    ))
}
