use crate::{
    cli::{
        info::get_in_development_info_page, menu::{MenuHandler, MenuPage, Page}, options::{OptionCallback, PageOption}
    },
    utils::misc::WalletsFundingType,
};

use super::{
    distribution_input_page::get_distribution_funding_input_page, human_like_input_page::get_human_like_funding_input_page, min_max_input_page::{get_min_max_funding_input_page}, static_input_page::get_fixed_funding_input_page
};

pub fn get_fund_bundle_wallets_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Use fixed amount."),
            Some(get_fixed_funding_input_page(menu_handler)),
            None,
            //Some(OptionCallback::FundBundleWallets(
            //    WalletsFundingType::Static,
            //)),
        ),
        PageOption::new(
            String::from("Min-Max range."),
            Some(get_min_max_funding_input_page(menu_handler)),
            None,
            //Some(OptionCallback::FundBundleWallets(
            //    WalletsFundingType::MinMax,
            //)),
        ),
        PageOption::new(
            String::from("Distribute total amount randomly."),
            Some(get_distribution_funding_input_page(menu_handler)),
            None,
            //Some(OptionCallback::FundBundleWallets(
            //    WalletsFundingType::Distribution,
            //)),
        ),
        PageOption::new(
            String::from("Human-like split of total amount."),
            Some(get_human_like_funding_input_page(menu_handler)),
            None,
            //Some(OptionCallback::FundBundleWallets(
            //    WalletsFundingType::Distribution,
            //)),
        ),
        PageOption::new(
            String::from("Interactive mode."),
            Some(get_in_development_info_page(menu_handler)),
            None,
            //Some(OptionCallback::FundBundleWallets(
            //    WalletsFundingType::Interactive,
            //)),
        ),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Bundle Wallet Funding")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
