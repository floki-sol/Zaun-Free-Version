use crate::cli::{
    info::InfoSegment,
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};


pub fn get_base58_ca_input_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<InfoSegment> = vec![
        InfoSegment::Normal(format!("Input Base58 private key for your preferred CA:")),
        InfoSegment::Emphasized(String::from("-- Must be a valid solana private key.")),
        InfoSegment::Emphasized(String::from("-- Must be in base58 format.")),
        InfoSegment::Emphasized(String::from("-- Must be a fresh key (has not been used and/or has no data/Sol).")),

    ];

    Page::InputPage(InputPage::new(
        options,
        Some(String::from("Base58 CA")),
        Some(OptionCallback::ValidateBase58Ca(String::from(""))),
        None,
        InputType::PubKey,
    ))
}
