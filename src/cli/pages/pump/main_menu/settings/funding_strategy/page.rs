use crate::{cli::{
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
}, utils::misc::FundingStrategy};

pub fn get_funding_strategy_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Pre-fund (fund wallets before bundling)."),
            None,
            Some(OptionCallback::ChangeFundingStrategy(FundingStrategy::PreFund)),
        ),
        PageOption::new(
            String::from("In-contract (fund wallets mid-bundle with a contract)."),
            None,
            Some(OptionCallback::ChangeFundingStrategy(FundingStrategy::InContract)),
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Change Funding Strategy")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
