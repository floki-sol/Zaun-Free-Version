use std::{
    future::Future,
    pin::Pin,
    str::FromStr,
    sync::{atomic::AtomicU64, Arc},
    time::{SystemTime, UNIX_EPOCH},
};
//use strip_ansi_escapes::strip;

use borsh::BorshDeserialize;
use chrono::{DateTime, Local, Utc};
use crossterm::terminal::{self, LeaveAlternateScreen};
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer,
};
use tokio::sync::{Mutex, RwLock};

use crate::{
    cli::{
        error::{display_error_page, spawn_error_input_listener},
        info::{display_info_page, InfoSegment},
        loading_indicator::display_loading_page,
        menu::MenuHandler,
        options::OptionCallback,
    },
    jito::tip::fetch_recent_tip_percentiles,
    loaders::global_config_loader::{
        load_global_config, save_global_config, GlobalConfig, JitoSplitBundlePercentages,
    },
    utils::{
        blockhash_manager::RecentBlockhashManager,
        misc::{
            is_valid_decimal_string, is_valid_small_integer_string, split_tip, FundingStrategy,
            PercentileGroup,
        },
    },
};

pub async fn invoke_settings_callback(
    menu_handler: Arc<Mutex<MenuHandler>>,
    connection: Arc<RpcClient>,
    wss_url: Arc<String>,
    _blockhash_manager: Arc<RecentBlockhashManager>,
    callback: OptionCallback,
    _dev_wallet: Arc<Keypair>,
    funding_wallet: Arc<Keypair>,
    subscription_wallet: Arc<Keypair>,
    global_config: Arc<RwLock<Option<GlobalConfig>>>,
    _current_tip: Arc<AtomicU64>,
    _capsolver_api_key: Arc<String>,
) {
    let mut loading_handler_ref = menu_handler.lock().await;
    let menu_handler_clone1 = Arc::clone(&menu_handler);

    //let menu_handler_clone1 = Arc::clone(&menu_handler);
    let menu_handler_clone2 = Arc::clone(&menu_handler);
    //let menu_handler_clone3 = Arc::clone(&menu_handler);

    match callback {
        //settings
        OptionCallback::ChangeFundingStrategy(strategy) => {
            let mut curr_config = load_global_config().unwrap();

            match strategy {
                FundingStrategy::PreFund => curr_config.funding_strategy = "pre-fund".to_string(),
                FundingStrategy::InContract => {
                    curr_config.funding_strategy = "in-contract".to_string()
                }
            };

            //save config
            let _ = save_global_config(&curr_config);
            let mut global_config_lock = global_config.write().await; // Get the write lock
            let funding_strategy: String = curr_config.funding_strategy.clone();
            if let Some(config_ref) = global_config_lock.as_mut() {
                *config_ref = curr_config; // Assign `curr_config` to the global config
            }

            //loading_handler_ref.to_previous_page();
            loading_handler_ref.to_previous_page();
            display_info_page(
                vec![InfoSegment::Normal(format!(
                    "Updated Funding Strategy to: {}",
                    funding_strategy
                ))],
                String::from("Success."),
                &mut loading_handler_ref,
                None,
                None,
                None,
            ); //stdout_task.await;
        }
        OptionCallback::ChangePumpCommentIntensity(new_intensity) => {
            ////
        }
        OptionCallback::ChangePercentile(percentile) => {
            let mut curr_config = load_global_config().unwrap();

            match percentile {
                PercentileGroup::P25 => {
                    curr_config.jito_tip_stream_percentile = 25;
                }
                PercentileGroup::P50 => {
                    curr_config.jito_tip_stream_percentile = 50;
                }
                PercentileGroup::P75 => {
                    curr_config.jito_tip_stream_percentile = 75;
                }
                PercentileGroup::P95 => {
                    curr_config.jito_tip_stream_percentile = 95;
                }
                PercentileGroup::P99 => {
                    curr_config.jito_tip_stream_percentile = 99;
                }
            };

            //save config
            let _ = save_global_config(&curr_config);
            let mut global_config_lock = global_config.write().await; // Get the write lock
            let mut new_percentile: u32 = curr_config.jito_tip_stream_percentile;
            if let Some(config_ref) = global_config_lock.as_mut() {
                *config_ref = curr_config; // Assign `curr_config` to the global config
            }

            loading_handler_ref.to_previous_page();
            loading_handler_ref.to_previous_page();
            display_info_page(
                vec![InfoSegment::Normal(format!(
                    "Updated jito tip percentile to {}th percentile group",
                    new_percentile
                ))],
                String::from("Success."),
                &mut loading_handler_ref,
                None,
                None,
                None,
            ); //stdout_task.await;
        }
        OptionCallback::ChangeMaxTip(data) => {
            // Attempt to parse the string into an integer
            let is_valid = is_valid_decimal_string(data, 0.00001, 5.0);

            if let Err(error) = is_valid {
                //loading_handler_ref.to_previous_page();
                display_error_page(
                    error,
                    Some(String::from("Jito Tip Config Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );

                spawn_error_input_listener(menu_handler_clone2, 5);
            } else if let Ok(val) = is_valid {
                let mut curr_config = load_global_config().unwrap();
                curr_config.jito_max_tip = val;

                //save config
                let _ = save_global_config(&curr_config);
                let mut global_config_lock = global_config.write().await; // Get the write lock
                if let Some(config_ref) = global_config_lock.as_mut() {
                    *config_ref = curr_config; // Assign `curr_config` to the global config
                }

                loading_handler_ref.to_previous_page();
                loading_handler_ref.to_previous_page();
                display_info_page(
                    vec![InfoSegment::Normal(format!("Updated max tip."))],
                    String::from("Success."),
                    &mut loading_handler_ref,
                    None,
                    None,
                    None,
                ); //stdout_task.await;
            }
        }
        OptionCallback::ChangeSplitBundleTipPercentages((dev_percentage, buy_percentage)) => {
            let mut curr_config = load_global_config().unwrap();
            curr_config
                .jito_tip_split_bundle_percentages
                .dev_bundle_tip_percentage = dev_percentage;
            curr_config
                .jito_tip_split_bundle_percentages
                .buy_bundle_tip_percentage = buy_percentage;

            //save config
            let _ = save_global_config(&curr_config);
            let mut global_config_lock = global_config.write().await; // Get the write lock
            if let Some(config_ref) = global_config_lock.as_mut() {
                *config_ref = curr_config; // Assign `curr_config` to the global config
            }

            loading_handler_ref.to_previous_page();
            loading_handler_ref.to_previous_page();
            display_info_page(
                vec![InfoSegment::Normal(format!(
                    "Updated split bundle percentages."
                ))],
                String::from("Success."),
                &mut loading_handler_ref,
                None,
                None,
                None,
            ); //stdout_task.await;
        }

        OptionCallback::ViewJitoTip => {
            //first of all lock the global config to get the percentile group and max tip.
            let global_config_lock = global_config.read().await; // Get the write lock
                                                                 //let mut timeout: u8 = 20;
            let mut percentile_group: u8 = 25;
            let mut max_tip: f64 = 0.0;
            let mut split_bundle_percentages = JitoSplitBundlePercentages {
                dev_bundle_tip_percentage: 0.2,
                buy_bundle_tip_percentage: 0.8,
            };
            if let Some(config_ref) = global_config_lock.as_ref() {
                percentile_group = config_ref.jito_tip_stream_percentile as u8;
                max_tip = config_ref.jito_max_tip;
                split_bundle_percentages = config_ref.jito_tip_split_bundle_percentages.clone();
            };
            //drop the global config lock
            drop(global_config_lock);

            display_loading_page(String::from("Fetching tip info"), &mut loading_handler_ref);
            drop(loading_handler_ref);
            tokio::spawn(async move {
                let recent_tip_percentiles = fetch_recent_tip_percentiles().await;

                let mut handler_ref = menu_handler_clone1.lock().await;

                if let Ok(recent_tip_percentiles) = recent_tip_percentiles {
                    //here we calculate

                    let header = "Tip Info";

                    let mut info_segments: Vec<InfoSegment> = vec![InfoSegment::Emphasized(
                        String::from("Recent Tip percentile info: "),
                    )];

                    let percentile_array = recent_tip_percentiles.get_percentile_array();

                    let mut curr_configured_tip = max_tip;
                    let choosen_percentile = match percentile_group {
                        25 => recent_tip_percentiles._25th,
                        50 => recent_tip_percentiles._50th,
                        75 => recent_tip_percentiles._75th,
                        95 => recent_tip_percentiles._95th,
                        99 => recent_tip_percentiles._99th,
                        _ => recent_tip_percentiles._75th,
                    };
                    curr_configured_tip = max_tip.min(choosen_percentile);
                    let tip_splits = split_tip(
                        split_bundle_percentages.dev_bundle_tip_percentage,
                        split_bundle_percentages.buy_bundle_tip_percentage,
                        (curr_configured_tip * LAMPORTS_PER_SOL as f64) as u64,
                    );

                    //let tip_splits = split_tip(split_bundle_percentages.dev_bundle_tip_percentage, split_bundle_percentages.buy_bundle_tip_percentage, curr_configured_tip)

                    for percentile in percentile_array {
                        let new_segment = InfoSegment::StringSplitInfo((
                            format!("{}th percentile group", percentile.0),
                            format!(
                                "{:.8} Sol {}",
                                percentile.1,
                                if percentile.0 == percentile_group {
                                    " <="
                                } else {
                                    ""
                                }
                            ),
                        ));
                        info_segments.push(new_segment);
                    }

                    info_segments.push(InfoSegment::Normal(format!("")));
                    info_segments.push(InfoSegment::NumericSplitInfo((
                        String::from("Configured max cap"),
                        format!("{:.6} Sol", max_tip),
                    )));
                    info_segments.push(InfoSegment::Normal(format!("")));
                    info_segments.push(InfoSegment::NumericSplitInfo((
                        String::from("Your configured jito tip"),
                        format!("{:.6} Sol", curr_configured_tip),
                    )));

                    info_segments.push(InfoSegment::Normal(String::from("")));
                    info_segments.push(InfoSegment::Emphasized(String::from("Tip splits:")));
                    info_segments.push(InfoSegment::StringSplitInfo((
                        String::from("-- Dev bundle tip"),
                        format!("{:.6} Sol", tip_splits.0 as f64 / LAMPORTS_PER_SOL as f64),
                    )));
                    info_segments.push(InfoSegment::StringSplitInfo((
                        String::from("-- Buy bundle(s) tip"),
                        format!("{:.6} Sol", tip_splits.1 as f64 / LAMPORTS_PER_SOL as f64),
                    )));

                    info_segments.push(InfoSegment::Normal(String::from("")));

                    if curr_configured_tip >= max_tip {
                        let segment = InfoSegment::Emphasized(String::from(
                            "Jito tip is set to your configured max cap.",
                        ));
                        info_segments.push(segment);
                    } else {
                        let segment = InfoSegment::Emphasized(String::from(
                            "Jito tip is less than your configured max cap.",
                        ));
                        info_segments.push(segment);
                    };

                    handler_ref.to_previous_page();
                    handler_ref.to_previous_page();

                    display_info_page(
                        info_segments,
                        header.to_string(),
                        &mut handler_ref,
                        None,
                        None,
                        None,
                    ); //stdout_task.await;
                       //display the info page containing the tip info
                } else if let Err(e) = recent_tip_percentiles {
                    handler_ref.to_previous_page();
                    handler_ref.to_previous_page();
                    display_error_page(
                        e,
                        Some(String::from("Tip percentile fetch error")),
                        None,
                        Some(10),
                        &mut handler_ref,
                        false, //menu_handler_clone2
                    );
                    spawn_error_input_listener(menu_handler_clone2, 10);
                }
            });

            //fetch the current tip percentiles
        }
        OptionCallback::ChangeBundleTimeout(data) => {
            let is_valid = is_valid_small_integer_string(data, 10, 180);

            if let Err(error) = is_valid {
                //loading_handler_ref.to_previous_page();
                display_error_page(
                    error,
                    Some(String::from("Bundle Timeout Config Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );

                spawn_error_input_listener(menu_handler_clone2, 5);
            } else if let Ok(val) = is_valid {
                let mut curr_config = load_global_config().unwrap();
                curr_config.bundle_timeout = val;

                //save config
                let _ = save_global_config(&curr_config);
                let mut global_config_lock = global_config.write().await; // Get the write lock
                if let Some(config_ref) = global_config_lock.as_mut() {
                    *config_ref = curr_config; // Assign `curr_config` to the global config
                }

                loading_handler_ref.to_previous_page();
                loading_handler_ref.to_previous_page();
                display_info_page(
                    vec![InfoSegment::Normal(format!("Updated bundle timeout."))],
                    String::from("Success."),
                    &mut loading_handler_ref,
                    None,
                    None,
                    None,
                ); //stdout_task.await;
            }
        }
        OptionCallback::ChangeUseVideo(data) => {
            let mut curr_config = load_global_config().unwrap();

            //save config
            curr_config.use_video = data;
            let _ = save_global_config(&curr_config);
            let mut global_config_lock = global_config.write().await; // Get the write lock
                                                                      //let funding_strategy: String = curr_config.funding_strategy.clone();
            if let Some(config_ref) = global_config_lock.as_mut() {
                *config_ref = curr_config; // Assign `curr_config` to the global config
            }

            //loading_handler_ref.to_previous_page();
            loading_handler_ref.to_previous_page();
            display_info_page(
                vec![InfoSegment::Normal(format!("Updated Metadata Config",))],
                String::from("Success."),
                &mut loading_handler_ref,
                None,
                None,
                None,
            ); //stdout_task.await;
        }
        OptionCallback::ChangeBumpAmount(data) => {
            ////
        }
        OptionCallback::ChangeBumpDelay(data) => {
            ////
        }
        OptionCallback::ChangeWalletsToBump(data) => {
            ////
        }
        OptionCallback::ChangeBumpFunder(data) => {
            ////
        }
        OptionCallback::ConfigureBumpOptionalProfile(data) => {
            ////
        }
        OptionCallback::RemoveOptionalProfile => {
            ////
        }
        OptionCallback::ChangeFollowerProfile(data) => {
            ////
        }
        OptionCallback::ChangePumpFollowIntensity(data) => {
            ////
        }
        OptionCallback::ToggleDebugMode(flavor) => {
            ////
        }
        OptionCallback::ChangeTrackerTransferRecepient(new_address) => {
            ////
        }
        OptionCallback::ChangeTrackerCommentType(new_comment_type) => {
            ////
        }
        OptionCallback::ChangeTrackerTargetMarketCap(new_marketcap) => {
            ////
        }
        OptionCallback::ChangeTrackerDelaySell(new_delay_timer) => {
            ////
        }
        OptionCallback::ChangeTrackerTradeFeedMinSolValue(data) => {
            ////
        }
        OptionCallback::ChangeRpcHealthCheckPreference(flavor) => {
            ////
        }
        OptionCallback::ViewSubscriptionDetails => {
            ////
        }

        _ => {}
    }
}
