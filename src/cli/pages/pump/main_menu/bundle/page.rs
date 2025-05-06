use crate::cli::{
    info::get_not_available_info_page, menu::{MenuHandler, MenuPage, Page}, options::{OptionCallback, PageOption}
};

use super::{
    launch_setup::CA::get_ca_choosing_page, simulations::page::get_simulations_option_page,
    warm_up::page::get_warm_up_option_page,
};

pub fn get_bundle_or_simulate_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Setup and bundle."),
            Some(get_ca_choosing_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Simulate launch Stats."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Wallet warm up."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Bundle/Launch Setup")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
