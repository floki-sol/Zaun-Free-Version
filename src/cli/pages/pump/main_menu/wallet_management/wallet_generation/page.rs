use crate::cli::{
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

pub fn get_wallet_generation_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Generate new bundler wallets."),
            None,
            Some(OptionCallback::BundlerWalletGenerationMenuCallBack),
        ),
        PageOption::new(
            String::from("Retrieve from backups."),
            None,
            Some(OptionCallback::RetrieveWalletsFromBackupsCallback),
        ),
        PageOption::new(String::from("Return."), None, None),
    ];

    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Wallet Generation")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
