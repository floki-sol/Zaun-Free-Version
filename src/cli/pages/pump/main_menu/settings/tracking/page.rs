use crate::cli::{
    info::InfoSegment,
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

use super::{
    advanced::get_advanced_tracker_config_page, comment_type::get_change_tracking_comment_type_config_page, min_trade_feed_sol_input::get_change_trade_feed_min_sol_value_input_page, transfer_receiver::get_change_tracking_transfer_receiver_page
};

pub fn get_tracker_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Change transfer receiver"),
            Some(get_change_tracking_transfer_receiver_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Change comments type"),
            Some(get_change_tracking_comment_type_config_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Change trade feed min Sol value."),
            Some(get_change_trade_feed_min_sol_value_input_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Advanced"),
            Some(get_advanced_tracker_config_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Tracking config")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
