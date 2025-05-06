use crate::{
    cli::{
        info::get_in_development_info_page,
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::comments_manager::CommentType,
};

pub fn get_sell_all_bonded_options_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Insta sell."),
            None,
            Some(OptionCallback::QuickSellAllBondedInsta),
        ),
        PageOption::new(
            String::from("Wait for migration."),
            None,
            Some(OptionCallback::QuickSellAllBondedAwaited),
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Sell All Bonded Strategy")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select quick action",
        )),
    ))
}
