use std::{
    process::Stdio,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
//use strip_ansi_escapes::strip;

use base64::decode;
use borsh::BorshDeserialize;
use futures::{SinkExt, StreamExt};
use log::info;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{
    address_lookup_table::state::{AddressLookupTable, LookupTableStatus},
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    slot_hashes::{self, SlotHashes},
    sysvar::slot_hashes::SlotHashesSysvar,
};
use solana_transaction_status::{option_serializer::OptionSerializer, UiInstruction, UiLoadedAddresses, UiParsedInstruction};
use tokio::{
    io::{AsyncReadExt, BufReader},
    process::Command,
    sync::{Mutex, Notify, RwLock, Semaphore},
    time::sleep,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    cli::{
        error::{
            display_bundle_guard_fetch_error_page, display_bundle_timeout_error_page,
            display_error_page, spawn_error_input_listener,
        },
        info::{display_info_page, InfoSegment},
        loading_indicator::display_loading_page,
        menu::MenuHandler,
        options::OptionCallback,
    },
    constants::general::{BundleGuard, CreateEvent, LutCallback, AMM_V4, BOT_PROGRAM_ID, MINT_AUTH, PUMP_AMM_ADDRESS, PUMP_MIGRATION_AUTHORITY, PUMP_PROGRAM_ID},
    jito::bundles::BundleSenderBalancer,
    loaders::global_config_loader::GlobalConfig,
    utils::{
        backups::{load_most_recent_lut, remove_most_recent_lut}, blockhash_manager::RecentBlockhashManager, bundle_factory::get_redeem_lookup_table_bundle, instructions, misc::{
            extract_lamports, fix_ipfs_url, get_account_subscription_message, get_transaction_logs_subscription_message, process_vanity_file, send_create_event_embed, send_new_koth_event_embed, send_new_migration_event, spawn_bundle_timeout_task, validate_discord_webhook_url
        }, pdas::get_bundle_guard, pump_helpers::{fetch_latest_koth, fetch_pump_token_general_data}
    },
};

pub async fn invoke_misc_callback(
    menu_handler: Arc<Mutex<MenuHandler>>,
    connection: Arc<RpcClient>,
    wss_url: Arc<String>,
    blockhash_manager: Arc<RecentBlockhashManager>,
    callback: OptionCallback,
    _dev_wallet: Arc<Keypair>,
    funding_wallet: Arc<Keypair>,
    global_config: Arc<RwLock<Option<GlobalConfig>>>,
    current_tip: Arc<AtomicU64>,
    _capsolver_api_key: Arc<String>,
) {
    //info!("{:?}", current_tip);

    let mut loading_handler_ref = menu_handler.lock().await;

    let menu_handler_clone1 = Arc::clone(&menu_handler);
    let menu_handler_clone2 = Arc::clone(&menu_handler);
    let _menu_handler_clone3 = Arc::clone(&menu_handler);

    match callback {
        //misc callbacks
        OptionCallback::ReturnToMenu => {
            loading_handler_ref.return_to_pump_main_menu();
        }

        OptionCallback::DoNothing => {
            //literally do nothing
        }
        
        OptionCallback::GrindVanityCallBack => {
            //first of all completely clear the terminal

            // Check if `solana-keygen` is installed

            // First check if solana-keygen exists
            let solana_installed =
                tokio::task::spawn_blocking(|| which::which("solana-keygen").is_ok())
                    .await
                    .unwrap_or(false);

            if !solana_installed {
                loading_handler_ref.to_previous_page();
                display_error_page(
                    String::from("Solana cli is not installed"),
                    Some(String::from("Vanity Generation Error")),
                    Some(vec![
                        String::from("install here: "),
                        String::from("https://docs.solanalabs.com/cli/install"),
                    ]),
                    Some(10),
                    &mut loading_handler_ref,
                    false,
                );

                spawn_error_input_listener(menu_handler_clone1, 10);
                return;
            }
            // Only spawn the process if we confirmed solana-keygen exists
            let child_guard: Arc<Mutex<tokio::process::Child>> = Arc::new(Mutex::new(
                Command::new("solana-keygen")
                    .arg("grind")
                    .arg("--ends-with")
                    .arg("pump:10000000")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap(),
            ));

            let mut child = child_guard.lock().await;
            let stdout = child.stdout.take().expect("Failed to capture stdout");
            let mut stdout_reader = BufReader::new(stdout);
            let mut stdout_line = vec![0; 256]; // Example buffer size

            let child_guard_clone = Arc::clone(&child_guard);

            // Event listener for stdout
            tokio::spawn(async move {
                loop {
                    //println!("Grind task running");
                    sleep(Duration::from_millis(100)).await;
                    let mut child = child_guard_clone.lock().await;

                    match child.try_wait() {
                        Ok(Some(status)) => {
                            // Child has finished, handle the exit status
                            //println!("Child process exited with status: {:?}", status);
                            break; // Exit the loop, as the child is done
                        }
                        _ => {}
                    }

                    // Check if the child has finished
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            // Child has finished, handle the exit status
                            info!("Child process exited with status: {:?}", status);
                            break; // Exit the loop, as the child is done
                        }
                        Ok(None) => {
                            // Child is still running, continue reading from stdout
                        }
                        Err(e) => {
                            info!("Error checking child process status: {:?}", e);
                            break;
                        }
                    }

                    let bytes_read = stdout_reader.read(&mut stdout_line).await.unwrap();
                    if bytes_read == 0 {
                        break; // No more data
                    }
                    let output = String::from_utf8_lossy(&stdout_line[..bytes_read]).to_string();

                    if output.contains("Wrote keypair to") {
                        // Split the line by space and get the last part
                        let parts: Vec<&str> = output.split_whitespace().collect();
                        if let Some(file_path) = parts.last() {
                            //info!("{file_path}");
                            process_vanity_file(file_path);
                            //println!("File path: {}", file_path.color(Color::Magenta));
                            // Uncomment to print the file path
                        }
                    } else if output.contains("Searched") {
                        // For "Searched" lines, just print the output (dimmed or normal)
                        //println!("{}", output.color(Color::BrightMagenta));
                    }
                }
            });

            loading_handler_ref.to_previous_page();
            display_info_page(
                        vec![
                            InfoSegment::Emphasized(String::from("Vanity Generation task started.")),
                            InfoSegment::Normal(String::from(
                                "New keypairs will be written to 'vanity-keypairs.json' under 'temp/' folder. ",
                            )),
                            InfoSegment::Normal(String::from("Return or continue from this page to stop the task.")),
                        ],
                        String::from("Grinding."),
                        &mut loading_handler_ref,
                        Some(OptionCallback::StopGrindTask(Arc::clone(&child_guard))),
                        None,
                        None,
                    ); //stdout_task.await;

            // Wait for the child process to finish
            //let status = child.wait().await;
        }
        
        OptionCallback::StopGrindTask(child) => {
            let mut child_lock = child.lock().await;
            // Now you can safely kill or interact with the child process
            child_lock.kill().await;

            loading_handler_ref.to_previous_page();
        }
        
        OptionCallback::ManageLut(operation_type) => {
            //first of all load the most recenlty created lut
            let lut_file = load_most_recent_lut();
            if let Ok(content) = lut_file {
                display_loading_page(
                    String::from("Fetching lookup table data."),
                    &mut loading_handler_ref,
                );
                drop(loading_handler_ref);

                let connection_ref = Arc::clone(&connection);
                tokio::spawn(async move {
                    let lut_data = connection
                        .get_account_data(&Pubkey::from_str(&content.lookup_table).unwrap());

                    if let Ok(account_data) = lut_data {
                        let lut_state = AddressLookupTable::deserialize(&account_data).unwrap();
                        // info!("{:#?}", content.lookup_table);
                        // info!("{:#?}", lut_state);

                        let curr_slot = connection_ref.get_slot();
                        let slot_hashes = connection_ref.get_account(&slot_hashes::sysvar::ID);

                        // info!("after slot and slot hashes fetching");

                        if curr_slot.is_err() || slot_hashes.is_err() {
                            let mut handler_ref = menu_handler_clone1.lock().await;
                            handler_ref.to_previous_page();
                            display_error_page(
                                String::from("Failed to fetch lookup table account state"),
                                Some(String::from("Lookup table error")),
                                None,
                                Some(10),
                                &mut handler_ref,
                                false, //menu_handler_clone2
                            );
                            spawn_error_input_listener(menu_handler_clone2, 10);
                            return;
                        }

                        let slot_hashes: SlotHashes =
                            bincode::deserialize(&slot_hashes.unwrap().data).unwrap();

                        // info!("after slot hashes deserialization");

                        let lookup_table_status =
                            lut_state.meta.status(curr_slot.unwrap(), &slot_hashes);
                        // info!("after getting status");

                        let mut handler_ref = menu_handler_clone1.lock().await;

                        match operation_type {
                            LutCallback::Deactivate => {
                                let mut lut_deactivation_err: Option<(String, Option<String>)> =
                                    None;
                                match lookup_table_status {
                                    LookupTableStatus::Deactivating { remaining_blocks } => {
                                        lut_deactivation_err = Some((
                                            String::from("Lookup Table is Already de-activating"),
                                            Some(format!(
                                                "-- Remaining slots till completion: {}",
                                                remaining_blocks
                                            )),
                                        ))
                                    }
                                    LookupTableStatus::Deactivated => {
                                        lut_deactivation_err = Some((
                                            String::from("Lookup Table is deactivated"),
                                            None,
                                        ))
                                    }
                                    LookupTableStatus::Activated => {}
                                }

                                if let Some((e, ctx)) = lut_deactivation_err {
                                    handler_ref.to_previous_page();
                                    display_error_page(
                                        e,
                                        Some(String::from("Lookup table error")),
                                        if ctx.is_some() {
                                            Some(vec![ctx.unwrap()])
                                        } else {
                                            None
                                        },
                                        Some(10),
                                        &mut handler_ref,
                                        false, //menu_handler_clone2
                                    );
                                    spawn_error_input_listener(menu_handler_clone2, 10);
                                    return;
                                }
                            }

                            LutCallback::Close => {
                                let mut lut_close_err: Option<(String, Option<String>)> = None;
                                match lookup_table_status {
                                    LookupTableStatus::Deactivating { remaining_blocks } => {
                                        lut_close_err = Some((
                                            String::from("Lookup Table is Already de-activating"),
                                            Some(format!(
                                                "-- Remaining slots till completion: {}",
                                                remaining_blocks
                                            )),
                                        ))
                                    }
                                    LookupTableStatus::Activated => {
                                        lut_close_err = Some((
                                            String::from(
                                                "Lookup Table needs to be deactivated first",
                                            ),
                                            None,
                                        ))
                                    }
                                    LookupTableStatus::Deactivated => {}
                                }

                                if let Some((e, ctx)) = lut_close_err {
                                    handler_ref.to_previous_page();
                                    display_error_page(
                                        e,
                                        Some(String::from("Lookup table error")),
                                        if ctx.is_some() {
                                            Some(vec![ctx.unwrap()])
                                        } else {
                                            None
                                        },
                                        Some(10),
                                        &mut handler_ref,
                                        false, //menu_handler_clone2
                                    );
                                    spawn_error_input_listener(menu_handler_clone2, 10);
                                    return;
                                }
                            }
                        }

                        // info!("before displaying bundling loading page");

                        display_loading_page(
                            match operation_type {
                                LutCallback::Deactivate => {
                                    String::from("Deactivating lookup table")
                                }
                                LutCallback::Close => String::from("Closing lookup table"),
                            },
                            &mut handler_ref,
                        );
                        drop(handler_ref);

                        let funder_ref = Arc::clone(&funding_wallet);
                        tokio::spawn(async move {
                            let bundle_guard = get_bundle_guard(
                                &funder_ref.pubkey(),
                                &Pubkey::from_str(BOT_PROGRAM_ID).unwrap(),
                            );
                            let guard_account = connection.get_account_with_commitment(
                                &bundle_guard,
                                CommitmentConfig::processed(),
                            );
                            if let Err(ref e) = guard_account {
                                let mut handler_ref = menu_handler_clone1.lock().await;
                                handler_ref.to_previous_page();
                                display_bundle_guard_fetch_error_page(&mut handler_ref);
                                spawn_error_input_listener(menu_handler, 10);
                                return
                            }

                            let guard_data = BundleGuard::deserialize(
                                &mut &guard_account.unwrap().value.unwrap().data[8..],
                            )
                            .unwrap();

                            let tip = current_tip.load(Ordering::Relaxed);

                            //here we perform the task of spamming bundles till success.
                            let bundle_balancer = Arc::new(BundleSenderBalancer::new());
                            let stop_flag = Arc::new(AtomicBool::new(false));
                            let timeout_flag = Arc::new(AtomicBool::new(false));
                            // Task for sending bundles
                            let bundle_sender_flag = Arc::clone(&stop_flag);
                            let bundle_timeout_flag = Arc::clone(&timeout_flag);
                            let bundle_balancer_ref = Arc::clone(&bundle_balancer);
                            let funder_keypair_ref = Arc::clone(&funding_wallet);
                            let notify_websocket_opened = Arc::new(Notify::new());
                            let notify_websocket_opened_ref = Arc::clone(&notify_websocket_opened);
                            let lut_address = Pubkey::from_str(&content.lookup_table).unwrap();

                            tokio::spawn(async move {
                                blockhash_manager.start();
                                notify_websocket_opened_ref.notified().await;
                                let semaphore = Arc::new(Semaphore::new(100));

                                while !bundle_sender_flag.load(Ordering::Relaxed)
                                    && !bundle_timeout_flag.load(Ordering::Relaxed)
                                {
                                    let permit =
                                        Arc::clone(&semaphore).acquire_owned().await.unwrap();

                                    let bundle_balancer_ref = Arc::clone(&bundle_balancer_ref);
                                    let funder_keypair_ref = Arc::clone(&funder_keypair_ref);
                                    let bundle = get_redeem_lookup_table_bundle(
                                        Arc::clone(&blockhash_manager),
                                        &bundle_guard,
                                        guard_data.nonce,
                                        funder_keypair_ref,
                                        lut_address,
                                        tip,
                                        operation_type,
                                    );

                                    //simulate_bundle(bundle, true).await;
                                    tokio::spawn(async move {
                                        let _ = bundle_balancer_ref.send_bundle(bundle).await;
                                        drop(permit);
                                    });
                                    sleep(Duration::from_millis(50)).await;
                                }
                                //info!("reached timeout");
                                blockhash_manager.stop();
                            });

                            spawn_bundle_timeout_task(
                                Arc::clone(&global_config),
                                Arc::clone(&timeout_flag),
                                0
                            );

                            let account_monitor_flag = Arc::clone(&stop_flag);
                            let account_monitor_timeout = Arc::clone(&timeout_flag);
                            let connection_ref = Arc::clone(&connection);
                            //let funder_ref = Arc::clone(&funding_wallet);
                            let notify_websocket_opened_ref = Arc::clone(&notify_websocket_opened);

                            tokio::spawn(async move {
                                let (ws_stream, _) = connect_async(wss_url.as_ref()).await.unwrap();
                                let (mut write, mut read) = ws_stream.split();

                                let subscription_message =
                                    get_account_subscription_message(&bundle_guard);

                                write
                                    .send(Message::Text(subscription_message.to_string()))
                                    .await
                                    .unwrap();
                                notify_websocket_opened_ref.notify_one();

                                loop {
                                    // Check exit conditions first
                                    if account_monitor_flag.load(Ordering::Relaxed)
                                        || account_monitor_timeout.load(Ordering::Relaxed)
                                    {
                                        break;
                                    }

                                    // Use tokio::select! to handle multiple async operations
                                    tokio::select! {
                                        Some(msg) = read.next() => {
                                            match msg {
                                                Ok(Message::Text(text)) => {
                                                    if let Some(notification_lamports) = extract_lamports(&text) {

                                                        account_monitor_flag.store(true, Ordering::Relaxed);
                                                        //break;
                                                    }
                                                }
                                                Ok(Message::Close(_)) => {
                                                    account_monitor_timeout.store(true, Ordering::Relaxed);
                                                }
                                                Err(_) => {
                                                }
                                                _ => {}
                                            }
                                        }
                                        _ = tokio::time::sleep(Duration::from_secs(1)) => {
                                            // Periodic check, do nothing special
                                            continue;
                                        }
                                    }
                                }

                                let mut handler_ref = menu_handler_clone2.lock().await;

                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                if account_monitor_timeout.load(Ordering::Relaxed) {
                                    display_bundle_timeout_error_page(&mut handler_ref);
                                    //spawn_error_input_listener(menu_handler_clone1, 10);
                                } else {
                                    if let LutCallback::Close = operation_type {
                                        let _ = remove_most_recent_lut();
                                    }

                                    display_info_page(
                                        vec![InfoSegment::Normal(match operation_type {
                                            LutCallback::Deactivate => {
                                                String::from("Deactivated lookup table")
                                            }
                                            LutCallback::Close => {
                                                String::from("Closed lookup table")
                                            }
                                        })],
                                        String::from("Success."),
                                        &mut handler_ref,
                                        None,
                                        None,
                                        None,
                                    ); //stdout_task.await;
                                }
                            });
                        });
                    } else if let Err(e) = lut_data {
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        handler_ref.to_previous_page();
                        display_error_page(
                            String::from("Failed to fetch lookup table account state"),
                            Some(String::from("Lookup table error")),
                            None,
                            Some(10),
                            &mut handler_ref,
                            false, //menu_handler_clone2
                        );
                        spawn_error_input_listener(menu_handler_clone2, 10);
                    }
                });
            } else if let Err(e) = lut_file {
                loading_handler_ref.to_previous_page();
                display_error_page(
                    e,
                    Some(String::from("Lookup table error")),
                    None,
                    Some(10),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );

                spawn_error_input_listener(menu_handler_clone2, 10);
            }
        }
        
        OptionCallback::StartNewCoinsMonitor(webhook_url_or_token, channel) => {
            //first of all validate the webhook url

            let webhook_url_validation = validate_discord_webhook_url(&webhook_url_or_token);

            if let Err(ref e) = webhook_url_validation {
                loading_handler_ref.to_previous_page();
                display_error_page(
                    e.clone(),
                    Some(String::from("Webhook error")),
                    None,
                    Some(10),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 10);

                return;
            }

            let stop_flag = Arc::new(AtomicBool::new(false));

            display_info_page(
                vec![
                    InfoSegment::Emphasized(String::from("New coins monitor task started.")),
                    InfoSegment::Normal(String::from(
                        "-- All new coin notifications will be sent to the provided channel.",
                    )),
                    InfoSegment::Normal(String::from(
                        "-- Return or continue from this page to explicitly stop the task.",
                    )),
                ],
                String::from("New Coins"),
                &mut loading_handler_ref,
                Some(OptionCallback::StopMonitorTask(Arc::clone(
                    &stop_flag,
                ))),
                None,
                None,
            );

            drop(loading_handler_ref);

            //launch the task
            let main_task_stop_flag = Arc::clone(&stop_flag);
            //let connection_ref = Arc::clone(&connection);
            tokio::spawn(async move {
                //create websocket connection
                let (ws_stream, _) = connect_async(wss_url.as_ref()).await.unwrap();
                let (mut write, mut read) = ws_stream.split();

                let subscription_message =
                    get_transaction_logs_subscription_message(&Pubkey::from_str_const(MINT_AUTH), "processed");

                write
                    .send(Message::Text(subscription_message.to_string()))
                    .await
                    .unwrap();

                loop {
                    if main_task_stop_flag.load(Ordering::Relaxed) {
                        break;
                    }

                    match read.next().await {
                        Some(message) => {
                            if let Ok(json_data) = message {
                                let text = json_data.to_text().unwrap_or("");
                                if text.is_empty() {
                                    continue;
                                }

                                if let Ok(deserialized_message) =
                                    serde_json::from_str::<Value>(text)
                                {
                                    // Navigate to the "value" object
                                    if let Some(value) = deserialized_message
                                        .get("params")
                                        .and_then(|p| p.get("result"))
                                        .and_then(|r| r.get("value"))
                                    {
                                        // Determine if the transaction failed
                                        let transaction_failed =
                                            value.get("err").map(|err| !err.is_null());

                                        if let Some(res) = transaction_failed {
                                            //info!("transaction failure deserialized");
                                            if res {
                                                info!("log subscribe transaction failed");
                                                continue;
                                            }
                                        }

                                        // Get the "signature" field
                                        let transaction_signature = value
                                            .get("signature")
                                            .and_then(|sig| sig.as_str())
                                            .map(|sig| sig.to_string());
                                        
                                        if transaction_signature.is_none() {
                                            info!("transaction sig could not be deserialized from log subscribe");
                                            continue;
                                        }


                                        //index into the transaction logs and get the create instruction logs
                                        if let Some(logs) = value.get("logs").and_then(|logs| logs.as_array()) {
                                            //info!("logs deserialized: {:#?}", logs);
                                            
                                            let mut create_logs = Vec::new();
                                            let mut in_create_instruction = false;
                                            
                                            for (index, log) in logs.iter().enumerate() {
                                                if let Some(log_str) = log.as_str() {
                                                    // If we're in the "Create" instruction, collect logs
                                                    if in_create_instruction {
                                                        // If the log contains a top-level invoke [1], that means the instruction has ended
                                                        if log_str.contains("invoke [1]") {
                                                            break; // End of the current instruction
                                                        }
                                                        create_logs.push(log_str.to_string()); // Add the log to the create_logs
                                                    
                                                    } else if log_str.contains("Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]") {
                                                        // When we find the specific invoke line, check the next log for "Create"
                                                        if let Some(next_log) = logs.get(index + 1) {
                                                            if let Some(next_log_str) = next_log.as_str() {
                                                                if next_log_str.contains("Program log: Instruction: Create") {
                                                                    in_create_instruction = true; // Start collecting logs
                                                                    create_logs.push(log_str.to_string()); // Add the current log to create_logs
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            //now we get the newly created info from the logged program data at the end
                                            'outer: for log in create_logs.iter() {
                                                if let Some(data_line) = log.strip_prefix("Program data: ") {
                                                    //info !("Base64 string: {}", data_line);
                                                    // Step 1: Base64 decode the "Program data"
                                                    match decode(data_line) {
                                                        Ok(decoded_data) => {
                                                            //info!("Decoded Data Length: {}", decoded_data.len());
                                                            //info!("Decoded Data Hex: {}", hex::encode(&decoded_data));
                                                            
                                                            //let discriminator = &decoded_data[0..8];
                                                            
                                                            let mut current_pos = 8; // Start after discriminators
                                                            let mut extracted_strings = Vec::new();
                                                            
                                                            // Deserialize 3 strings
                                                            for _ in 0..3 {
                                                                // Read 4-byte length
                                                                let length = match decoded_data
                                                                    [current_pos..current_pos + 4]
                                                                    .try_into()
                                                                    .map(u32::from_le_bytes)
                                                                {
                                                                    Ok(len) => len as usize,
                                                                    Err(_) => {
                                                                        break 'outer; // Break outer loop on error
                                                                    }
                                                                };                                                                //info!("string {} length: {}", idx+1, length);
                                                                current_pos += 4;
                                                            
                                                                // Extract string
                                                                if current_pos + length > decoded_data.len() {
                                                                    break 'outer; // Break outer loop on error
                                                                }
                                                                let string_data = &decoded_data[current_pos..current_pos + length];
                                                                let string = match String::from_utf8(string_data.to_vec()) {
                                                                    Ok(s) => s,
                                                                    Err(_) => {
                                                                        break 'outer; // Break outer loop on error
                                                                    }
                                                                };
                                                                extracted_strings.push(string);
                                                                current_pos += length;
                                                            }

                                                            if current_pos + 32 > decoded_data.len() {break 'outer;}
                                                            let mint_bytes = &decoded_data[current_pos..current_pos+32];
                                                            let mint_pubkey = Pubkey::new_from_array(mint_bytes.try_into().unwrap());
                                                            current_pos += 32;

                                                            if current_pos + 32 > decoded_data.len() {break 'outer;}
                                                            let bonding_curve_bytes = &decoded_data[current_pos..current_pos+32];
                                                            let bonding_curve_pubkey = Pubkey::new_from_array(bonding_curve_bytes.try_into().unwrap());
                                                            current_pos += 32;

                                                            if current_pos + 32 > decoded_data.len() {break 'outer;}
                                                            let user_bytes = &decoded_data[current_pos..current_pos+32];
                                                            let user_pubkey = Pubkey::new_from_array(user_bytes.try_into().unwrap());
                                                            current_pos += 32;

                                                            //info!("{:#?}", &extracted_strings);
                                                            //info!("{:?}", mint_pubkey);
                                                            //info!("{:?}", bonding_curve_pubkey);
                                                            //info!("{:?}", user_pubkey);

                                                            //now launch a task to fetch the image and then send the embed
                                                            //through the provided webhook
                                                            let task_webhook_url= webhook_url_or_token.clone();
                                                            tokio::spawn(async move {
                                                                //first of all fetch the image for the new mint from teh ipfs url
                                                                let adjusted_metadata_url = fix_ipfs_url(&extracted_strings[2]);
                                                                let client = Client::new();
                                                                let response = client.get(&adjusted_metadata_url).send().await;
                                                                if let Ok(res) = response {
                                                                    let json_response = res.json::<Value>().await;
                                                                    if let Ok(metadata) = json_response {
                                                                        let image_link = metadata.get("image").and_then(|val| val.as_str());
                                                                        //now send the embed 
                                                                        let _  = send_create_event_embed(&task_webhook_url, &extracted_strings[0], &extracted_strings[1], &mint_pubkey.to_string(), &user_pubkey.to_string(), image_link, transaction_signature.unwrap().as_str()).await;
                                                                    }
                                                                }
                                                            });


                                                        }


                                                        Err(err) => {
                                                            //eprintln!("Failed to decode Base64 data: {}", err);
                                                        }
                                                    }
                                                    break
                                                }
                                            }
                                
                                        }

                                    }
                                }

                            }
                        }
                        None => break, // WebSocket stream ended
                    }
                }
            });

            //now we display the info page about successful task start
        }
        
        OptionCallback::StartKothMonitor(webhook_url_or_token, channel ) => {
            let webhook_url_validation = validate_discord_webhook_url(&webhook_url_or_token);

            if let Err(ref e) = webhook_url_validation {
                loading_handler_ref.to_previous_page();
                display_error_page(
                    e.clone(),
                    Some(String::from("Webhook error")),
                    None,
                    Some(10),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 10);

                return;
            }

            let stop_flag = Arc::new(AtomicBool::new(false));

            display_info_page(
                vec![
                    InfoSegment::Emphasized(String::from("KOTH monitor task started.")),
                    InfoSegment::Normal(String::from(
                        "-- All new KOTH notifications will be sent to the provided channel.",
                    )),
                    InfoSegment::Normal(String::from(
                        "-- Return or continue from this page to explicitly stop the task.",
                    )),
                ],
                String::from("KOTH"),
                &mut loading_handler_ref,
                Some(OptionCallback::StopMonitorTask(Arc::clone(
                    &stop_flag,
                ))),
                None,
                None,
            );

            drop(loading_handler_ref);

            //launch the task
            let main_task_stop_flag = Arc::clone(&stop_flag);
            //let connection_ref = Arc::clone(&connection);

            tokio::spawn(async move {
            let mut prev_mint: Option<Pubkey> = None; 

            //here im gonna fetch the newest KOTH coin every 2 seconds
            loop {
                if main_task_stop_flag.load(Ordering::Relaxed) {
                    break;
                }
                sleep(Duration::from_secs(2)).await;


                let newest_koth_validation = fetch_latest_koth().await;
                if newest_koth_validation.is_err() {
                    info!("{:#?}", &newest_koth_validation);
                    continue
                }

                let koth = newest_koth_validation.unwrap();
                //now im gonna extract the data I need to showcase on the koth notification
                let mint = koth.get("mint").and_then(|name| name.as_str());
                let name = koth.get("name").and_then(|name| name.as_str());
                let symbol = koth.get("symbol").and_then(|name| name.as_str());
                let creator = koth.get("creator").and_then(|name| name.as_str());
                let image_uri = koth.get("image_uri").and_then(|name| name.as_str());

                if mint.is_none() | name.is_none() | symbol.is_none() | creator.is_none() | image_uri.is_none() {
                    continue;
                }

                let curr_pubkey = Pubkey::from_str(mint.clone().unwrap()).unwrap();
                if let Some(prev_mint) = prev_mint {
                    if curr_pubkey.eq(&prev_mint){
                        continue
                    }
                };

                prev_mint = Some(curr_pubkey);


                let _  = send_new_koth_event_embed(
                    &webhook_url_or_token,
                    name.unwrap(),
                    symbol.unwrap(),
                    mint.unwrap(),
                    creator.unwrap(),
                    image_uri
                ).await;

            }
            
            });
        }
    
        OptionCallback::StartMigrationMonitor(webhook_url_or_token, channel ) => {

            let webhook_url_validation = validate_discord_webhook_url(&webhook_url_or_token);

            if let Err(ref e) = webhook_url_validation {
                loading_handler_ref.to_previous_page();
                display_error_page(
                    e.clone(),
                    Some(String::from("Webhook error")),
                    None,
                    Some(10),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 10);

                return;
            }

            let stop_flag = Arc::new(AtomicBool::new(false));

            display_info_page(
                vec![
                    InfoSegment::Emphasized(String::from("Migration monitor task started.")),
                    InfoSegment::Normal(String::from(
                        "-- All new migration notifications will be sent to the provided channel.",
                    )),
                    InfoSegment::Normal(String::from(
                        "-- Return or continue from this page to explicitly stop the task.",
                    )),
                ],
                String::from("Migrations"),
                &mut loading_handler_ref,
                Some(OptionCallback::StopMonitorTask(Arc::clone(
                    &stop_flag,
                ))),
                None,
                None,
            );

            drop(loading_handler_ref);

            //launch the task
            let main_task_stop_flag = Arc::clone(&stop_flag);
            let connection_ref = Arc::clone(&connection);
            tokio::spawn(async move {
                //create websocket connection
                let (ws_stream, _) = connect_async(wss_url.as_ref()).await.unwrap();
                let (mut write, mut read) = ws_stream.split();

                let subscription_message =
                    get_transaction_logs_subscription_message(&Pubkey::from_str_const(PUMP_MIGRATION_AUTHORITY), "confirmed");

                write
                    .send(Message::Text(subscription_message.to_string()))
                    .await
                    .unwrap();

                loop {
                    if main_task_stop_flag.load(Ordering::Relaxed) {
                        break;
                    }

                    match read.next().await {
                        Some(message) => {
                            if let Ok(json_data) = message {
                                let text = json_data.to_text().unwrap_or("");
                                if text.is_empty() {
                                    continue;
                                }

                                if let Ok(deserialized_message) =
                                    serde_json::from_str::<Value>(text)
                                {
                                    // Navigate to the "value" object
                                    if let Some(value) = deserialized_message
                                        .get("params")
                                        .and_then(|p| p.get("result"))
                                        .and_then(|r| r.get("value"))
                                    {
                                        // Determine if the transaction failed
                                        let transaction_failed =
                                            value.get("err").map(|err| !err.is_null());

                                        if let Some(res) = transaction_failed {
                                            //info!("transaction failure deserialized");
                                            if res {
                                                info!("log subscribe transaction failed");
                                                continue;
                                            }
                                        }

                                        // Get the "signature" field
                                        let transaction_signature = value
                                            .get("signature")
                                            .and_then(|sig| sig.as_str())
                                            .map(|sig| sig.to_string());
                                        
                                        if transaction_signature.is_none() {
                                            info!("transaction sig could not be deserialized from log subscribe");
                                            continue;
                                        }
                                        info!("{:?}", transaction_signature);


                                        //index into the transaction logs and get the migration instruction logs
                                        if let Some(logs) = value.get("logs").and_then(|logs| logs.as_array()) {
                                            
                                            //info!("logs deserialized: {:#?}", logs);
                                            
                                            //let mut create_logs = Vec::new();
                                            let mut is_lp_creation_instruction = false;


                                            //info!("{:#?}", logs);
                                            for (index, log) in logs.iter().enumerate() {
                                                if let Some(log_str) = log.as_str() {
                                                    if log_str.contains("Program log: Instruction: Migrate") {
                                                        
                                                        if logs[index + 1].as_str().unwrap().contains("Program log: Bonding curve already migrated") {
                                                            info!("logs do not contain pump amm creation");
                                                        }else {
                                                            info!("logs contain pump amm creation");
                                                            is_lp_creation_instruction = true; // Start collecting logs
                                                        }
                                                        break;
                                                    }
                                            }
                                        }
                                            
                                                let signature_bytes = bs58::decode(transaction_signature.unwrap()).into_vec().unwrap();
                                                let signature_fixed_byte_array: [u8; 64] = signature_bytes.try_into().unwrap();
                                                let transaction_details_verification = connection_ref.get_transaction_with_config(
                                                    &Signature::from(signature_fixed_byte_array), 
                                                    RpcTransactionConfig {
                                                        encoding: Some(solana_transaction_status::UiTransactionEncoding::JsonParsed),
                                                        commitment: Some(CommitmentConfig::confirmed()),
                                                        max_supported_transaction_version: Some(0),
                                                });
                                                if let Ok(transaction_details) = transaction_details_verification {
                                                    //info!("{:#?}", &transaction_details);

                                                    if let Some(meta) = transaction_details.transaction.meta {
                                                        
                                                        //ok i have to loop through all the inner_ixs of all the ixs so I know which one is the ix i need.

                                                        let meta_inner_ixs = &meta.inner_instructions.unwrap();
                                                        //info!("{:#?}", meta_inner_ixs);
                                                        let mut mint: Option<String> = None;
                                                        let mut pool_id: Option<String> = None;
                                                        for ix in meta_inner_ixs {
                                                            for possible_ix in &ix.instructions {
                                                                if let UiInstruction::Parsed(parsed_ix) = possible_ix {
                                                                    match parsed_ix {
                                                                        UiParsedInstruction::PartiallyDecoded(partially_parsed_instruction) => {
                                                                            //here we check for program id first
                                                                            if &partially_parsed_instruction.program_id == PUMP_AMM_ADDRESS {
                                                                                //now im gonna check for pool creation discriminator:

                                                                                //info!("decoded ix: {:#?}", partially_parsed_instruction);

                                                                                let decoded = bs58::decode(&partially_parsed_instruction.data)
                                                                                .into_vec();


                                                                                if let Ok(decoded_data) = decoded {
                                                                                    //get the first 8 bytes and check for matching discrimniator:


                                                                                    let first_8_bytes = decoded_data.get(0..8);
                                                                                    //info!("first 8 bytes: {:#?}", &first_8_bytes);
                                                                                    
                                                                                    if let Some(possible_disciminator) = first_8_bytes {
                                                                                                                                                                                
                                                                                        if possible_disciminator == [
                                                                                            233,
                                                                                            146,
                                                                                            209,
                                                                                            142,
                                                                                            207,
                                                                                            104,
                                                                                            64,
                                                                                            188
                                                                                        ] {
                                                                                            //info!("{:#?}", "discriminator matches!!!");

                                                                                            mint = partially_parsed_instruction.accounts.get(3).cloned();
                                                                                            pool_id = partially_parsed_instruction.accounts.get(0).cloned();
                                                                                            break
                                                                                            //then we know for a fact this is the pool creation ix
                                                                                            //we get the accounts that we need
                                                                                        }                                                                                    }

                                                                                }

                                                                            }
                                                                        },
                                                                        _=> {},
                                                                    };
                                                                }
                                                            }
                                                        }
                                                        

                                                        if pool_id.is_none() | mint.is_none() {
                                                            continue
                                                        }

                                                        let mint_pubkey = Pubkey::from_str(&mint.unwrap()).unwrap();
                                                        let pool_id_pubkey = Pubkey::from_str(&pool_id.unwrap()).unwrap();
                                                        //info!("{:#?}", mint_pubkey);
                                                        //info!("{:#?}", pool_id_pubkey);

                                                        let coin_data_result = fetch_pump_token_general_data(&mint_pubkey).await;

                                                        if let Ok(coin_data) = coin_data_result {
                                                            //now we extract the needed fields
                                                            let mint = coin_data.mint;
                                                            let name = coin_data.name;
                                                            let symbol = coin_data.symbol;
                                                            let image_uri = coin_data.image_uri;

                                                            //info!("{:?}", name);
                                                            //info!("{:?}", symbol);
                                                            //info!("{:?}", image_uri);

                                                            //send the embed for the migration event
                                                            let _  = send_new_migration_event(
                                                                &webhook_url_or_token,
                                                                &name,
                                                                &symbol,
                                                                &mint,
                                                                image_uri.as_deref(),
                                                                &pool_id_pubkey.to_string()
                                                            ).await;
                                                        }
                                                    };

                                                    //here we parse and get the accounts and info we need for the info alert.
                                                }else if let Err(e) = transaction_details_verification{
                                                    info!("{:#?}", e);
                                                    continue;
                                                }
                                            };

                                    }
                                }

                            }
                        }
                        None => break, // WebSocket stream ended
                    }
                }
            });

        }
        
        
        OptionCallback::StopMonitorTask(flag) => {
            flag.store(true, Ordering::Relaxed);
            loading_handler_ref.return_to_pump_main_menu();
        }
        
        _ => {}
    };
}
