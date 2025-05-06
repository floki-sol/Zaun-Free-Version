use crate::cli::{
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

pub fn get_video_settings_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Yes."),
            None,
            Some(OptionCallback::ChangeUseVideo(true)),
        ),
        PageOption::new(
            String::from("No."),
            None,
            Some(OptionCallback::ChangeUseVideo(false)),
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Metadata & Media")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select highlighted option",
        )),
    ))
}
