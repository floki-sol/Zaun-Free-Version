use crate::{cli::{
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
}, utils::{comments_manager::CommentType, misc::FundingStrategy}};

pub fn get_change_tracking_comment_type_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Bullish comments"),
            None,
            Some(OptionCallback::ChangeTrackerCommentType(CommentType::Bullish)),
        ),
        PageOption::new(
            String::from("Bearish comments"),
            None,
            Some(OptionCallback::ChangeTrackerCommentType(CommentType::Bearish)),
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Change Tracker comments")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}