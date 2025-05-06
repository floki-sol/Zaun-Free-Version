use crate::{
    cli::{
        menu::{MenuHandler, MenuPage, Page},
        options::{OptionCallback, PageOption},
    },
    utils::misc::PercentileGroup,
};

pub fn get_split_bundle_tip_percentages_config_page(menu_handler: &mut MenuHandler) -> Page {
    let options: Vec<PageOption> = vec![
        PageOption::new(
            String::from("Dev Bundle: 10% | Buy Bundle: 90%"),
            None,
            Some(OptionCallback::ChangeSplitBundleTipPercentages((0.1, 0.9))),
        ),
        PageOption::new(
            String::from("Dev Bundle: 20% | Buy Bundle: 80%"),
            None,
            Some(OptionCallback::ChangeSplitBundleTipPercentages((0.2, 0.8))),
        ),
        PageOption::new(
            String::from("Dev Bundle: 30% | Buy Bundle: 70%"),
            None,
            Some(OptionCallback::ChangeSplitBundleTipPercentages((0.3, 0.7))),
        ),
        PageOption::new(
            String::from("Dev Bundle: 40% | Buy Bundle: 60%"),
            None,
            Some(OptionCallback::ChangeSplitBundleTipPercentages((0.4, 0.6))),
        ),
        PageOption::new(
            String::from("Dev Bundle: 50% | Buy Bundle: 50%"),
            None,
            Some(OptionCallback::ChangeSplitBundleTipPercentages((0.5, 0.5))),
        ),
        PageOption::new(
            String::from("Dev Bundle: 60% | Buy Bundle: 40%"),
            None,
            Some(OptionCallback::ChangeSplitBundleTipPercentages((0.6, 0.4))),
        ),
        PageOption::new(
            String::from("Dev Bundle: 70% | Buy Bundle: 30%"),
            None,
            Some(OptionCallback::ChangeSplitBundleTipPercentages((0.7, 0.3))),
        ),
        PageOption::new(
            String::from("Dev Bundle: 80% | Buy Bundle: 20%"),
            None,
            Some(OptionCallback::ChangeSplitBundleTipPercentages((0.8, 0.2))),
        ),
        PageOption::new(
            String::from("Dev Bundle: 90% | Buy Bundle: 10%"),
            None,
            Some(OptionCallback::ChangeSplitBundleTipPercentages((0.9, 0.1))),
        ),
        PageOption::new(String::from("Return."), None, None),
    ];
    Page::MenuPage(MenuPage::new(
        options,
        Some(String::from("Split Bundle Percentages Config")),
        Some(String::from(
            "[⇑⇓] keys to navigate and ⏎ to select percentile group",
        )),
    ))
}
