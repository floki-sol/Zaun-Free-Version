use crate::{
    cli::{
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    }, constants::general::OperationIntensity, utils::misc::PercentileGroup
};

pub fn get_follow_intensity_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Low (slow follows)"),
            None,
            Some(OptionCallback::ChangePumpFollowIntensity(
                OperationIntensity::Low,
            )),
        ),
        PageOption::new(
            String::from("Medium (moderate follow speed)"),
            None,
            Some(OptionCallback::ChangePumpFollowIntensity(
                OperationIntensity::Medium,
            )),
        ),
        PageOption::new(
            String::from("High (fast follow speed)"),
            None,
            Some(OptionCallback::ChangePumpFollowIntensity(
                OperationIntensity::High,
            )),
        ),
        PageOption::new(
            String::from("Spam (what the label says)"),
            None,
            Some(OptionCallback::ChangePumpFollowIntensity(
                OperationIntensity::Spam,
            )),
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Follow Intensity")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select preferred intensity",
        )),
    ))
}
