use crate::{
    cli::{
        info::InfoSegment,
        input::{InputPage, InputType},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::misc::WalletsFundingType,
};

pub fn get_fixed_funding_input_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<InfoSegment> = vec![
        InfoSegment::Normal(format!("Input amount to fund wallets.")),
        InfoSegment::Emphasized(String::from(
            "-- All configured wallets will be funded with the same amount.",
        )),
        InfoSegment::Emphasized(String::from(
            "-- Make sure to have enough funds for the operation.",
        )),
        InfoSegment::Emphasized(String::from("-- Min amount: 0.001.")),
        InfoSegment::Emphasized(String::from("-- Max amount: 100.")),
    ];

    Page::InputPage(InputPage::new(
        options,
        Some(String::from("Fixed Funding")),
        Some(OptionCallback::FundBundleWallets(
            WalletsFundingType::Static(String::from("")),
        )),
        None,
        InputType::DecimalNumber,
    ))
}
