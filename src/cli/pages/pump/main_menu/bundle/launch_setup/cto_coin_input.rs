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

pub fn get_cto_input_page(
    menu_handler: &mut MenuHandler,
    token: Arc<Keypair>,
    metadata_uri: String,
    name: String,
    symbol: String,
    launch_mode: LaunchMode,
) -> Page {
    let options: Vec<InfoSegment> = vec![
        InfoSegment::Normal(format!(
            "Input Solana address for the coin you want to CTO:"
        )),
        InfoSegment::Emphasized(String::from("-- Must be a valid solana Address.")),
        InfoSegment::Emphasized(String::from("-- Must not be bonded.")),
    ];

    Page::InputPage(InputPage::new(
        options,
        Some(String::from("CTO Coin Address")),
        Some(OptionCallback::ValidateCTOInput((
            token,
            metadata_uri,
            (name, symbol),
            launch_mode,
            String::from(""),
        ))),
        None,
        InputType::PubKey,
    ))
}
