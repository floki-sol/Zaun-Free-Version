use std::sync::Arc;

use solana_sdk::signature::Keypair;

use crate::{
    cli::{
        info::{get_in_development_info_page, InfoSegment},
        input::{InputPage, InputType},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    constants::general::LaunchMode,
};

use super::{cto_coin_input::get_cto_input_page, dev_buy::get_dev_buy_page};

pub fn get_bundle_snipe_flavor_option_page(
    menu_handler: &mut MenuHandler,
    token: Arc<Keypair>,
    metadata_uri: String,
    name: String,
    symbol: String,
    launch_mode: LaunchMode,
    dev_buy: u64,
    amounts: Vec<u64>,
) -> Page {
    let buy_bundle_delay_input_page = Page::InputPage(InputPage::new(
        vec![
            InfoSegment::Normal(String::from("Enter preferred delay in seconds for the buy bundle.")),
            InfoSegment::Emphasized(String::from(
                "-- buy bundle task will start executing regardless if the first (dev) bundle",
            )),
            InfoSegment::Emphasized(String::from(
                "-- has already landed or not after the specified delay.",
            )),
            InfoSegment::Emphasized(String::from(
                "-- Min Value: 0 (No Delay)",
            )),
            InfoSegment::Emphasized(String::from(
                "-- Max Value: 180 (three minutes)",
            )),
        ],
        Some(String::from("Buy Bundle Delay")),
        Some(OptionCallback::ValidateSplitBundleDelay((
            Arc::clone(&token),
            metadata_uri.clone(),
            (name.clone(), symbol.clone()),
            launch_mode.clone(),
            dev_buy.clone(),
            amounts.clone(),
            String::from("0"),
        ))),
        None,
        InputType::WholeNumber,
    ));

    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Use delay for buy bundle."),
            Some(buy_bundle_delay_input_page),
            None,
        ),
        PageOption::new(
            String::from("No Delay on buy bundle."),
            None,
            Some(OptionCallback::ValidateSplitBundleDelay((
                Arc::clone(&token),
                metadata_uri.clone(),
                (name.clone(), symbol.clone()),
                launch_mode.clone(),
                dev_buy.clone(),
                amounts.clone(),
                String::from("0"),
            ))),
        ),
        PageOption::new(
            String::from("Manually send buy bundle."),
            None,
            Some(OptionCallback::ValidateSplitBundleDelay((
                token,
                metadata_uri,
                (name, symbol),
                launch_mode,
                dev_buy,
                amounts,
                String::from("manual"),
            ))),
        ),
        PageOption::new(
            String::from("Return"),
            None,
            Some(OptionCallback::ReturnToMenu),
        ),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Bundle Snipe.")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
