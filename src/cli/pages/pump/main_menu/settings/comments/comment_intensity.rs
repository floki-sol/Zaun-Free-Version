use crate::{
    cli::{
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    }, constants::general::OperationIntensity, utils::misc::PercentileGroup
};

pub fn get_comment_intensity_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Low (slow comments)"),
            None,
            Some(OptionCallback::ChangePumpCommentIntensity(OperationIntensity::Low)),
        ),
        PageOption::new(
            String::from("Medium (moderate commenting speed)"),
            None,
            Some(OptionCallback::ChangePumpCommentIntensity(OperationIntensity::Medium)),
        ),
        PageOption::new(
            String::from("High (fast commenting speed)"),
            None,
            Some(OptionCallback::ChangePumpCommentIntensity(OperationIntensity::High)),
        ),
        PageOption::new(
            String::from("Spam (what the label says)"),
            None,
            Some(OptionCallback::ChangePumpCommentIntensity(OperationIntensity::Spam)),
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Comment Intensity")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select preferred intensity",
        )),
    ))
}
