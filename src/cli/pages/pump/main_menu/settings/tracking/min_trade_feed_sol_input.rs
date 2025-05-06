use crate::cli::{
    info::InfoSegment,
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

pub fn get_change_trade_feed_min_sol_value_input_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<InfoSegment> = vec![
        InfoSegment::Normal(format!("Input your new trade feed min sol amount filter")),
        InfoSegment::Emphasized(String::from("-- Max value: 5")),
        InfoSegment::Emphasized(String::from("-- Min value: 0.001")),
    ];

    Page::InputPage(InputPage::new(
        options,
        Some(String::from("Trade Feed Config")),
        Some(OptionCallback::ChangeTrackerTradeFeedMinSolValue(
            String::from(""),
        )),
        None,
        InputType::DecimalNumber,
    ))
}
