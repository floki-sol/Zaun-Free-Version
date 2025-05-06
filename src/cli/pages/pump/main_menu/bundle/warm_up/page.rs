use std::sync::{atomic::AtomicBool, Arc};

use crate::cli::{
    info::{get_in_development_info_page, InfoPage, InfoSegment},
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

pub fn get_warm_up_option_page(menu_handler: &mut MenuHandler) -> Page {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let dust_info_page: InfoPage = InfoPage::new(
        vec![
            InfoSegment::Emphasized(String::from("All bundle wallets will buy dust coins.")),
            InfoSegment::Normal(String::from(
                "-- Buying will continue until funder wallet has no balance left.",
            )),
            InfoSegment::Warning(String::from(
                "-- Do not let this run indefinitly or the task will continue buying tokens.",
            )),
        ],
        Some(String::from("Dust coins")),
        Some(OptionCallback::StartDustCoinsTask(stop_flag)),
        None,
        None,
    );

    let trading_activity_info_page: InfoPage = InfoPage::new(
        vec![
            InfoSegment::Emphasized(String::from("All bundle wallets will begin trading newly created coins.")),
            InfoSegment::Normal(String::from(
                "-- trading will continue until funder wallet has no balance left.",
            )),
            InfoSegment::Warning(String::from(
                "-- Do not let this run indefinitly or the task will exhaust funder balance from fees.",
            )),
        ],
        Some(String::from("Trading Activity")),
        Some(OptionCallback::StartTradingActivityTask),
        None,
        None,
    );

    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Dust coins."),
            Some(Page::InfoPage(dust_info_page)),
            None,
        ),
        PageOption::new(
            String::from("Trading Activity."),
            Some(Page::InfoPage(trading_activity_info_page)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Wallet Warm Up")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
