use crate::cli::{
    info::InfoSegment,
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

pub fn get_change_tracking_transfer_receiver_page(menu_handler: &mut MenuHandler) -> Page {
    Page::InputPage(InputPage::new(
        vec![
            InfoSegment::Normal(String::from("Enter new transfer recepient")),
            InfoSegment::Emphasized(String::from("-- Must be a valid solana address")),
        ],
        Some(String::from("Transfer receiver input")),
        Some(OptionCallback::ChangeTrackerTransferRecepient(
            String::from(""),
        )),
        None,
        InputType::General,
    ))
}
