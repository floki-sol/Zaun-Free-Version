use artem::ConfigBuilder;
use borsh::BorshDeserialize;
use crossterm::{style::SetBackgroundColor, ExecutableCommand};
use futures::{SinkExt, StreamExt};
use log::info;
use num_traits::Zero;
use serde_json::Value;
use solana_client::{
    rpc_client::RpcClient, rpc_config::RpcTransactionConfig, rpc_response::Response,
};
use solana_sdk::{
    address_lookup_table::{state::AddressLookupTable, AddressLookupTableAccount},
    commitment_config::CommitmentConfig,
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use solana_transaction_status::{UiCompiledInstruction, UiInstruction, UiParsedInstruction};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{
    native_mint,
    state::{Account, GenericTokenAccount},
};
use std::{
    collections::VecDeque,
    io::stdout,
    num::NonZeroU32,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::{Mutex, Notify, RwLock, Semaphore},
    time::sleep,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tui::style::Color;

use crate::{
    cli::{
        error::{
            display_balance_fetch_error_page, display_bundle_guard_fetch_error_page,
            display_bundle_timeout_error_page, display_error_page, spawn_error_input_listener,
        },
        info::{display_info_page, InfoSegment},
        loading_indicator::display_loading_page,
        menu::{MenuHandler, Page},
        options::OptionCallback,
    },
    constants::general::{
        BundleGuard, OperationIntensity, PumpKeys, TokenAccountBalanceState, BOT_PROGRAM_ID,
        PUMP_MIGRATION_AUTHORITY, PUMP_TOKEN_DECIMALS,
    },
    jito::bundles::{simulate_bundle, BundleSenderBalancer},
    loaders::{
        global_config_loader::GlobalConfig,
        launch_manifest_loader::validate_and_retrieve_launch_manifest, metadata_loader::Metadata,
    },
    utils::{
        blockhash_manager::RecentBlockhashManager,
        bonding_curve_provider::{
            get_bonding_curve_creator, run_curve_provider, BondingCurve, BondingCurveProvider,
        },
        bundle_factory::{
            get_bonded_multi_wallet_sell_bundle, get_burn_or_retrieve_tokens_bundle,
            get_normal_multi_wallet_sell_bundle,
        },
        comments_manager::{CommentType, CommentsManager},
        misc::{
            extract_lamports, get_account_subscription_message, get_associated_accounts,
            get_global_lut_data, spawn_bundle_timeout_task,
        },
        pdas::get_bundle_guard,
        pump_helpers::{
            derive_all_pump_dex_keys, derive_all_pump_keys, fetch_pump_token_general_data,
            PumpDexKeys,
        },
    },
};

pub async fn invoke_tracking_callback(
    menu_handler: Arc<Mutex<MenuHandler>>,
    connection: Arc<RpcClient>,
    wss_url: Arc<String>,
    blockhash_manager: Arc<RecentBlockhashManager>,
    callback: OptionCallback,
    dev_wallet: Arc<Keypair>,
    funding_wallet: Arc<Keypair>,
    global_config: Arc<RwLock<Option<GlobalConfig>>>,
    current_tip: Arc<AtomicU64>,
    _: Arc<String>,
) {
    let mut loading_handler_ref = menu_handler.lock().await;

    let menu_handler_clone1 = Arc::clone(&menu_handler);
    let menu_handler_clone2 = Arc::clone(&menu_handler);
    //let menu_handler_clone3 = Arc::clone(&menu_handler);

    match callback {
        //tracking
        OptionCallback::TrackNormal => {
            ////
        }
        OptionCallback::TrackBonded => {
            ////
        }

        OptionCallback::SingleWalletTradePump((
            trade_type,
            transfer_receiver,
            wallets,
            wallets_idx,
            amount,
            log_entries,
            pump_keys,
            ui_logs,
            limited_trade_logs,
            session_lut,
            global_lut,
        )) => {
            ////
        }

        OptionCallback::MultiWalletTradePump((
            trade_type,
            transfer_receiver,
            wallets,
            wallet_indices_and_amounts,
            log_entries,
            pump_keys,
            ui_logs,
            limited_trade_logs,
            session_lut,
            global_lut,
        )) => {
            ////
        }

        OptionCallback::StopNormalTracking((curve_provider, bump_manager, comments_manager)) => {
            ////
        }
        OptionCallback::StopBondedTracking => {
            ////
        }

        //quick actions
        OptionCallback::QuickSellAllNormal => {
            //first of all load the launch manifest
            let manifest_validation = validate_and_retrieve_launch_manifest();
            if let Err(ref e) = manifest_validation {
                display_error_page(
                    e.clone(),
                    Some(String::from("Launch manifest error")),
                    None,
                    Some(10),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 10);
                return;
            }

            let manifest = Arc::new(manifest_validation.unwrap());

            display_loading_page(String::from("Checking balances."), &mut loading_handler_ref);
            drop(loading_handler_ref);

            //now im gonna fetch the balances of the wallets and filter the valid ones
            tokio::spawn(async move {
                let creator = get_bonding_curve_creator(&manifest.mint, Arc::clone(&connection));
                if creator.is_none() {
                    let mut handler_ref = menu_handler_clone1.lock().await;
                    handler_ref.to_previous_page();
                    display_error_page(
                        String::from("Failed to fetch creator-vault address"),
                        Some(String::from("Creator Fetch Error")),
                        None,
                        Some(5),
                        &mut handler_ref,
                        false, //menu_handler_clone2
                    );
                    spawn_error_input_listener(menu_handler_clone2, 5);
                    return;
                }

                let creator_address = creator.unwrap();

                //first derive the pump keys
                let pump_keys = Arc::new(derive_all_pump_keys(
                    &dev_wallet.pubkey(),
                    manifest.mint.clone(),
                    &creator_address,
                ));

                // Fetch data outside the lock
                let pubkeys = Arc::clone(&manifest)
                    .wallet_entries
                    .iter()
                    .map(|val| val.wallet.pubkey())
                    .collect::<Vec<Pubkey>>();

                let launch_alu_table: Option<Arc<AddressLookupTableAccount>> = if pubkeys.len() > 1
                {
                    let alu_keys = pubkeys.iter().skip(1).cloned().collect::<Vec<Pubkey>>();
                    Some(Arc::new(AddressLookupTableAccount {
                        key: manifest.lookup_table,
                        addresses: get_associated_accounts(&alu_keys, pump_keys.mint),
                    }))
                } else {
                    None
                };

                let atas = get_associated_accounts(&pubkeys, Arc::clone(&pump_keys).mint);

                let accounts_result = Arc::clone(&connection)
                    .get_multiple_accounts_with_commitment(&atas, CommitmentConfig::processed());

                let mut balance_states_vec: Vec<TokenAccountBalanceState> = vec![];
                if let Ok(data) = accounts_result {
                    for (idx, data) in data.value.iter().enumerate() {
                        if let Some(account_data) = data {
                            let token_account_data = Account::unpack(&account_data.data).unwrap();
                            if token_account_data.amount != 0 {
                                balance_states_vec.push(
                                    TokenAccountBalanceState::ExistsWithBalance(
                                        token_account_data.amount,
                                    ),
                                );
                            } else {
                                balance_states_vec
                                    .push(TokenAccountBalanceState::ExistsWithNoBalance);
                            }
                        } else {
                            balance_states_vec.push(TokenAccountBalanceState::DoesNotExist);
                        }
                    }
                } else {
                    let mut handler_ref = menu_handler_clone1.lock().await;
                    handler_ref.to_previous_page();
                    display_balance_fetch_error_page(&mut handler_ref);
                    spawn_error_input_listener(menu_handler_clone2, 10);
                    return;
                }

                //check if there is no balance in all wallets
                let all_clear = balance_states_vec.iter().all(|balance_state| {
                    balance_state.eq(&TokenAccountBalanceState::DoesNotExist)
                        | balance_state.eq(&TokenAccountBalanceState::ExistsWithNoBalance)
                });

                if all_clear {
                    let mut handler_ref = menu_handler_clone1.lock().await;
                    handler_ref.to_previous_page();
                    display_info_page(
                        vec![InfoSegment::Normal(format!(
                            "No balance in wallets for operation.",
                        ))],
                        String::from("Info"),
                        &mut handler_ref,
                        None,
                        None,
                        None,
                    ); //stdout_task.await;
                    return;
                }

                //now filter the wallets that have balance in them
                let mut wallets_to_sell: Vec<Arc<Keypair>> = vec![];
                let mut amounts: Vec<u64> = vec![];
                for (idx, balance_state) in balance_states_vec.iter().enumerate() {
                    let wallet_entry = &manifest.wallet_entries[idx];
                    let pkey = Arc::clone(&wallet_entry.wallet);
                    if let TokenAccountBalanceState::ExistsWithBalance(ref amount) = balance_state {
                        wallets_to_sell.push(pkey);
                        amounts.push(*amount);
                    }
                }

                //now we just launch the operation bundle
                let mut handler_ref = menu_handler_clone1.lock().await;
                display_loading_page(String::from("Selling Everything"), &mut handler_ref);
                drop(handler_ref);
                let funder_ref = Arc::clone(&funding_wallet);
                tokio::spawn(async move {
                    let bundle_guard = get_bundle_guard(
                        &funder_ref.pubkey(),
                        &Pubkey::from_str(BOT_PROGRAM_ID).unwrap(),
                    );
                    let guard_account = connection
                        .get_account_with_commitment(&bundle_guard, CommitmentConfig::processed());

                    if let Err(ref e) = guard_account {
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        handler_ref.to_previous_page();
                        display_bundle_guard_fetch_error_page(&mut handler_ref);
                        spawn_error_input_listener(menu_handler, 10);
                        return;
                    }

                    let guard_data = BundleGuard::deserialize(
                        &mut &guard_account.unwrap().value.unwrap().data[8..],
                    )
                    .unwrap();

                    //let mut handler_ref = menu_handler_clone1.lock().await;
                    let tip = current_tip.load(Ordering::Relaxed);
                    let global_lut = Arc::new(get_global_lut_data());

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

                    tokio::spawn(async move {
                        blockhash_manager.start();
                        notify_websocket_opened_ref.notified().await;
                        let semaphore = Arc::new(Semaphore::new(100));

                        while !bundle_sender_flag.load(Ordering::Relaxed)
                            && !bundle_timeout_flag.load(Ordering::Relaxed)
                        {
                            let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();

                            let bundle_balancer_ref = Arc::clone(&bundle_balancer_ref);
                            let funder_keypair_ref = Arc::clone(&funder_keypair_ref);

                            let bundle = get_normal_multi_wallet_sell_bundle(
                                Arc::clone(&pump_keys),
                                Arc::clone(&blockhash_manager),
                                &wallets_to_sell,
                                &amounts,
                                Arc::clone(&funder_keypair_ref),
                                &bundle_guard,
                                guard_data.nonce,
                                tip,
                                Arc::clone(&global_lut),
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
                        0,
                    );

                    let account_monitor_flag = Arc::clone(&stop_flag);
                    let account_monitor_timeout = Arc::clone(&timeout_flag);
                    let connection_ref = Arc::clone(&connection);
                    let notify_websocket_opened_ref = Arc::clone(&notify_websocket_opened);

                    tokio::spawn(async move {
                        let (ws_stream, _) = connect_async(wss_url.as_ref()).await.unwrap();
                        let (mut write, mut read) = ws_stream.split();

                        let subscription_message = get_account_subscription_message(&bundle_guard);

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
                        handler_ref.to_previous_page();
                        if timeout_flag.load(Ordering::Relaxed) {
                            display_bundle_timeout_error_page(&mut handler_ref);
                            //spawn_error_input_listener(menu_handler_clone1, 10);
                        } else {
                            display_info_page(
                                vec![InfoSegment::Normal(format!("Sold Everything."))],
                                String::from("Success."),
                                &mut handler_ref,
                                None,
                                None,
                                None,
                            ); //stdout_task.await;
                        }
                    });
                });
            });
        }
        OptionCallback::QuickSellAllBondedInsta => {
            //first of all load the launch manifest
            let manifest_validation = validate_and_retrieve_launch_manifest();
            if let Err(ref e) = manifest_validation {
                display_error_page(
                    e.clone(),
                    Some(String::from("Launch manifest error")),
                    None,
                    Some(10),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 10);
                return;
            }

            let manifest = Arc::new(manifest_validation.unwrap());

            display_loading_page(
                String::from("Retrieving Pump AMM pool."),
                &mut loading_handler_ref,
            );
            drop(loading_handler_ref);
            tokio::spawn(async move {
                let creator = get_bonding_curve_creator(&manifest.mint, Arc::clone(&connection));
                if creator.is_none() {
                    let mut handler_ref = menu_handler_clone1.lock().await;
                    handler_ref.to_previous_page();
                    display_error_page(
                        String::from("Failed to fetch creator-vault address"),
                        Some(String::from("Creator Fetch Error")),
                        None,
                        Some(5),
                        &mut handler_ref,
                        false, //menu_handler_clone2
                    );
                    spawn_error_input_listener(menu_handler_clone2, 5);
                    return;
                }

                let creator_address = creator.unwrap();

                //first derive the pump keys
                let pump_keys = Arc::new(derive_all_pump_keys(
                    &dev_wallet.pubkey(),
                    manifest.mint.clone(),
                    &creator_address,
                ));

                let derived_pump_dex_keys =
                    Arc::new(derive_all_pump_dex_keys(&pump_keys.mint, &creator_address));

                //now im gonna fetch the account data for the pool
                let account_data = connection.get_account_with_commitment(
                    &derived_pump_dex_keys.pool_id,
                    CommitmentConfig::processed(),
                );

                if matches!(account_data, Err(_) | Ok(Response { value: None, .. })) {
                    let mut handler_ref = menu_handler_clone1.lock().await;
                    handler_ref.to_previous_page();
                    display_error_page(
                        String::from("Coin did not bond."),
                        Some(String::from("Bonding Error")),
                        None,
                        Some(10),
                        &mut handler_ref,
                        false, //menu_handler_clone2
                    );
                    spawn_error_input_listener(menu_handler_clone2, 10);
                    return;
                }

                let mut handler_ref = menu_handler_clone1.lock().await;
                display_loading_page(
                    String::from("Pool found. Selling all tokens"),
                    &mut handler_ref,
                );

                drop(handler_ref);
                tokio::spawn(async move {
                    let pubkeys = Arc::clone(&manifest)
                        .wallet_entries
                        .iter()
                        .map(|val| val.wallet.pubkey())
                        .collect::<Vec<Pubkey>>();

                    let atas = get_associated_accounts(&pubkeys, Arc::clone(&pump_keys).mint);

                    let accounts_result = Arc::clone(&connection)
                        .get_multiple_accounts_with_commitment(
                            &atas,
                            CommitmentConfig::processed(),
                        );

                    let mut balance_states_vec: Vec<TokenAccountBalanceState> = vec![];
                    if let Ok(data) = accounts_result {
                        for (idx, data) in data.value.iter().enumerate() {
                            if let Some(account_data) = data {
                                let token_account_data =
                                    Account::unpack(&account_data.data).unwrap();
                                if token_account_data.amount != 0 {
                                    balance_states_vec.push(
                                        TokenAccountBalanceState::ExistsWithBalance(
                                            token_account_data.amount,
                                        ),
                                    );
                                } else {
                                    balance_states_vec
                                        .push(TokenAccountBalanceState::ExistsWithNoBalance);
                                }
                            } else {
                                balance_states_vec.push(TokenAccountBalanceState::DoesNotExist);
                            }
                        }
                    } else {
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        handler_ref.to_previous_page();
                        handler_ref.to_previous_page();
                        display_balance_fetch_error_page(&mut handler_ref);
                        spawn_error_input_listener(menu_handler_clone2, 10);
                        return;
                    }

                    //check if there is no balance in all wallets
                    let all_clear = balance_states_vec.iter().all(|balance_state| {
                        balance_state.eq(&TokenAccountBalanceState::DoesNotExist)
                            | balance_state.eq(&TokenAccountBalanceState::ExistsWithNoBalance)
                    });

                    if all_clear {
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        handler_ref.to_previous_page();
                        handler_ref.to_previous_page();
                        display_info_page(
                            vec![InfoSegment::Normal(format!(
                                "No balance in wallets for operation.",
                            ))],
                            String::from("Info"),
                            &mut handler_ref,
                            None,
                            None,
                            None,
                        ); //stdout_task.await;
                        return;
                    }

                    //now filter the wallets that have balance in them
                    let mut wallets_to_sell: Vec<Arc<Keypair>> = vec![];
                    let mut amounts: Vec<u64> = vec![];
                    for (idx, balance_state) in balance_states_vec.iter().enumerate() {
                        let wallet_entry = &manifest.wallet_entries[idx];
                        let pkey = Arc::clone(&wallet_entry.wallet);
                        if let TokenAccountBalanceState::ExistsWithBalance(ref amount) =
                            balance_state
                        {
                            wallets_to_sell.push(pkey);
                            amounts.push(*amount);
                        }
                    }

                    //info!("{:#?}", wallets_to_sell);
                    //info!("{:#?}", amounts);
                    let bundle_guard = get_bundle_guard(
                        &funding_wallet.pubkey(),
                        &Pubkey::from_str(BOT_PROGRAM_ID).unwrap(),
                    );
                    let guard_account = connection
                        .get_account_with_commitment(&bundle_guard, CommitmentConfig::processed());

                    if let Err(ref e) = guard_account {
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        handler_ref.to_previous_page();
                        handler_ref.to_previous_page();
                        display_bundle_guard_fetch_error_page(&mut handler_ref);
                        spawn_error_input_listener(menu_handler, 10);
                        return;
                    }

                    let guard_data = BundleGuard::deserialize(
                        &mut &guard_account.unwrap().value.unwrap().data[8..],
                    )
                    .unwrap();

                    //let mut handler_ref = menu_handler_clone1.lock().await;
                    let tip = current_tip.load(Ordering::Relaxed);
                    let global_lut = Arc::new(get_global_lut_data());

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

                    tokio::spawn(async move {
                        blockhash_manager.start();
                        notify_websocket_opened_ref.notified().await;
                        let semaphore = Arc::new(Semaphore::new(100));

                        while !bundle_sender_flag.load(Ordering::Relaxed)
                            && !bundle_timeout_flag.load(Ordering::Relaxed)
                        {
                            let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();

                            let bundle_balancer_ref = Arc::clone(&bundle_balancer_ref);
                            let funder_keypair_ref = Arc::clone(&funder_keypair_ref);

                            let bundle = get_bonded_multi_wallet_sell_bundle(
                                Arc::clone(&derived_pump_dex_keys),
                                Arc::clone(&blockhash_manager),
                                &wallets_to_sell,
                                &amounts,
                                Arc::clone(&funder_keypair_ref),
                                &bundle_guard,
                                guard_data.nonce,
                                tip,
                                Arc::clone(&global_lut),
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
                        0,
                    );

                    let account_monitor_flag = Arc::clone(&stop_flag);
                    let account_monitor_timeout = Arc::clone(&timeout_flag);
                    let connection_ref = Arc::clone(&connection);
                    let notify_websocket_opened_ref = Arc::clone(&notify_websocket_opened);

                    tokio::spawn(async move {
                        let (ws_stream, _) = connect_async(wss_url.as_ref()).await.unwrap();
                        let (mut write, mut read) = ws_stream.split();

                        let subscription_message = get_account_subscription_message(&bundle_guard);

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
                        handler_ref.to_previous_page();
                        if timeout_flag.load(Ordering::Relaxed) {
                            display_bundle_timeout_error_page(&mut handler_ref);
                            //spawn_error_input_listener(menu_handler_clone1, 10);
                        } else {
                            display_info_page(
                                vec![InfoSegment::Normal(format!("Sold Everything."))],
                                String::from("Success."),
                                &mut handler_ref,
                                None,
                                None,
                                None,
                            ); //stdout_task.await;
                        }
                    });

                    //now we launch the ray sell bundle
                });
            });
        }
        OptionCallback::QuickSellAllBondedAwaited => {
            //first of all load the launch manifest
            let manifest_validation = validate_and_retrieve_launch_manifest();
            if let Err(ref e) = manifest_validation {
                display_error_page(
                    e.clone(),
                    Some(String::from("Launch manifest error")),
                    None,
                    Some(10),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 10);
                return;
            }
            let manifest = Arc::new(manifest_validation.unwrap());

            display_loading_page(String::from("Preparing"), &mut loading_handler_ref);
            drop(loading_handler_ref);

            //first of all I have to load the bundle guard, wallets data and creator from api.
            tokio::spawn(async move {
                let creator = get_bonding_curve_creator(&manifest.mint, Arc::clone(&connection));
                if creator.is_none() {
                    let mut handler_ref = menu_handler_clone1.lock().await;
                    handler_ref.to_previous_page();
                    display_error_page(
                        String::from("Failed to fetch creator-vault address"),
                        Some(String::from("Creator Fetch Error")),
                        None,
                        Some(5),
                        &mut handler_ref,
                        false, //menu_handler_clone2
                    );
                    spawn_error_input_listener(menu_handler_clone2, 5);
                    return;
                }

                let creator_address = creator.unwrap();

                //first derive the pump keys
                let pump_keys = Arc::new(derive_all_pump_keys(
                    &dev_wallet.pubkey(),
                    manifest.mint.clone(),
                    &creator_address,
                ));

                let derived_pump_dex_keys =
                    Arc::new(derive_all_pump_dex_keys(&pump_keys.mint, &creator_address));

                let pubkeys = Arc::clone(&manifest)
                    .wallet_entries
                    .iter()
                    .map(|val| val.wallet.pubkey())
                    .collect::<Vec<Pubkey>>();

                let atas = get_associated_accounts(&pubkeys, Arc::clone(&pump_keys).mint);

                let accounts_result = Arc::clone(&connection)
                    .get_multiple_accounts_with_commitment(&atas, CommitmentConfig::processed());

                let mut balance_states_vec: Vec<TokenAccountBalanceState> = vec![];
                if let Ok(data) = accounts_result {
                    for (idx, data) in data.value.iter().enumerate() {
                        if let Some(account_data) = data {
                            let token_account_data = Account::unpack(&account_data.data).unwrap();
                            if token_account_data.amount != 0 {
                                balance_states_vec.push(
                                    TokenAccountBalanceState::ExistsWithBalance(
                                        token_account_data.amount,
                                    ),
                                );
                            } else {
                                balance_states_vec
                                    .push(TokenAccountBalanceState::ExistsWithNoBalance);
                            }
                        } else {
                            balance_states_vec.push(TokenAccountBalanceState::DoesNotExist);
                        }
                    }
                } else {
                    let mut handler_ref = menu_handler_clone1.lock().await;
                    //handler_ref.to_previous_page();
                    handler_ref.to_previous_page();
                    display_balance_fetch_error_page(&mut handler_ref);
                    spawn_error_input_listener(menu_handler_clone2, 10);
                    return;
                }

                //check if there is no balance in all wallets
                let all_clear = balance_states_vec.iter().all(|balance_state| {
                    balance_state.eq(&TokenAccountBalanceState::DoesNotExist)
                        | balance_state.eq(&TokenAccountBalanceState::ExistsWithNoBalance)
                });

                if all_clear {
                    let mut handler_ref = menu_handler_clone1.lock().await;
                    //handler_ref.to_previous_page();
                    handler_ref.to_previous_page();
                    display_info_page(
                        vec![InfoSegment::Normal(format!(
                            "No balance in wallets for operation.",
                        ))],
                        String::from("Info"),
                        &mut handler_ref,
                        None,
                        None,
                        None,
                    ); //stdout_task.await;
                    return;
                }

                //now filter the wallets that have balance in them
                let mut wallets_to_sell: Vec<Arc<Keypair>> = vec![];
                let mut amounts: Vec<u64> = vec![];
                for (idx, balance_state) in balance_states_vec.iter().enumerate() {
                    let wallet_entry = &manifest.wallet_entries[idx];
                    let pkey = Arc::clone(&wallet_entry.wallet);
                    if let TokenAccountBalanceState::ExistsWithBalance(ref amount) = balance_state {
                        wallets_to_sell.push(pkey);
                        amounts.push(*amount);
                    }
                }

                let bundle_guard = get_bundle_guard(
                    &funding_wallet.pubkey(),
                    &Pubkey::from_str(BOT_PROGRAM_ID).unwrap(),
                );
                let guard_account = connection
                    .get_account_with_commitment(&bundle_guard, CommitmentConfig::processed());

                if let Err(ref e) = guard_account {
                    let mut handler_ref = menu_handler_clone1.lock().await;
                    //handler_ref.to_previous_page();
                    handler_ref.to_previous_page();
                    display_bundle_guard_fetch_error_page(&mut handler_ref);
                    spawn_error_input_listener(menu_handler, 10);
                    return;
                }

                let guard_data =
                    BundleGuard::deserialize(&mut &guard_account.unwrap().value.unwrap().data[8..])
                        .unwrap();
                //let guard_data_clone = guard_data.clone();

                //shared values
                let should_stop_task = Arc::new(AtomicBool::new(false));

                let mut handler_ref = menu_handler_clone1.lock().await;
                display_info_page(
                    vec![
                        InfoSegment::Emphasized(String::from("Waiting for migration.")),
                        InfoSegment::Normal(String::from(
                            "-- All tokens will be sold once the coin migrates to pump swap.",
                        )),
                        InfoSegment::Normal(String::from(
                            "-- Return or continue from this page to explicitly stop.",
                        )),
                    ],
                    String::from("Migrations"),
                    &mut handler_ref,
                    Some(OptionCallback::StopMonitorTask(Arc::clone(
                        &should_stop_task,
                    ))),
                    None,
                    None,
                );

                drop(handler_ref);

                let api_task_flag_ref = Arc::clone(&should_stop_task);
                let connection_ref = Arc::clone(&connection);
                let wss_url_clone = Arc::clone(&wss_url);

                async fn sell_all_pump_dex(
                    connection: Arc<RpcClient>,
                    current_tip: Arc<AtomicU64>,
                    bundle_guard: Pubkey,
                    guard_data: BundleGuard,
                    blockhash_manager: Arc<RecentBlockhashManager>,
                    funding_wallet: Arc<Keypair>,
                    pool_keys: Arc<PumpDexKeys>,
                    menu_handler_clone2: Arc<Mutex<MenuHandler>>,
                    wallets_to_sell: Vec<Arc<Keypair>>,
                    amounts: Vec<u64>,
                    global_config: Arc<RwLock<Option<GlobalConfig>>>,
                    wss_url_clone: Arc<String>,
                ) {
                    let tip = current_tip.load(Ordering::Relaxed);
                    let global_lut = Arc::new(get_global_lut_data());

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
                    //let ray_keys = Arc::new(pool_keys.lock().await.take().unwrap());

                    tokio::spawn(async move {
                        blockhash_manager.start();
                        notify_websocket_opened_ref.notified().await;
                        let semaphore = Arc::new(Semaphore::new(100));

                        while !bundle_sender_flag.load(Ordering::Relaxed)
                            && !bundle_timeout_flag.load(Ordering::Relaxed)
                        {
                            let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();

                            let bundle_balancer_ref = Arc::clone(&bundle_balancer_ref);
                            let funder_keypair_ref = Arc::clone(&funder_keypair_ref);

                            //here we do the raydium sell all bundle
                            let bundle = get_bonded_multi_wallet_sell_bundle(
                                Arc::clone(&pool_keys),
                                Arc::clone(&blockhash_manager),
                                &wallets_to_sell,
                                &amounts,
                                Arc::clone(&funder_keypair_ref),
                                &bundle_guard,
                                guard_data.nonce,
                                tip,
                                Arc::clone(&global_lut),
                            );
                            //
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
                        0,
                    );

                    let account_monitor_flag = Arc::clone(&stop_flag);
                    let account_monitor_timeout = Arc::clone(&timeout_flag);
                    //let connection_ref = Arc::clone(&connection);
                    let notify_websocket_opened_ref = Arc::clone(&notify_websocket_opened);

                    tokio::spawn(async move {
                        let (ws_stream, _) = connect_async(wss_url_clone.as_ref()).await.unwrap();
                        let (mut write, mut read) = ws_stream.split();

                        let subscription_message = get_account_subscription_message(&bundle_guard);

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
                        handler_ref.return_to_pump_main_menu();
                        if timeout_flag.load(Ordering::Relaxed) {
                            display_bundle_timeout_error_page(&mut handler_ref);
                            //spawn_error_input_listener(menu_handler_clone1, 10);
                        } else {
                            display_info_page(
                                vec![InfoSegment::Normal(format!("Sold Everything."))],
                                String::from("Success."),
                                &mut handler_ref,
                                None,
                                None,
                                None,
                            ); //stdout_task.await;
                        }
                    });
                }

                //we will have only one periodic check every 300ms for the pool account data

                let current_tip_ref1 = Arc::clone(&current_tip);
                let blockhash_manager_ref1 = Arc::clone(&blockhash_manager);
                let funding_wallet_ref1 = Arc::clone(&funding_wallet);
                let menu_handler_clone2_ref1 = Arc::clone(&menu_handler_clone2);
                let wallets_to_sell_clone = wallets_to_sell.clone();
                let amounts_clone = amounts.clone();
                let global_config_ref1 = Arc::clone(&global_config);
                let wss_url_clone_ref1 = Arc::clone(&wss_url_clone);
                tokio::spawn(async move {
                    loop {
                        if api_task_flag_ref.load(Ordering::Relaxed) {
                            break;
                        }

                        //now im gonna fetch the account data for the pool
                        let account_data = connection.get_account_with_commitment(
                            &derived_pump_dex_keys.pool_id,
                            CommitmentConfig::processed(),
                        );

                        if matches!(account_data, Err(_) | Ok(Response { value: None, .. })) {
                            sleep(Duration::from_millis(300)).await;
                            continue;
                        }

                        api_task_flag_ref.store(true, Ordering::Relaxed);
                        //here i change the text for the info text on teh input page,

                        let mut handler_ref = menu_handler_clone1.lock().await;
                        display_info_page(
                            vec![
                                InfoSegment::Success(String::from("Pool detected!.")),
                                InfoSegment::Normal(String::from("-- Selling all tokens.")),
                                InfoSegment::Normal(String::from(
                                    "-- Return from this page to check bundle status.",
                                )),
                            ],
                            String::from("Migrations"),
                            &mut handler_ref,
                            Some(OptionCallback::StopMonitorTask(Arc::clone(
                                &should_stop_task,
                            ))),
                            None,
                            None,
                        );

                        let (curr_page, term) = handler_ref.get_current_page_and_terminal();

                        if let Page::InfoPage(info_page) = curr_page {
                            let mut stdout = stdout();
                            let _ =
                                stdout.execute(SetBackgroundColor(crossterm::style::Color::Black));
                            let _ = info_page.display(term);
                        }
                        handler_ref.return_to_pump_main_menu();
                        display_loading_page(
                            String::from("Pool found. Selling all tokens"),
                            &mut handler_ref,
                        );

                        drop(handler_ref);

                        tokio::spawn(async move {
                            sell_all_pump_dex(
                                connection_ref,
                                current_tip_ref1,
                                bundle_guard,
                                guard_data,
                                blockhash_manager_ref1,
                                funding_wallet_ref1,
                                Arc::clone(&derived_pump_dex_keys),
                                menu_handler_clone2_ref1,
                                wallets_to_sell_clone,
                                amounts_clone,
                                global_config_ref1,
                                wss_url_clone_ref1,
                            )
                            .await;
                        });

                        break;
                    }
                });
            });
        }
        OptionCallback::StopQuickSellAllBondedTask(flag) => {
            flag.store(true, Ordering::Relaxed);
            loading_handler_ref.return_to_pump_main_menu();
        }
        OptionCallback::BurnDevAll => {
            //first of all load the launch manifest
            let manifest_validation = validate_and_retrieve_launch_manifest();
            if let Err(ref e) = manifest_validation {
                display_error_page(
                    e.clone(),
                    Some(String::from("Launch manifest error")),
                    None,
                    Some(10),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 10);
                return;
            }

            let manifest = Arc::new(manifest_validation.unwrap());

            display_loading_page(
                String::from("Checking Dev Balance."),
                &mut loading_handler_ref,
            );
            drop(loading_handler_ref);

            tokio::spawn(async move {
                let creator = get_bonding_curve_creator(&manifest.mint, Arc::clone(&connection));
                if creator.is_none() {
                    let mut handler_ref = menu_handler_clone1.lock().await;
                    handler_ref.to_previous_page();
                    display_error_page(
                        String::from("Failed to fetch creator-vault address"),
                        Some(String::from("Creator Fetch Error")),
                        None,
                        Some(5),
                        &mut handler_ref,
                        false, //menu_handler_clone2
                    );
                    spawn_error_input_listener(menu_handler_clone2, 5);
                    return;
                }

                let creator_address = creator.unwrap();

                //first derive the pump keys
                let pump_keys = Arc::new(derive_all_pump_keys(
                    &dev_wallet.pubkey(),
                    manifest.mint.clone(),
                    &creator_address,
                ));

                let dev = Arc::clone(&manifest.wallet_entries.get(0).unwrap().wallet);
                let dev_ata = get_associated_token_address(&dev.pubkey(), &pump_keys.mint);
                let ata_account_data = connection.get_account(&dev_ata);

                let balance_state = {
                    if let Err(ref e) = ata_account_data {
                        TokenAccountBalanceState::DoesNotExist
                    } else {
                        let dev_account_data = ata_account_data.unwrap();
                        let account_data = Account::unpack(&dev_account_data.data);
                        if let Ok(data) = account_data {
                            let amount_held = data.amount;
                            if amount_held.is_zero() {
                                TokenAccountBalanceState::ExistsWithNoBalance
                            } else {
                                TokenAccountBalanceState::ExistsWithBalance(amount_held)
                            }
                        } else {
                            TokenAccountBalanceState::DoesNotExist
                        }
                    }
                };

                match balance_state {
                    TokenAccountBalanceState::ExistsWithNoBalance
                    | TokenAccountBalanceState::DoesNotExist => {
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        handler_ref.to_previous_page();
                        display_info_page(
                            vec![InfoSegment::Normal(format!("No balance in dev wallet",))],
                            String::from("Info"),
                            &mut handler_ref,
                            None,
                            None,
                            None,
                        ); //stdout_task.await;
                        return;
                    }
                    _ => {}
                };

                //now we send the burn bundle
                //now we just launch the operation bundle
                let mut handler_ref = menu_handler_clone1.lock().await;
                display_loading_page(
                    String::from("Burning all tokens from dev wallet"),
                    &mut handler_ref,
                );
                drop(handler_ref);
                let funder_ref = Arc::clone(&funding_wallet);
                tokio::spawn(async move {
                    let bundle_guard = get_bundle_guard(
                        &funder_ref.pubkey(),
                        &Pubkey::from_str(BOT_PROGRAM_ID).unwrap(),
                    );
                    let guard_account = connection
                        .get_account_with_commitment(&bundle_guard, CommitmentConfig::processed());

                    if let Err(ref e) = guard_account {
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        handler_ref.to_previous_page();
                        display_bundle_guard_fetch_error_page(&mut handler_ref);
                        spawn_error_input_listener(menu_handler, 10);
                        return;
                    }

                    let guard_data = BundleGuard::deserialize(
                        &mut &guard_account.unwrap().value.unwrap().data[8..],
                    )
                    .unwrap();

                    //let mut handler_ref = menu_handler_clone1.lock().await;
                    let tip = current_tip.load(Ordering::Relaxed);
                    //let global_lut = Arc::new(get_global_lut_data());

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

                    tokio::spawn(async move {
                        blockhash_manager.start();
                        notify_websocket_opened_ref.notified().await;
                        let semaphore = Arc::new(Semaphore::new(100));

                        while !bundle_sender_flag.load(Ordering::Relaxed)
                            && !bundle_timeout_flag.load(Ordering::Relaxed)
                        {
                            let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();

                            let bundle_balancer_ref = Arc::clone(&bundle_balancer_ref);
                            let funder_keypair_ref = Arc::clone(&funder_keypair_ref);

                            let bundle = get_burn_or_retrieve_tokens_bundle(
                                Arc::clone(&blockhash_manager),
                                &pump_keys.mint,
                                funder_keypair_ref,
                                &vec![Arc::clone(&dev)],
                                dev.pubkey(),
                                &bundle_guard,
                                guard_data.nonce,
                                &vec![balance_state],
                                tip,
                                true,
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
                        0,
                    );

                    let account_monitor_flag = Arc::clone(&stop_flag);
                    let account_monitor_timeout = Arc::clone(&timeout_flag);
                    let connection_ref = Arc::clone(&connection);
                    let notify_websocket_opened_ref = Arc::clone(&notify_websocket_opened);

                    tokio::spawn(async move {
                        let (ws_stream, _) = connect_async(wss_url.as_ref()).await.unwrap();
                        let (mut write, mut read) = ws_stream.split();

                        let subscription_message = get_account_subscription_message(&bundle_guard);

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
                                    continue;
                                }
                            }
                        }
                        let mut handler_ref = menu_handler_clone2.lock().await;
                        handler_ref.to_previous_page();
                        handler_ref.to_previous_page();
                        handler_ref.to_previous_page();
                        if timeout_flag.load(Ordering::Relaxed) {
                            display_bundle_timeout_error_page(&mut handler_ref);
                            //spawn_error_input_listener(menu_handler_clone1, 10);
                        } else {
                            display_info_page(
                                vec![InfoSegment::Normal(format!("All tokens burnt."))],
                                String::from("Success."),
                                &mut handler_ref,
                                None,
                                None,
                                None,
                            ); //stdout_task.await;
                        }
                    });
                });
            });
        }

        _ => {}
    }
}
