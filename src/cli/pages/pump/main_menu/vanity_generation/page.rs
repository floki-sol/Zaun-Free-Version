use crate::cli::{
    info::{InfoPage, InfoSegment}, menu::{MenuHandler, MenuPage, Page}, options::{OptionCallback, PageOption}
};




pub fn get_grind_info_page(menu_handler: &mut MenuHandler) -> Page {
    
    
    let info_page: InfoPage = InfoPage::new(
        vec![
            //InfoSegment::StringSplitInfo((
            //    String::from("Address"),
            //    dev_wallet.pubkey().to_string(),
            //)),
            //InfoSegment::StringSplitInfo((
            //    String::from("Sol Balance"),
            //    sol_balance_str,
            //)),

            InfoSegment::Emphasized(String::from("Requirements:")),
            InfoSegment::Normal(String::from("-- Solana Cli Installed.")),
            InfoSegment::Normal(String::from("-- Sufficient Hardware (8 core CPU min).")),
            InfoSegment::Normal(String::from("")),
            InfoSegment::Emphasized(String::from("Warning:")),
            InfoSegment::Warning(String::from("-- Intensive task, Do not run for long if you")),
            InfoSegment::Warning(String::from("-- have slow hardware.")),
        ],
        Some(String::from("Grind")),
        Some(OptionCallback::GrindVanityCallBack),
        None,
        None,

    );

    Page::InfoPage(info_page)
}

pub fn get_vanity_generation_page(menu_handler: &mut MenuHandler) -> Page {
    
    
    let grind_info_page = get_grind_info_page(menu_handler);
    let options: Vec<PageOption> = vec![
        //PageOption::new(String::from("Fetch new addresses (PUMP API)."), None, None),
        PageOption::new(String::from("Grind new addresses (HARDWARE)."), Some(grind_info_page), None),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Vanity Generation")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
