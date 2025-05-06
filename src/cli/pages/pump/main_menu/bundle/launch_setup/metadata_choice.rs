use std::sync::Arc;

use solana_sdk::signature::Keypair;

use crate::{
    cli::{
        info::InfoSegment,
        input::{InputPage, InputType},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::misc::WalletType,
};

use super::base58_ca_input::get_base58_ca_input_page;

pub fn get_metadata_configuration_page(
    menu_handler: &mut MenuHandler,
    token_keypair: Arc<Keypair>,
) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Upload metadata."),
            None,
            Some(OptionCallback::VerifyAndUploadMetadata(Arc::clone(
                &token_keypair,
            ))),
        ),
        PageOption::new(
            String::from("Clone token."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("Enter Token address, Pump or Bullx link.")),
                    InfoSegment::Emphasized(String::from("-- Must be a valid Pump token")),
                ],
                Some(String::from("Token details")),
                Some(OptionCallback::CloneTokenMetadata((
                    Arc::clone(&token_keypair),
                    String::new(),
                ))),
                None,
                InputType::General,
            ))),
            None,
        ),
        PageOption::new(
            String::from("Use pre-existing metadata Uri."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("Enter metadata uri to use for your launch.")),
                    InfoSegment::Emphasized(String::from(
                        "-- Uri must link to valid json metadata of this format:",
                    )),
                    InfoSegment::Normal(String::from("")),
                    InfoSegment::StringSplitInfo((
                        String::from("-- name"),
                        String::from("<your token name>"),
                    )),
                    InfoSegment::StringSplitInfo((
                        String::from("-- symbol"),
                        String::from("<your token symbol>"),
                    )),
                    InfoSegment::StringSplitInfo((
                        String::from("-- description"),
                        String::from("<optional description>"),
                    )),
                    InfoSegment::StringSplitInfo((
                        String::from("-- twitter"),
                        String::from("<optional twitter link>"),
                    )),
                    InfoSegment::StringSplitInfo((
                        String::from("-- telegram"),
                        String::from("<optional telegram link>"),
                    )),
                    InfoSegment::StringSplitInfo((
                        String::from("-- website"),
                        String::from("<optional website link>"),
                    )),
                    InfoSegment::StringSplitInfo((
                        String::from("-- showName"),
                        String::from("<always set to: 'true'>"),
                    )),
                    InfoSegment::StringSplitInfo((
                        String::from("-- image"),
                        String::from("<token image link>"),
                    )),
                ],
                Some(String::from("Metadata URI input")),
                Some(OptionCallback::VerifyMetadataLinkInput((
                    token_keypair,
                    String::new(),
                ))),
                None,
                InputType::General,
            ))),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Metadata Configuration.")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
