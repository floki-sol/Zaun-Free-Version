use crate::cli::{
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

use super::{max_tip::get_max_tip_input_page, percentile_group::get_percentile_group_config_page, split_bundle_percentages::get_split_bundle_tip_percentages_config_page};

pub fn get_jito_tip_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Change tip stream percentile group."),
            Some(get_percentile_group_config_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Adjust jito tip max cap."),
            Some(get_max_tip_input_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("Change jito tip split bundle percentages."),
            Some(get_split_bundle_tip_percentages_config_page(menu_handler)),
            None,
        ),
        PageOption::new(
            String::from("View configured jito tip."),
            None,
            Some(OptionCallback::ViewJitoTip),
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Jito tip config")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
