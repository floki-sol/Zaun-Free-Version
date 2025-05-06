use crate::cli::{
    info::InfoSegment,
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};


pub fn get_advanced_tracker_config_page(menu_handler: &mut MenuHandler) -> Page {
    //need to create two input pages here

    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Change sell all target market-cap"),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("Enter new target marketcap (USD)")),
                    InfoSegment::Emphasized(String::from("-- Min value: $3000")),
                ],
                Some(String::from("Marketcap input")),
                Some(OptionCallback::ChangeTrackerTargetMarketCap(String::from(
                    "",
                ))),
                None,
                InputType::WholeNumber,
            ))),
            None,
        ),
        PageOption::new(
            String::from("Change delay-sell timer"),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("Enter new delay timer (Seconds)")),
                    InfoSegment::Emphasized(String::from("-- Max value: 180")),
                    InfoSegment::Emphasized(String::from("-- Min value: 3")),
                ],
                Some(String::from("Delay input")),
                Some(OptionCallback::ChangeTrackerDelaySell(String::from(""))),
                None,
                InputType::WholeNumber,
            ))),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Advanced tracking config")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
