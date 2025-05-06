use crate::cli::{
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

use super::base58_ca_input::get_base58_ca_input_page;

pub fn get_ca_choosing_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Generate random CA for your launch."),
            None,
            Some(OptionCallback::GenerateCa),
        ),
        PageOption::new(
            String::from("Use configured keypair file."),
            None,
            Some(OptionCallback::ValidateCaFile),
        ),
        PageOption::new(
            String::from("Input vanity Base58 keypair for your launch."),
            Some(get_base58_ca_input_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("CA Configuration.")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
