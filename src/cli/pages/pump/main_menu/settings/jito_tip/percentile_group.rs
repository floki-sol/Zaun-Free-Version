use crate::{
    cli::{
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::misc::PercentileGroup,
};

pub fn get_percentile_group_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("25th Percentile (very low tips | very low landing rates)."),
            None,
            Some(OptionCallback::ChangePercentile(PercentileGroup::P25)),
        ),
        PageOption::new(
            String::from("50th Percentile (low tips | low landing rates)."),
            None,
            Some(OptionCallback::ChangePercentile(PercentileGroup::P50)),
        ),
        PageOption::new(
            String::from("75th Percentile (default | decent landing rates)."),
            None,
            Some(OptionCallback::ChangePercentile(PercentileGroup::P75)),
        ),
        PageOption::new(
            String::from("95th Percentile (high tips | high landing rates)."),
            None,
            Some(OptionCallback::ChangePercentile(PercentileGroup::P95)),
        ),
        PageOption::new(
            String::from("99th Percentile (VERY high tips | Near guaranteed landing rates)."),
            None,
            Some(OptionCallback::ChangePercentile(PercentileGroup::P99)),
        ),

        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Change Percentile Group")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select percentile group",
        )),
    ))
}
