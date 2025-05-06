use crate::cli::{
    info::{get_in_development_info_page, InfoSegment},
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

pub fn get_zaun_account_option_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("View subscription details."),
            None,
            Some(OptionCallback::ViewSubscriptionDetails),
        ),
        PageOption::new(
            String::from("Manage subscription."),
            Some(get_in_development_info_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("My Account")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to toggle select highlighted option",
        )),
    ))
}
