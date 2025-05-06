use std::sync::Arc;

use crate::{
    cli::{
        info::InfoSegment,
        input::{InputPage, InputType},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::misc::WalletType,
};

pub fn get_burn_tokens_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Burn from Dev wallet."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("Enter Token address, Pump or Bullx link.")),
                    InfoSegment::Emphasized(String::from("-- Must be a valid Pump token")),
                    InfoSegment::Normal(String::from("")),
                    InfoSegment::Warning(String::from("Warning: Destructive Action.")),
                    InfoSegment::Warning(String::from(
                        "DO NOT CONTINUE IF YOU DO NOT WISH TO BURN TOKENS.",
                    )),
                    InfoSegment::Normal(String::from("Burnt tokens cannot be retrieved.")),
                ],
                Some("Burn Token Input".to_string()),
                Some(OptionCallback::OnDemandBurn((WalletType::DevWallet, String::from("")))),
                None,
                InputType::PubKey,
            ))),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Burn Tokens.")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
