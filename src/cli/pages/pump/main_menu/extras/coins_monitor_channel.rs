use crate::cli::{
    info::{get_in_development_info_page, InfoSegment},
    input::{InputPage, InputType},
    menu::{MenuHandler, MenuPage, Page},
    options::{OptionCallback, PageOption},
};

pub fn get_channel_option_page(menu_handler: &mut MenuHandler, operation_type: &str) -> Page {
    
    let discord_callback = match operation_type {
        "new coins" => OptionCallback::StartNewCoinsMonitor(String::new(), String::from("discord")),
        "koth" => OptionCallback::StartKothMonitor(String::new(), String::from("discord")),
        "migration" => OptionCallback::StartMigrationMonitor(String::new(), String::from("discord")),
        _ => OptionCallback::StartNewCoinsMonitor(String::new(), String::from("discord")),
    };

    let discord_input_page = Page::InputPage(InputPage::new(
        vec![
            InfoSegment::Emphasized(String::from("Input your discord webhook.")),
            InfoSegment::Normal(String::from(
                "-- Make sure its a valid discord webhook url.",
            )),
        ],
        Some(String::from("Webhook Input")),
        Some(discord_callback),
        None,
        InputType::General,
    ));

    let telegram_callback = match operation_type {
        "new coins" => {
            OptionCallback::StartNewCoinsMonitor(String::new(), String::from("telegram"))
        }
        "koth" => OptionCallback::StartNewCoinsMonitor(String::new(), String::from("telegram")),
        "migration" => {
            OptionCallback::StartNewCoinsMonitor(String::new(), String::from("telegram"))
        }
        _ => OptionCallback::StartNewCoinsMonitor(String::new(), String::from("telegram")),
    };

    let telegram_input_page = get_in_development_info_page(menu_handler);

    let options: Vec<PageOption> = vec![
        PageOption::new(String::from("Discord."), Some(discord_input_page), None),
        PageOption::new(String::from("Telegram."), Some(telegram_input_page), None),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Channel Option")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select comment type",
        )),
    ))
}
