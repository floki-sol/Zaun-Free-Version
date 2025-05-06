use crate::cli::{
    info::{get_in_development_info_page, get_not_available_info_page}, menu::{MenuHandler, MenuPage, Page}, options::PageOption, pages::pump::main_menu::bundle::launch_setup::metadata_choice::get_metadata_configuration_page
};

use super::{
    advanced::page::get_advanced_config_page, bump_bot::page::get_bump_bot_config_page,
    comments::page::get_comments_config_page, follow_bot::page::get_follow_bot_config_page,
    funding_strategy::page::get_funding_strategy_config_page,
    jito_tip::page::get_jito_tip_config_page, metadata::page::get_metadata_settings_page,
    timeouts::page::get_timeouts_page, tracking::page::get_tracker_config_page,
};

pub fn get_settings_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Funding Strategy."),
            Some(get_funding_strategy_config_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Jito Fee Configurations."),
            Some(get_jito_tip_config_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Timeouts."),
            Some(get_timeouts_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Metadata."),
            Some(get_metadata_settings_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Bump bot."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Comments."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Follow bot."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Tracking."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Advanced."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Settings")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
