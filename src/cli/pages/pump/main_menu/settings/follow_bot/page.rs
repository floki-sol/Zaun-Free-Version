use crate::cli::{
    info::InfoSegment,
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

use super::follow_intensity::get_follow_intensity_config_page;

pub fn get_follow_bot_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Change follow Intensity."),
            Some(get_follow_intensity_config_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Change follow profile."),
            Some(Page::InputPage(InputPage::new(
                vec![
                    InfoSegment::Normal(String::from("Enter new follower profile keypair")),
                    InfoSegment::Emphasized(String::from("-- Must be a valid base58 keypair")),
                ],
                Some(String::from("follower profile input")),
                Some(OptionCallback::ChangeFollowerProfile(String::from(""))),
                None,
                InputType::General,
            ))),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Follow bot config")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
