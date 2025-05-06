use crate::{
    cli::{
        info::{get_not_available_info_page, InfoSegment},
        input::{InputPage, InputType},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::misc::WalletType,
};

pub fn get_wallet_cleanup_page(menu_handler: &mut MenuHandler, receiver: WalletType) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Retrieve All Sol from Dev wallet."),
            None,
            Some(OptionCallback::CleanUpSol(
                WalletType::DevWallet,
                receiver.clone(),
            )),
        ),
        PageOption::new(
            String::from("Retrieve All Tokens from dev wallet."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("Enter Token address, Pump or Bullx link.")),
                    InfoSegment::Emphasized(String::from("-- Must be a valid Pump token")),
                ],
                Some("Transfer Tokens Input".to_string()),
                Some(OptionCallback::CleanUpTokens(
                    String::from(""),
                    WalletType::DevWallet,
                    receiver.clone(),
                )),
                None,
                InputType::PubKey,
            ))),
            None,
        ),
        PageOption::new(
            String::from("Retrieve All Sol from Bumper wallet."),
            Some(get_not_available_info_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Retrieve All Sol from Bundle wallets."),
            None,
            Some(OptionCallback::CleanUpSol(
                WalletType::BundleWalletSol,
                receiver.clone(),
            )),
        ),
        PageOption::new(
            String::from("Retrieve All Tokens from Bundle wallets."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("Enter Token address, Pump or Bullx link.")),
                    InfoSegment::Emphasized(String::from("-- Must be a valid Pump token")),
                ],
                Some("Transfer Tokens Input".to_string()),
                Some(OptionCallback::CleanUpTokens(
                    String::from(""),
                    WalletType::BundleWalletTokens,
                    receiver.clone(),
                )),
                None,
                InputType::PubKey,
            ))),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Clean-up")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
