use crate::{
    cli::{
        info::InfoSegment,
        input::{InputPage, InputType},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::misc::WalletsFundingType,
};

pub fn get_distribution_funding_input_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<InfoSegment> = vec![
        InfoSegment::Normal(format!("Input total amount to fund wallets.")),
        InfoSegment::Emphasized(String::from(
            "-- Amount will be split across wallets randomly.",
        )),
        InfoSegment::Emphasized(String::from(
            "-- Make sure to have enough funds for the operation.",
        )),
        InfoSegment::Emphasized(String::from(
            "-- Min amount: 0.001 * number of wallets configured.",
        )),
        InfoSegment::Emphasized(String::from(
            "-- Max amount: 100 * number of wallets configured.",
        )),
    ];

    Page::InputPage(InputPage::new(
        options,
        Some(String::from("Human-Like Distribution Funding")),
        Some(OptionCallback::FundBundleWallets(
            WalletsFundingType::Distribution(String::from("")),
        )),
        None,
        InputType::DecimalNumber,
    ))
}
