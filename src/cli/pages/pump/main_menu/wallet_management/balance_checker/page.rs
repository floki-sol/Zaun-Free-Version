use std::sync::Arc;

use crate::{
    cli::{
        info::{get_not_available_info_page, InfoSegment},
        input::{InputPage, InputType},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::misc::WalletType,
};

pub fn get_check_balance_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Funding wallet Sol balance."),
            None,
            Some(OptionCallback::BalanceCheckerCallback((
                WalletType::FundingWallet,
                String::new(),
            ))),
        ),
        PageOption::new(
            String::from("Dev wallet Sol balance."),
            None,
            Some(OptionCallback::BalanceCheckerCallback((
                WalletType::DevWallet,
                String::new(),
            ))),
        ),
        PageOption::new(
            String::from("Bump wallet Sol balance."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("External wallet Sol balance."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("Enter External wallet address")),
                    InfoSegment::Emphasized(String::from("-- Must be a valid base58 Publickey")),
                ],
                Some(String::from("External Address input")),
                Some(OptionCallback::BalanceCheckerCallback((
                    WalletType::Another(String::from("")),
                    String::new(),
                ))),
                None,
                InputType::General,
            ))),
            None,
        ),
        PageOption::new(
            String::from("Bundle wallets Sol balances."),
            None,
            Some(OptionCallback::BalanceCheckerCallback((
                WalletType::BundleWalletSol,
                String::new(),
            ))),
        ),
        PageOption::new(
            String::from("Bundle wallets token balances."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("Enter Token address, Pump or Bullx link.")),
                    InfoSegment::Emphasized(String::from("-- Must be a valid Pump token")),
                ],
                Some(String::from("Token details")),
                Some(OptionCallback::BalanceCheckerCallback((
                    WalletType::BundleWalletTokens,
                    String::new(),
                ))),
                None,
                InputType::General,
            ))),
            None,
            //Some(OptionCallback::BalanceCheckerCallback(
            //    WalletType::BundleWalletTokens,
            //)),
        ),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Check Balances.")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
