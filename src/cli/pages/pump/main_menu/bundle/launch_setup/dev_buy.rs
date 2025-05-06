use std::sync::Arc;

use solana_sdk::signature::Keypair;

use crate::{
    cli::{
        info::InfoSegment,
        input::{InputPage, InputType},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    constants::general::LaunchMode,
};

pub fn get_dev_buy_page(
    menu_handler: &mut MenuHandler,
    token: Arc<Keypair>,
    metadata_uri: String,
    name:String,
    symbol:String,
    launch_mode: LaunchMode,
) -> Page {
    let options: Vec<InfoSegment> = vec![
        InfoSegment::Normal(format!("Input the dev buy amount for your launch in sol")),
        InfoSegment::Emphasized(String::from("-- Dev must have at least 0.03 sol (excluded from buy amount) for tx and creation fees.")),
        InfoSegment::Emphasized(String::from("-- Dev must Also cover platform fee (1%) for the buy.")),
        InfoSegment::Emphasized(String::from("-- Setting amount as 0 means that dev won't buy any tokens.")),
        InfoSegment::Emphasized(String::from("")),
        InfoSegment::Emphasized(String::from("-- Min amount: 0.0")),
        InfoSegment::Emphasized(String::from("-- Max amount: 100")),
    ];

    Page::InputPage(InputPage::new(
        options,
        Some(String::from("Dev Buy")),
        Some(OptionCallback::ValidateDevBuy((
            token,
            metadata_uri,
            (name, symbol),
            launch_mode,
            String::from(""),
        ))),
        None,
        InputType::DecimalNumber,
    ))
}
