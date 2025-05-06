use crate::cli::{
    info::{get_in_development_info_page, get_not_available_info_page},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

use super::{
    bundle::page::get_bundle_or_simulate_page, comments::page::get_comments_type_page,
    extras::page::get_extras_option_page, follow_bot::page::get_follow_mode_page,
    lookup_table_management::page::get_lookup_table_management_page,
    settings::page::get_settings_page, tracking_ui::page::get_tracking_ui_option_page,
    vanity_generation::page::get_vanity_generation_page,
    wallet_management::page::get_wallet_management_page,
};

pub fn get_pump_main_menu_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Bundle/Launch Setup."),
            Some(get_bundle_or_simulate_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Launch Tracker UI."),
            Some(get_tracking_ui_option_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Bump bot."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Comment bot."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Follow bot."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Wallet management."),
            Some(get_wallet_management_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Generate pump vanity addresses."),
            Some(get_vanity_generation_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Lookup table management."),
            Some(get_lookup_table_management_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Extras."),
            Some(get_extras_option_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Settings."),
            Some(get_settings_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Exit."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Zaun Version:[Free]")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
