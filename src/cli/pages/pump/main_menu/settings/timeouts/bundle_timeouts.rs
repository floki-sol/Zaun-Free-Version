use crate::cli::{
    info::InfoSegment,
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};


pub fn get_bundle_timeouts_input_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<InfoSegment> = vec![
        InfoSegment::Normal(format!("Input your preferred Bundle timeout in seconds:")),
        InfoSegment::Emphasized(String::from("-- Max value: 180")),
        InfoSegment::Emphasized(String::from("-- Min value: 10")),
    ];

    Page::InputPage(InputPage::new(
        options,
        Some(String::from("Bundle Timeout")),
        Some(OptionCallback::ChangeBundleTimeout(String::from(""))),
        None,
        InputType::DecimalNumber,
    ))
}
