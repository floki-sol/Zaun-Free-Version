use crate::{
    cli::{
        info::get_in_development_info_page,
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::comments_manager::CommentType,
};

pub fn get_comments_type_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Bullish comments"),
            None,
            Some(OptionCallback::ValidateAndConfirmOnDemandCommentInput(
                CommentType::Bullish,
            )),
        ),
        PageOption::new(
            String::from("Bearish comments."),
            None,
            Some(OptionCallback::ValidateAndConfirmOnDemandCommentInput(
                CommentType::Bearish,
            )),
        ),
        PageOption::new(
            String::from("Custom comments."),
            Some(get_in_development_info_page(menu_handler)),
            None,
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Comment type")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select comment type",
        )),
    ))
}
