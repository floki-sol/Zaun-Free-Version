use crate::{
    cli::{
        info::InfoSegment,
        input::{InputPage, InputType},
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::misc::WalletType,
};

pub fn get_fund_single_wallet_input_page(
    menu_handler: &mut MenuHandler,
    wallet_type: WalletType,
) -> Page {
    let options: Vec<InfoSegment> = vec![
        InfoSegment::Normal(format!("Input funding amount in sol:")),
        InfoSegment::Emphasized(String::from("-- Max value: 100")),
        InfoSegment::Emphasized(String::from("-- Min value: 0.001")),
    ];

    let header_label = match wallet_type {
        WalletType::DevWallet => String::from("Dev Wallet"),
        WalletType::BumpWallet => String::from("Bump Wallet"),
        _ => String::from("Dev Wallet"),
    };

    Page::InputPage(InputPage::new(
        options,
        Some(format!("Fund {} Amount", header_label)),
        Some(OptionCallback::FundSingleWallet((
            wallet_type,
            String::from(""),
        ))),
        None,
        InputType::DecimalNumber,
    ))
}
