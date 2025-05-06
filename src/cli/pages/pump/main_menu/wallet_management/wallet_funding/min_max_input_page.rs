use crate::{
    cli::{
        info::InfoSegment,
        input::{InputPage, InputType},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::misc::WalletsFundingType,
};

pub fn get_min_max_funding_input_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<InfoSegment> = vec![
        InfoSegment::Normal(format!("Input Min and Max amount to fund wallets.")),
        InfoSegment::Emphasized(String::from(
            "-- Amount will be randomly assigned to each wallet in the range.",
        )),
        InfoSegment::Emphasized(String::from(
            "-- Make sure to have enough funds for the operation.",
        )),
        InfoSegment::Emphasized(String::from("-- Min amount: 0.001.")),
        InfoSegment::Emphasized(String::from("-- Max amount: 100")),
        InfoSegment::Emphasized(String::from("-- input format: <min_amount>-<max_amount> ")),
        InfoSegment::Emphasized(String::from("-- Examples: 1-5, 0.001-0.7, 10-100")),
    ];

    Page::InputPage(InputPage::new(
        options,
        Some(String::from("Min-Max Funding")),
        Some(OptionCallback::FundBundleWallets(
            WalletsFundingType::MinMax(String::from("")),
        )),
        None,
        InputType::General,
    ))
}
