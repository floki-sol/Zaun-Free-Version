use crate::cli::{
    info::InfoSegment,
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

use super::percentile_group::get_percentile_group_config_page;

pub fn get_max_tip_input_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<InfoSegment> = vec![
        InfoSegment::Normal(format!("Input your preferred jito max tip:")),
        InfoSegment::Emphasized(String::from("-- Max value: 5")),
        InfoSegment::Emphasized(String::from("-- Min value: 0.00001")),
    ];

    Page::InputPage(InputPage::new(
        options,
        Some(String::from("Jito Max Tip")),
        Some(OptionCallback::ChangeMaxTip(String::from(""))),
        None,
        InputType::DecimalNumber,
    ))
}
