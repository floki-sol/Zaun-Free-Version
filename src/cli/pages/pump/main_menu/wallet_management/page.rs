use crate::cli::{
    menu::{MenuHandler, MenuPage, Page},
    options::PageOption,
};

use super::{
    balance_checker::page::get_check_balance_page, burn_tokens::page::get_burn_tokens_page, wallet_cleanup::receiver::get_wallet_cleanup_receiver_page, wallet_funding::page::get_wallet_funding_page, wallet_generation::page::get_wallet_generation_page
};

pub fn get_wallet_management_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Balance checker."),
            Some(get_check_balance_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Wallet funding."),
            Some(get_wallet_funding_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Wallet generation."),
            Some(get_wallet_generation_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Wallet clean-up."),
            Some(get_wallet_cleanup_receiver_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Burn tokens."),
            Some(get_burn_tokens_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Wallet Management")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
