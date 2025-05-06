use crate::{
    cli::{
        info::InfoSegment,
        input::{InputPage, InputType},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::misc::WalletType,
};

use super::page::get_wallet_cleanup_page;

pub fn get_wallet_cleanup_receiver_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Funding wallet."),
            Some(get_wallet_cleanup_page(
                menu_handler,
                WalletType::FundingWallet,
            )),
            None,
        ),
        PageOption::new(
            String::from("Another wallet."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("Enter receiver wallet address.")),
                    InfoSegment::Emphasized(String::from(
                        "-- Must be a valid solana wallet address",
                    )),
                ],
                Some(String::from("Receiver Input")),
                Some(OptionCallback::ReceiverInputCallback(String::new())),
                None,
                InputType::General,
            ))),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Clean-up Recipeint")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
