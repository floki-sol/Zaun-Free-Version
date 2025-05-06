use crate::cli::{
    info::InfoSegment,
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

pub fn get_bump_bot_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Change Bump amount."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("")),
                    InfoSegment::Normal(String::from("Enter desired bump amount in sol")),
                    InfoSegment::Emphasized(String::from("-- Min: 0.005 ")),
                    InfoSegment::Emphasized(String::from("-- Max: 0.1 ")),
                ],
                Some(String::from("Bump Amount")),
                Some(OptionCallback::ChangeBumpAmount(String::from(""))),
                None,
                InputType::DecimalNumber,
            ))),
            None,
        ),
        PageOption::new(
            String::from("Change Bump Delay."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("")),
                    InfoSegment::Normal(String::from("Enter desired bump delay in seconds")),
                    InfoSegment::Emphasized(String::from("-- Min: 1")),
                    InfoSegment::Emphasized(String::from("-- Max: 30")),
                ],
                Some(String::from("Bump Delay")),
                Some(OptionCallback::ChangeBumpDelay(String::from(""))),
                None,
                InputType::WholeNumber,
            ))),
            None,
        ),
        PageOption::new(
            String::from("Change Bump wallet amount."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("")),
                    InfoSegment::Normal(String::from("Enter desired wallet amount to bump with")),
                    InfoSegment::Emphasized(String::from("-- Min: 1")),
                    InfoSegment::Emphasized(String::from("-- Max: 50")),
                ],
                Some(String::from("Bump wallet amount")),
                Some(OptionCallback::ChangeWalletsToBump(String::from(""))),
                None,
                InputType::WholeNumber,
            ))),
            None,
        ),
        PageOption::new(
            String::from("Change Bump funder."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("")),
                    InfoSegment::Normal(String::from("Enter new bump funder keypair")),
                    InfoSegment::Emphasized(String::from("-- Must be a valid base58 keypair")),
                    InfoSegment::Emphasized(String::from("-- Cannot be same as funder keypair")),
                ],
                Some(String::from("bump funder input")),
                Some(OptionCallback::ChangeBumpFunder(String::from(""))),
                None,
                InputType::General,
            ))),
            None,
        ),
        PageOption::new(
            String::from("Configure Optional profile."),
            Some(Page::MenuPage(MenuPage::new(
                vec![
                    PageOption::new(
                        String::from("Configure profile"),
                        Some(Page::InputPage(InputPage::new(
                            vec![
                                InfoSegment::Normal(String::from("")),
                                InfoSegment::Normal(String::from("Enter Optional profile keypair")),
                                InfoSegment::Emphasized(String::from(
                                    "-- Must be a valid base58 keypair",
                                )),
                                InfoSegment::Emphasized(String::from(
                                    "-- Wallet amount will be automatically set to 1",
                                )),
                            ],
                            Some(String::from("Optional profile input")),
                            Some(OptionCallback::ConfigureBumpOptionalProfile(String::from(
                                "",
                            ))),
                            None,
                            InputType::General,
                        ))),
                        None,
                    ),
                    PageOption::new(
                        String::from("Remove profile"),
                        None,
                        Some(OptionCallback::RemoveOptionalProfile),
                    ),
                ],
                Some(String::from("Optional Profile Config")),
                Some(String::from(
                    "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
                )),
            ))),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Bump bot config")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
