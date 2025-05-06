use std::sync::Arc;

use solana_sdk::signature::Keypair;

use crate::{
    cli::{
        info::{get_in_development_info_page, get_not_available_info_page, InfoPage, InfoSegment},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    constants::general::LaunchMode,
};

use super::{cto_coin_input::get_cto_input_page, dev_buy::get_dev_buy_page};

pub fn get_launch_option_page(
    menu_handler: &mut MenuHandler,
    token: Arc<Keypair>,
    metadata_uri: String,
    name: String,
    symbol: String,
) -> Page {

    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Classic Launch (Block 0)."),
            Some(get_dev_buy_page(
                menu_handler,
                Arc::clone(&token),
                metadata_uri.clone(),
                name.clone(),
                symbol.clone(),
                LaunchMode::Classic,
            )),
            None,
        ),
        PageOption::new(
            String::from("Bundle Snipe Launch."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Mass Multi-Wallet Snipe Launch."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Staggered Launch."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Dev Only Launch."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("CTO (No Dev)."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Quick Launch Decoy/Fake Coin."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Return"),
            None,
            Some(OptionCallback::ReturnToMenu),
        ),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Launch Mode.")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
