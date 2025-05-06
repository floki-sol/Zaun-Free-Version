use crate::cli::{
    info::InfoSegment,
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

pub fn get_rpc_health_check_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(String::from("Enable RPC health check on app start."), None, Some(OptionCallback::ChangeRpcHealthCheckPreference(false))),
        PageOption::new(String::from("Skip RPC health check on app start."), None, Some(OptionCallback::ChangeRpcHealthCheckPreference(true))),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Rpc Check preference.")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to toggle check preference",
        )),
    ))
}
