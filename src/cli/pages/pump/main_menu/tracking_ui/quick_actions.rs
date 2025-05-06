use crate::cli::{
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    };

use super::sell_all_bonded_options::get_sell_all_bonded_options_page;

pub fn get_quick_actions_option_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Sell All (Pump)."),
            //Some(get_in_development_info_page(menu_handler)),
            None,
            Some(OptionCallback::QuickSellAllNormal),
        ),
        PageOption::new(
            String::from("Sell All (Bonded)."),
            Some(get_sell_all_bonded_options_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Burn Dev (ALL)."),
            None,
            Some(OptionCallback::BurnDevAll),
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Quick Actions")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select quick action",
        )),
    ))
}
