use artem::{config::ResizingDimension, ConfigBuilder};
use base64::decode;
use borsh::BorshDeserialize;
use cli::{
    error::{display_error_page, spawn_error_input_listener},
    menu::{render, MenuHandler},
    pages::pump::main_menu::page::get_pump_main_menu_page,
};
use colored::*;

use constants::general::BOT_PROGRAM_ID;
use crossterm::{
    event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    style::SetBackgroundColor,
    terminal::{self, SetTitle},
    ExecutableCommand,
};
use futures::{AsyncBufReadExt, SinkExt, StreamExt};
use jito::{bundles::BundleSenderBalancer, tip::run_jito_fee_websocket};
//use jito_protos::*;
use loaders::global_config_loader::{load_global_config, GlobalConfig};
use log::{info, LevelFilter};
use serde_json::Value;
use solana_account_decoder::UiDataSliceConfig;
//use searcher::{searcher_service_client::SearcherServiceClient, NextScheduledLeaderRequest};
use solana_client::{
    rpc_config::{
        RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcSendTransactionConfig,
        RpcTransactionConfig,
    },
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{
    address_lookup_table::AddressLookupTableAccount,
    commitment_config::CommitmentConfig,
    compute_budget,
    hash::Hash,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_program,
    transaction::VersionedTransaction,
};
use solana_transaction_status::{UiInstruction, UiParsedInstruction};
use std::{
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use std::{
    collections::HashMap,
    fs,
    io::{self, Read, Write},
    num::NonZeroU32,
    path::Path,
    process::Stdio,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, BufReader},
    process::Command,
    sync::{Mutex, Notify, RwLock, Semaphore},
    time::sleep,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tonic::{
    client::Grpc,
    transport::{Channel, Endpoint},
};
use tui::{
    layout::{Layout, Rect},
    style::{Color as TuiColor, Style},
    widgets::{Block, Borders},
};
use utils::{
    blockhash_manager::{run_blockhash_updater, RecentBlockhashManager},
    bonding_curve_provider::{run_curve_provider, BondingCurve, BondingCurveProvider},
    bundle_factory::get_create_bundle_guard_bundle,
    instructions::get_extend_lookup_table_ix,
    misc::{
        adjust_file_path, extract_lamports, get_account_subscription_message, graceful_shutdown,
    },
    pdas::get_bundle_guard,
    pump_helpers::{
        derive_all_pump_dex_keys, derive_all_pump_keys, fetch_pump_token_general_data, login, register,
    },
};

mod cli;
mod constants;
mod jito;
mod loaders;
mod utils;

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() {
    //load environment variables
    let env_config = match loaders::env_loader::EnvConfig::load_env() {
        Ok(conf) => conf,
        Err(e) => {
            eprintln!("{}", format!("Error loading env config: {}", e).red());
            sleep(Duration::from_secs(5)).await;
            return;
        }
    };
    let connection = env_config.get_rpc_client();
    let wss_url = env_config.get_rpc_wss_url();
    let subscription_keypair = env_config.get_subscription_keypair();
    let funder_keypair = env_config.get_funding_keypair();
    let dev_keypair = env_config.get_dev_keypair();
    let capsolver_api_key = Arc::new(String::new());
    drop(env_config);
    println!("{}", "Loaded env config".magenta());

    //loading global configurations
    let global_config_validation = load_global_config();
    if let Err(e) = &global_config_validation {
        eprintln!("{}", format!("Error loading global config: {}", e).red());
        sleep(Duration::from_secs(5)).await;
        return;
    }
    let global_config: Arc<RwLock<Option<GlobalConfig>>> =
        Arc::new(RwLock::new(global_config_validation.ok()));
    println!("{}", "Loaded global configurations".magenta());

    //check for rpc and wss health check
    let global_config_lock = global_config.read().await;
    let mut should_check = true;
    if let Some(config) = global_config_lock.as_ref() {
        should_check = !config.skip_rpc_health_check;
    }
    drop(global_config_lock);

    if should_check {
        println!("{}", "Checking RPC health".magenta());
        let https_check = connection.get_balance(&system_program::ID);
        let wss_connection = connect_async(wss_url.as_ref()).await;

        if let Err(e) = https_check {
            eprintln!(
                "{}",
                format!("Https endpoint health check failed: {}", e).red()
            );
            sleep(Duration::from_secs(5)).await;
            return;
        } else if let Ok(res) = https_check {
            println!("{}", "Https endpoint operational ".green());
        };
        if let Err(e) = wss_connection {
            eprintln!(
                "{}",
                format!("Websocket connection health check failed: {}", e).red()
            );
            sleep(Duration::from_secs(5)).await;
            return;
        } else if let Ok(res) = wss_connection {
            println!("{}", "Wss endpoint operational ".green());
        };

        sleep(Duration::from_millis(500)).await;
    }

    //initialize the blockhash manager
    let rpc_ref = Arc::clone(&connection);
    let blockhash_manager = Arc::new(RecentBlockhashManager::new(rpc_ref, Hash::default()));
    let blockhash_manager_ref = Arc::clone(&blockhash_manager);
    let initial_blockhash_manager_ref = Arc::clone(&blockhash_manager);

    tokio::spawn(async move {
        initial_blockhash_manager_ref.start();
        sleep(Duration::from_secs(1)).await;
        initial_blockhash_manager_ref.stop();
    });

    let blockhash_manager_update_ref = Arc::clone(&blockhash_manager);
    tokio::spawn(async move {
        run_blockhash_updater(blockhash_manager_update_ref).await;
    });
    println!("{}", "Initialized blockhash manager".magenta());

    //check for essential program accounts and create_them_if necessary
    let bundle_guard = get_bundle_guard(
        &funder_keypair.pubkey(),
        &Pubkey::from_str(BOT_PROGRAM_ID).unwrap(),
    );

    let connection_ref_1 = Arc::clone(&connection);
    let bundle_guard_handle = thread::spawn(move || connection_ref_1.get_account(&bundle_guard));

    let (bundle_guard_account_data, _) = (
        bundle_guard_handle
            .join()
            .unwrap()
            .map_err(|e| e.to_string()),
        (),
    );

    //then check for the bundle guard and create it if needed
    if let Err(data) = bundle_guard_account_data {
        let mut input = String::new();
        println!();
        println!(
            "{}",
            "New Funder wallet detected. Need to create essential program accounts. (y/n): "
                .magenta()
        );
        println!("{}", "1. Bundle Guard".magenta());
        io::stdout().flush();
        io::stdin().read_line(&mut input);

        if input.trim().to_lowercase().starts_with('y') {
            println!("{}", "Creating accounts...".yellow());
        } else {
            std::process::exit(0);
        }

        input.clear();
        println!(
            "{}",
            "Continue with operation? (~0.001 SOL).  (y/n):".blue()
        );
        io::stdout().flush();
        io::stdin().read_line(&mut input);

        if input.trim().to_lowercase().starts_with('y') {
            println!("{}", "Creating bundle guard...".yellow());

            let bundle_balancer = Arc::new(BundleSenderBalancer::new());
            let stop_flag = Arc::new(AtomicBool::new(false));
            // Task for sending bundles
            let bundle_sender_flag = Arc::clone(&stop_flag);
            let bundle_balancer_ref = Arc::clone(&bundle_balancer);
            let funder_keypair_ref = Arc::clone(&funder_keypair);
            let notify_websocket_opened = Arc::new(Notify::new());
            let notify_websocket_opened_ref = Arc::clone(&notify_websocket_opened);
            tokio::spawn(async move {
                // Wait for WebSocket connection to be established
                notify_websocket_opened_ref.notified().await;
                //println!("Starting bundle-spamming task...");
                //check for essential program accounts
                let bundle_guard = get_bundle_guard(
                    &funder_keypair_ref.pubkey(),
                    &Pubkey::from_str(BOT_PROGRAM_ID).unwrap(),
                );

                let semaphore: Arc<Semaphore> = Arc::new(Semaphore::new(100));

                while !bundle_sender_flag.load(Ordering::Relaxed) {
                    let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();

                    let bundle_balancer_ref = Arc::clone(&bundle_balancer_ref);
                    let funder_keypair_ref = Arc::clone(&funder_keypair_ref);
                    let bundle = get_create_bundle_guard_bundle(
                        Arc::clone(&funder_keypair_ref),
                        Arc::clone(&blockhash_manager_ref),
                        bundle_guard,
                        (0.0001 * LAMPORTS_PER_SOL as f64) as u64,
                    );
                    tokio::spawn(async move {
                        let _ = bundle_balancer_ref.send_bundle(bundle).await;
                        drop(permit);
                    });

                    sleep(Duration::from_millis(50)).await;
                }
            });

            let account_monitor_flag = Arc::clone(&stop_flag);
            let connection_ref = Arc::clone(&connection);
            let funder_ref = Arc::clone(&funder_keypair);
            let notify_websocket_opened_ref = Arc::clone(&notify_websocket_opened);

            // Send the subscription request

            let (ws_stream, _) = connect_async(wss_url.as_ref()).await.unwrap();
            let (mut write, mut read) = ws_stream.split();

            let subscription_message = get_account_subscription_message(&bundle_guard);

            write
                .send(Message::Text(subscription_message.to_string()))
                .await
                .unwrap();
            notify_websocket_opened_ref.notify_one();

            // Listen for messages
            while !account_monitor_flag.load(Ordering::Relaxed) {
                if let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Some(notification_lamports) = extract_lamports(&text) {
                                account_monitor_flag.store(true, Ordering::Relaxed);
                            }
                        }
                        Ok(Message::Close(_)) => {
                            println!("WebSocket closed");
                            break;
                        }
                        Err(e) => {
                            eprintln!("WebSocket error: {:?}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            }

            println!();
            println!("{}", "Initial setup complete. please restart".blue());
            std::process::exit(0);
        } else {
            std::process::exit(0);
        }
    }

    //creating menu handler and initializing it with main page
    let menu_handler: Arc<Mutex<MenuHandler>> =
        Arc::new(Mutex::new(MenuHandler::new().await.unwrap()));
    let mut handler = menu_handler.lock().await;
    handler.initialize();
    drop(handler);

    //run jito tip provider thread
    let tip_stream_config_ref = Arc::clone(&global_config);
    let jito_fee: Arc<AtomicU64> = Arc::new(AtomicU64::new(100000));
    let jito_fee_clone = Arc::clone(&jito_fee);
    tokio::spawn(async move {
        loop {
            run_jito_fee_websocket(Arc::clone(&jito_fee_clone), tip_stream_config_ref.clone())
                .await;
            //println!("Jito fee WebSocket disconnected. Reconnecting in 3 seconds...");
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    });
    println!("{}", "Started tip stream listener".magenta());

    loop {
        let _ = render(
            Arc::clone(&menu_handler),
            Arc::clone(&connection),
            Arc::clone(&blockhash_manager),
            Arc::clone(&dev_keypair),
            Arc::clone(&funder_keypair),
            Arc::clone(&subscription_keypair),
            Arc::clone(&global_config),
            Arc::clone(&jito_fee),
            Arc::clone(&capsolver_api_key),
            Arc::clone(&wss_url),
        )
        .await;
    }
}
