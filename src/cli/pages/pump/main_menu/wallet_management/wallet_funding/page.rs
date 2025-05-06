use crate::{
    cli::{
        info::get_not_available_info_page, menu::{MenuHandler, MenuPage, Page}, options::{OptionCallback, PageOption}
    },
    utils::misc::WalletType,
};

use super::{
    fund_bundle_wallets::get_fund_bundle_wallets_page, fund_single_wallet::get_fund_single_wallet_input_page,
};

pub fn get_wallet_funding_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Fund Dev wallet."),
            Some(get_fund_single_wallet_input_page(
                menu_handler,
                WalletType::DevWallet,
            )),
            None,
        ),
        PageOption::new(
            String::from("Fund Bump wallet."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Fund Bundle wallets."),
            Some(get_fund_bundle_wallets_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Wallet Funding")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
