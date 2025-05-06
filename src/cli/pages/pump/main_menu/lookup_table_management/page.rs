use crate::{cli::{
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
}, constants::general::LutCallback};

pub fn get_lookup_table_management_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Deactivate Most recently created lut."),
            None,
            Some(OptionCallback::ManageLut(LutCallback::Deactivate)),
        ),
        PageOption::new(
            String::from("Close Most recently created lut."),
            None,
            Some(OptionCallback::ManageLut(LutCallback::Close)),
        ),
        //PageOption::new(
        //    String::from("Wallet management."),
        //    Some(get_wallet_management_page(menu_handler)),
        //    None,
        //),
        //PageOption::new(
        //    String::from("Generate pump vanity addresses."),
        //    Some(get_vanity_generation_page(menu_handler)),
        //    None,
        //),
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
