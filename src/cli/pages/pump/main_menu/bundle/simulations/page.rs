use crate::cli::{
    info::{get_in_development_info_page, InfoSegment}, input::{InputPage, InputType}, menu::{MenuHandler, MenuPage, Page}, options::{OptionCallback, PageOption}
};

pub fn get_simulations_option_page(menu_handler: &mut MenuHandler) -> Page {
    
    
    let dev_buy_segments: Vec<InfoSegment> = vec![
        InfoSegment::Normal(format!("Input the dev buy amount for your launch in sol")),
        InfoSegment::Emphasized(String::from("-- Dev must have at least 0.03 sol (excluded from buy amount) for tx and creation fees.")),
        InfoSegment::Emphasized(String::from("-- Dev must Also cover platform fee (1%) for the buy.")),
        InfoSegment::Emphasized(String::from("-- Setting amount as 0 means that dev won't buy any tokens.")),
        InfoSegment::Emphasized(String::from("")),
        InfoSegment::Emphasized(String::from("-- Min amount: 0.0.")),
        InfoSegment::Emphasized(String::from("-- Max amount: 100.")),
    ];

    let dev_buy_page = Page::InputPage(InputPage::new(
        dev_buy_segments,
        Some(String::from("Dev Buy")),
        Some(OptionCallback::SimulateHolderDistributions(String::from(""))),
        None,
        InputType::DecimalNumber,
    ));
    
    
    let options: Vec<PageOption> = vec![
        PageOption::new(String::from("Preview holder distributions."), Some(dev_buy_page), None),
        PageOption::new(String::from("Simulate classic bundle."), Some(get_in_development_info_page(menu_handler)), None),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Simulations")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
