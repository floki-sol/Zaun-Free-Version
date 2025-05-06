use std::{
    fs,
    path::Path,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
//use strip_ansi_escapes::strip;

use borsh::BorshDeserialize;
use futures::{SinkExt, StreamExt};
use log::info;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, native_token::LAMPORTS_PER_SOL, program_pack::Pack,
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_program,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Account;
use tokio::{
    sync::{Mutex, Notify, RwLock, Semaphore},
    time::sleep,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    cli::{
        error::{
            display_balance_fetch_error_page, display_bundle_guard_fetch_error_page,
            display_bundle_timeout_error_page, display_error_page,
            display_insufficient_funds_error_page, spawn_error_input_listener,
        },
        info::{display_info_page, InfoSegment},
        input::{display_input_page, InputType},
        loading_indicator::display_loading_page,
        menu::MenuHandler,
        options::OptionCallback,
        pages::pump::main_menu::wallet_management::{
            wallet_cleanup::page::get_wallet_cleanup_page,
            wallet_funding::showcase_funding_page::display_funding_showcase_page,
            wallet_generation::wallet_backup_choice::get_choose_backup_page,
        },
    },
    constants::general::{
        BundleGuard, TokenAccountBalanceState, BOT_PROGRAM_ID, PUMP_TOKEN_DECIMALS,
    },
    jito::bundles::{simulate_bundle, BundleSenderBalancer},
    loaders::{
        bundle_wallets_loader::validate_and_retrieve_pump_bundler_keypairs,
        global_config_loader::GlobalConfig,
    },
    utils::{
        backups::{
            backup_files, get_bundle_wallet_backups, restore_bundle_wallet_backup, BackupType,
        },
        blockhash_manager::RecentBlockhashManager,
        bundle_factory::{
            get_burn_or_retrieve_tokens_bundle, get_fund_wallet_bundle, get_fund_wallets_bundle,
            get_retrieve_sol_bundle,
        },
        misc::{
            adjust_file_path, create_funding_manifest, extract_lamports,
            get_account_subscription_message, get_balance, get_balances_for_wallets,
            get_human_readable_amounts, get_random_normalized_amounts, get_random_range_amounts,
            get_token_balances_for_wallets, is_pump_token, is_valid_decimal_string,
            is_valid_small_integer_string, parse_token_address, spawn_bundle_timeout_task,
            write_pump_keypairs_to_bundler_file, WalletType, WalletsFundingType,
        },
        pdas::get_bundle_guard,
        pump_helpers::fetch_pump_token_general_data,
    },
};

pub async fn invoke_wallet_management_callback(
    menu_handler: Arc<Mutex<MenuHandler>>,
    connection: Arc<RpcClient>,
    wss_url: Arc<String>,
    blockhash_manager: Arc<RecentBlockhashManager>,
    callback: OptionCallback,
    dev_wallet: Arc<Keypair>,
    funding_wallet: Arc<Keypair>,
    global_config: Arc<RwLock<Option<GlobalConfig>>>,
    current_tip: Arc<AtomicU64>,
    _capsolver_api_key: Arc<String>,
) {
    let mut loading_handler_ref = menu_handler.lock().await;

    let menu_handler_clone1 = Arc::clone(&menu_handler);
    let menu_handler_clone2 = Arc::clone(&menu_handler);
    let _menu_handler_clone3 = Arc::clone(&menu_handler);

    match callback {
        //wallet management

        //balance check
        OptionCallback::BalanceCheckerCallback((ref wallet_type, data)) => match wallet_type {
            WalletType::BundleWalletSol => {
                let bundle_wallets_validation = validate_and_retrieve_pump_bundler_keypairs(
                    &funding_wallet.pubkey(),
                    &dev_wallet.pubkey(),
                );
                if let Ok(bundle_wallets) = bundle_wallets_validation {
                    display_loading_page(
                        String::from("Fetching Bundle Wallets Balance"),
                        &mut loading_handler_ref,
                    );
                    drop(loading_handler_ref);

                    let arced_vec = Arc::new(bundle_wallets);
                    let balance_task_vec_ref = Arc::clone(&arced_vec);

                    tokio::spawn(async move {
                        let fetch_res =
                            get_balances_for_wallets(Arc::clone(&connection), balance_task_vec_ref)
                                .await;
                        let mut handler_ref = menu_handler_clone1.lock().await;

                        match fetch_res {
                            Ok(lamports_array) => {
                                // Convert lamports to SOL

                                let sol_balances_array = lamports_array
                                    .iter()
                                    .map(|lamports| {
                                        format!("{:.3} SOL", *lamports as f64 / 1_000_000_000.0)
                                    })
                                    .collect::<Vec<String>>();

                                let total_sol: u64 = lamports_array.iter().sum();
                                //                                let sol_balance = lamports as f64 / 1_000_000_000.0;
                                //let sol_balance_str = format!("{:.3} SOL", sol_balance); // Format to 3 decimal places

                                let mut info_segment_entries: Vec<InfoSegment> = vec![
                                    InfoSegment::Normal(format!(
                                        "Showing balances of {} wallets:",
                                        sol_balances_array.len()
                                    )),
                                    InfoSegment::NumericSplitInfo((
                                        String::from("-- Total Balance"),
                                        format!(
                                            "{}",
                                            format!(
                                                "{:.3} SOL",
                                                total_sol as f64 / 1_000_000_000.0
                                            )
                                        ),
                                    )),
                                ];
                                for (idx, bs58_keypair) in arced_vec.iter().enumerate() {
                                    let decoded_bytes =
                                        bs58::decode(bs58_keypair.clone()).into_vec().unwrap();
                                    let keypair = Keypair::from_bytes(&decoded_bytes).unwrap();

                                    info_segment_entries.push(InfoSegment::Normal("".to_string()));
                                    info_segment_entries
                                        .push(InfoSegment::Normal(format!("Wallet {}", idx + 1)));
                                    info_segment_entries.push(InfoSegment::StringSplitInfo((
                                        String::from("-- Address: "),
                                        keypair.pubkey().to_string(),
                                    )));
                                    info_segment_entries.push(InfoSegment::NumericSplitInfo((
                                        String::from("-- Balance: "),
                                        sol_balances_array[idx].clone(),
                                    )));
                                }

                                handler_ref.to_previous_page();
                                display_info_page(
                                    info_segment_entries,
                                    String::from("Wallet Balances"),
                                    &mut handler_ref,
                                    None,
                                    None,
                                    Some(17),
                                );
                            }
                            Err(e) => {
                                handler_ref.to_previous_page();
                                display_balance_fetch_error_page(&mut handler_ref);
                                spawn_error_input_listener(menu_handler_clone2, 10);
                            }
                        }
                    });
                } else if let Err(loading_err) = bundle_wallets_validation {
                    //loading_handler_ref.to_previous_page();
                    display_error_page(
                        loading_err,
                        Some(String::from("Wallet Configuration Error")),
                        None,
                        Some(5),
                        &mut loading_handler_ref,
                        false, //menu_handler_clone2
                    );

                    spawn_error_input_listener(menu_handler_clone2, 5);
                }
            }
            WalletType::BundleWalletTokens => {
                let bundle_wallets_validation = validate_and_retrieve_pump_bundler_keypairs(
                    &funding_wallet.pubkey(),
                    &dev_wallet.pubkey(),
                );
                if let Ok(bundle_wallets) = bundle_wallets_validation {
                    let token_addy = parse_token_address(data.as_str()).unwrap_or_default();
                    let token_pubkey = Pubkey::from_str(token_addy.as_str());
                    if let Ok(key) = token_pubkey {
                        display_loading_page(
                            String::from("Checking Inputted Token Validity"),
                            &mut loading_handler_ref,
                        );
                        drop(loading_handler_ref);

                        tokio::spawn(async move {
                            let pump_token_validation =
                                is_pump_token(Arc::clone(&connection), &key).await;
                            let mut handler_ref = menu_handler_clone1.lock().await;

                            if let Ok(_) = pump_token_validation {
                                handler_ref.to_previous_page();
                                display_loading_page(
                                    String::from("Fetching Bundle Wallets Token Balance"),
                                    &mut handler_ref,
                                );
                                drop(handler_ref);

                                let arced_vec = Arc::new(bundle_wallets);
                                let balance_task_vec_ref = Arc::clone(&arced_vec);

                                tokio::spawn(async move {
                                    let fetch_res = get_token_balances_for_wallets(
                                        Arc::clone(&connection),
                                        balance_task_vec_ref,
                                        key,
                                    )
                                    .await;

                                    let token_data = fetch_pump_token_general_data(&key).await;
                                    let token_symbol = if let Ok(data) = token_data {
                                        format!("${}", data.symbol)
                                    } else {
                                        String::from("$???")
                                    };

                                    let mut handler_ref = menu_handler_clone1.lock().await;

                                    match fetch_res {
                                        Ok(token_balance_array) => {
                                            // Convert lamports to SOL

                                            let token_balances_array = token_balance_array
                                                .iter()
                                                .map(|raw_balance| {
                                                    format!(
                                                        "{:.3} {}", // Shows 3 decimal places
                                                        *raw_balance as f64
                                                            / 10u64.pow(PUMP_TOKEN_DECIMALS as u32)
                                                                as f64, // Divide by 10^6 for 6 decimals,
                                                        token_symbol
                                                    )
                                                })
                                                .collect::<Vec<String>>();
                                            //                                let sol_balance = lamports as f64 / 1_000_000_000.0;
                                            //let sol_balance_str = format!("{:.3} SOL", sol_balance); // Format to 3 decimal places

                                            let mut info_segment_entries: Vec<InfoSegment> =
                                                vec![InfoSegment::Normal(format!(
                                                    "Showing token_symbol balances for {} wallets:",
                                                    token_balance_array.len()
                                                ))];
                                            for (idx, bs58_keypair) in arced_vec.iter().enumerate()
                                            {
                                                let decoded_bytes =
                                                    bs58::decode(bs58_keypair.clone())
                                                        .into_vec()
                                                        .unwrap();
                                                let keypair =
                                                    Keypair::from_bytes(&decoded_bytes).unwrap();

                                                info_segment_entries
                                                    .push(InfoSegment::Normal("".to_string()));
                                                info_segment_entries.push(InfoSegment::Normal(
                                                    format!("Wallet {}", idx + 1),
                                                ));
                                                info_segment_entries.push(
                                                    InfoSegment::StringSplitInfo((
                                                        String::from("-- Address: "),
                                                        keypair.pubkey().to_string(),
                                                    )),
                                                );
                                                info_segment_entries.push(
                                                    InfoSegment::NumericSplitInfo((
                                                        String::from("-- Balance: "),
                                                        token_balances_array[idx].clone(),
                                                    )),
                                                );
                                            }

                                            handler_ref.to_previous_page();
                                            handler_ref.to_previous_page();
                                            display_info_page(
                                                info_segment_entries,
                                                String::from("Wallet Balances"),
                                                &mut handler_ref,
                                                None,
                                                None,
                                                Some(17),
                                            );
                                        }
                                        Err(e) => {
                                            handler_ref.to_previous_page();
                                            display_balance_fetch_error_page(&mut handler_ref);
                                            spawn_error_input_listener(menu_handler_clone2, 10);
                                        }
                                    }
                                });
                            } else if let Err(e) = pump_token_validation {
                                handler_ref.to_previous_page();
                                display_error_page(
                                    e,
                                    Some(String::from("Input Error")),
                                    None,
                                    Some(5),
                                    &mut handler_ref,
                                    false, //menu_handler_clone2
                                );
                                spawn_error_input_listener(menu_handler_clone2, 5);
                            }
                        });
                    } else if let Err(e) = token_pubkey {
                        //loading_handler_ref.to_previous_page();
                        display_error_page(
                            "Invalid Publickey provided.".to_string(),
                            Some(String::from("Input Error")),
                            None,
                            Some(5),
                            &mut loading_handler_ref,
                            false, //menu_handler_clone2
                        );
                        spawn_error_input_listener(menu_handler_clone2, 5);
                    }
                } else if let Err(loading_err) = bundle_wallets_validation {
                    //loading_handler_ref.to_previous_page();
                    display_error_page(
                        loading_err,
                        Some(String::from("Wallet Configuration Error")),
                        None,
                        Some(5),
                        &mut loading_handler_ref,
                        false, //menu_handler_clone2
                    );

                    spawn_error_input_listener(menu_handler_clone2, 5);
                }
            }
            _single_wallet => {
                let wallet_to_check: Result<(Pubkey, String), String> = match wallet_type {
                    WalletType::DevWallet => Ok((dev_wallet.pubkey(), String::from("Dev"))),
                    WalletType::FundingWallet => {
                        Ok((funding_wallet.pubkey(), String::from("Funder")))
                    }
                    WalletType::BumpWallet => Ok((funding_wallet.pubkey(), String::from("Funder"))),
                    WalletType::Another(_) => {
                        let pub_key_res = Pubkey::from_str(&data);
                        if let Ok(pkey) = pub_key_res {
                            Ok((pkey, String::from("External")))
                        } else {
                            Err(format!(
                                "External wallet input err: {}",
                                pub_key_res.err().unwrap().to_string()
                            ))
                        }
                    }
                    _ => Ok((funding_wallet.pubkey(), String::from("Funder"))),
                };

                if let Ok((wallet, label)) = wallet_to_check {
                    display_loading_page(
                        format!("Fetching {label} Balance"),
                        &mut loading_handler_ref,
                    );
                    drop(loading_handler_ref);

                    tokio::spawn(async move {
                        let fetch_res = get_balance(Arc::clone(&connection), &wallet).await;
                        let mut handler_ref = menu_handler_clone1.lock().await;

                        match fetch_res {
                            Ok(lamports) => {
                                // Convert lamports to SOL
                                let sol_balance = lamports as f64 / 1_000_000_000.0;
                                let sol_balance_str = format!("{:.3} SOL", sol_balance); // Format to 3 decimal places

                                handler_ref.to_previous_page();
                                display_info_page(
                                    vec![
                                        InfoSegment::StringSplitInfo((
                                            String::from("Address"),
                                            wallet.to_string(),
                                        )),
                                        InfoSegment::StringSplitInfo((
                                            String::from("Sol Balance"),
                                            sol_balance_str,
                                        )),
                                    ],
                                    format!("{label} wallet"),
                                    &mut handler_ref,
                                    None,
                                    None,
                                    None,
                                );
                            }
                            Err(e) => {
                                handler_ref.to_previous_page();
                                display_balance_fetch_error_page(&mut handler_ref);
                                spawn_error_input_listener(menu_handler_clone2, 10);
                            }
                        }
                    });
                } else if let Err(e) = wallet_to_check {
                    display_error_page(
                        e,
                        Some(String::from("wallet balance check error")),
                        None,
                        Some(10),
                        &mut loading_handler_ref,
                        false, //menu_handler_clone2
                    );
                    spawn_error_input_listener(menu_handler_clone2, 10);
                }
            }
        },

        //wallet generation
        OptionCallback::RetrieveWalletsFromBackupsCallback => {
            let backup_wallets_validation = get_bundle_wallet_backups();

            if let Ok(backup_wallets) = backup_wallets_validation {
                if backup_wallets.is_empty() {
                    //loading_handler_ref.to_previous_page();
                    display_error_page(
                        String::from("No previously generated wallet backups found"),
                        Some(String::from("Wallet Generation Error")),
                        None,
                        Some(5),
                        &mut loading_handler_ref,
                        false, //menu_handler_clone2
                    );
                    spawn_error_input_listener(menu_handler_clone2, 5);
                } else {
                    let backups_page =
                        get_choose_backup_page(&mut loading_handler_ref, backup_wallets);
                    loading_handler_ref.change_page(backups_page);
                }
            } else if let Err(e) = backup_wallets_validation {
                //loading_handler_ref.to_previous_page();
                display_error_page(
                    e,
                    Some(String::from("Wallet Generation Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
            }
        }
        OptionCallback::ConfirmRetrieveBackup(buf) => {
            let restore_res = restore_bundle_wallet_backup(&buf);

            loading_handler_ref.to_previous_page();
            if let Ok(_) = restore_res {
                display_info_page(
                    vec![
                        InfoSegment::Normal(String::from("Old backup restored")),
                        InfoSegment::Normal(String::from("")),
                        InfoSegment::Emphasized(String::from(
                            "-- Current configured wallets backed-up.",
                        )),
                    ],
                    String::from("Success."),
                    &mut loading_handler_ref,
                    None,
                    None,
                    None,
                ); //stdout_task.await;
            } else if let Err(e) = restore_res {
                display_error_page(
                    e,
                    Some(String::from("Backup Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
            }
        }
        OptionCallback::BundlerWalletGenerationMenuCallBack => {
            let file_path = &adjust_file_path("configurations/pump/bundler-wallets.json");
            let path: &Path = Path::new(file_path);

            loading_handler_ref.to_previous_page();

            match fs::metadata(path) {
                Ok(_) => {
                    display_info_page(
                        vec![
                            InfoSegment::Emphasized(String::from("Important!")),
                            InfoSegment::Normal(String::from(
                                "You can generate up to 20 wallets for bundling.",
                            )),
                            InfoSegment::Normal(String::from("")),
                            InfoSegment::Normal(String::from("They will be saved under:")),
                            InfoSegment::Emphasized(String::from(&adjust_file_path(
                                "=> 'configurations/pump/bundler-wallets.json'",
                            ))),
                            InfoSegment::Normal(String::from("")),
                            InfoSegment::Warning(String::from("Warning!")),
                            InfoSegment::Warning(String::from(
                                "-- Destructive Action and non-reversible.",
                            )),
                            InfoSegment::Warning(String::from(
                                "-- Make sure that all currently configured wallets are empty. ",
                            )),
                        ],
                        String::from("Generation Details."),
                        &mut loading_handler_ref,
                        Some(OptionCallback::GenerateBundlerWalletsCallback(false)),
                        None,
                        None,
                    ); //stdout_task.await;
                } // File exists
                Err(_) => {
                    display_info_page(
                        vec![
                            InfoSegment::Normal(String::from(
                                "No previously generated wallets detected!",
                            )),
                            InfoSegment::Normal(String::from("")),
                            InfoSegment::Normal(String::from("generating new wallets file under:")),
                            InfoSegment::Emphasized(String::from(&adjust_file_path(
                                "=> 'configurations/pump/bundler-wallets.json'",
                            ))),
                        ],
                        String::from("First time Setup."),
                        &mut loading_handler_ref,
                        Some(OptionCallback::GenerateBundlerWalletsCallback(true)),
                        None,
                        None,
                    ); //stdout_task.await;
                } // File doesn't exist
            };
        }
        OptionCallback::GenerateBundlerWalletsCallback(is_new) => {
            loading_handler_ref.to_previous_page();
            display_input_page(
                vec![
                    InfoSegment::Normal(String::from(
                        "Enter how many wallets you want to generate below.",
                    )),
                    InfoSegment::StringSplitInfo((String::from("-- Minimum"), String::from("1"))),
                    InfoSegment::StringSplitInfo((String::from("-- Maximum"), String::from("20"))),
                ],
                String::from("Generation Details"),
                &mut loading_handler_ref,
                Some(OptionCallback::ConfirmGenerationInput(String::new())),
                None,
                InputType::WholeNumber,
            );
        }
        OptionCallback::ConfirmGenerationInput(data) => {
            // Attempt to parse the string into an integer
            let is_valid = is_valid_small_integer_string(data, 1, 20);

            if let Err(error) = is_valid {
                //loading_handler_ref.to_previous_page();
                display_error_page(
                    error,
                    Some(String::from("Wallet Generation Input Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );

                spawn_error_input_listener(menu_handler_clone2, 5);
            } else if let Ok(amount) = is_valid {
                let _ = backup_files(BackupType::BundleWallets);
                let _ = write_pump_keypairs_to_bundler_file(amount as usize);
                //loading_handler_ref.to_previous_page();

                loading_handler_ref.to_previous_page();
                display_info_page(
                    vec![
                        InfoSegment::Emphasized(String::from("Success!")),
                        InfoSegment::Normal(format!("generated {} new keypairs. ", amount)),
                    ],
                    String::from("Generation Result."),
                    &mut loading_handler_ref,
                    None,
                    None,
                    None,
                ); //stdout_task.await;
            }
        }

        //wallet funding
        OptionCallback::FundSingleWallet((ref wallet_type, data)) => {
            let wallet_to_fund: Result<(Pubkey, String), String> = match wallet_type {
                WalletType::DevWallet => Ok((dev_wallet.pubkey(), String::from("Dev"))),
                WalletType::BumpWallet => Ok((dev_wallet.pubkey(), String::from("Dev"))),
                _ => Ok((funding_wallet.pubkey(), String::from("Funder"))),
            };

            if let Ok((wallet, label)) = wallet_to_fund {
                let is_valid = is_valid_decimal_string(data, 0.001, 100.0);
                if let Err(error) = is_valid {
                    //loading_handler_ref.to_previous_page();
                    display_error_page(
                        error,
                        Some(String::from("Funding Error")),
                        None,
                        Some(5),
                        &mut loading_handler_ref,
                        false, //menu_handler_clone2
                    );

                    spawn_error_input_listener(menu_handler_clone2, 5);
                } else if let Ok(val) = is_valid {
                    display_loading_page(
                        String::from("Checking Funder balance."),
                        &mut loading_handler_ref,
                    );
                    drop(loading_handler_ref);

                    let connection_ref = Arc::clone(&connection);
                    let funder_ref = Arc::clone(&funding_wallet);
                    tokio::spawn(async move {
                        let funder_balance_res =
                            get_balance(connection_ref, &funder_ref.pubkey()).await;
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
                            return;
                        }

                        let guard_data = BundleGuard::deserialize(
                            &mut &guard_account.unwrap().value.unwrap().data[8..],
                        )
                        .unwrap();

                        let mut handler_ref = menu_handler_clone1.lock().await;
                        match funder_balance_res {
                            Ok(funder_balance) => {
                                let tip = current_tip.load(Ordering::Relaxed);
                                let amount_to_send = (val * LAMPORTS_PER_SOL as f64) as u64;
                                let transaction_fees = (0.000005 * LAMPORTS_PER_SOL as f64) as u64;

                                if funder_balance <= tip + amount_to_send + transaction_fees {
                                    handler_ref.to_previous_page();
                                    display_insufficient_funds_error_page(&mut handler_ref);
                                    spawn_error_input_listener(menu_handler_clone2, 10);
                                } else {
                                    display_loading_page(
                                        String::from("Funding wallet."),
                                        &mut handler_ref,
                                    );

                                    drop(handler_ref);

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
                                    let notify_websocket_opened_ref =
                                        Arc::clone(&notify_websocket_opened);

                                    tokio::spawn(async move {
                                        blockhash_manager.start();
                                        notify_websocket_opened_ref.notified().await;
                                        let semaphore = Arc::new(Semaphore::new(100));
                                        while !bundle_sender_flag.load(Ordering::Relaxed)
                                            && !bundle_timeout_flag.load(Ordering::Relaxed)
                                        {
                                            let permit = Arc::clone(&semaphore)
                                                .acquire_owned()
                                                .await
                                                .unwrap();

                                            let bundle_balancer_ref =
                                                Arc::clone(&bundle_balancer_ref);
                                            let funder_keypair_ref =
                                                Arc::clone(&funder_keypair_ref);
                                            let bundle = get_fund_wallet_bundle(
                                                Arc::clone(&blockhash_manager),
                                                Arc::clone(&funder_keypair_ref),
                                                wallet,
                                                &bundle_guard,
                                                guard_data.nonce,
                                                amount_to_send,
                                                tip,
                                            );

                                            //simulate_bundle(bundle, true).await;
                                            tokio::spawn(async move {
                                                let _ =
                                                    bundle_balancer_ref.send_bundle(bundle).await;
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
                                    //let funder_ref = Arc::clone(&funding_wallet);
                                    let notify_websocket_opened_ref =
                                        Arc::clone(&notify_websocket_opened);

                                    tokio::spawn(async move {
                                        let (ws_stream, _) =
                                            connect_async(wss_url.as_ref()).await.unwrap();
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
                                        handler_ref.to_previous_page();

                                        //info!("reached post operation");
                                        if account_monitor_timeout.load(Ordering::Relaxed) {
                                            info!("bundle timed out");
                                            display_bundle_timeout_error_page(&mut handler_ref);
                                            //spawn_error_input_listener(menu_handler_clone1, 10);
                                        } else {
                                            display_info_page(
                                                vec![InfoSegment::Normal(format!(
                                                    "Funded {label} Wallet."
                                                ))],
                                                String::from("Success."),
                                                &mut handler_ref,
                                                None,
                                                None,
                                                None,
                                            ); //stdout_task.await;
                                        }
                                    });
                                };
                            }
                            Err(e) => {
                                handler_ref.to_previous_page();
                                display_balance_fetch_error_page(&mut handler_ref);
                                spawn_error_input_listener(menu_handler_clone2, 10);
                            }
                        }
                    });
                }
            } else if let Err(e) = wallet_to_fund {
                display_error_page(
                    e,
                    Some(String::from("wallet funding error")),
                    None,
                    Some(10),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 10);
            }
        }

        OptionCallback::FundBundleWallets(wallet_funding_type) => {
            let bundle_wallets_validation = validate_and_retrieve_pump_bundler_keypairs(
                &funding_wallet.pubkey(),
                &dev_wallet.pubkey(),
            );
            let mut global_config_lock = global_config.write().await; // Get the write lock

            let mut funding_type: String = String::from("");
            if let Some(config_ref) = global_config_lock.as_mut() {
                funding_type = config_ref.funding_strategy.clone();
            };
            drop(global_config_lock);
            if let Ok(wallets) = bundle_wallets_validation {
                match wallet_funding_type {
                    WalletsFundingType::Static(amount) => {
                        let mut amounts_vec: Vec<f64> = vec![f64::default(); wallets.len()];
                        let is_valid = is_valid_decimal_string(amount, 0.001, 100.0);

                        if let Err(error) = is_valid {
                            display_error_page(
                                error,
                                Some(String::from("Funding Error")),
                                None,
                                Some(5),
                                &mut loading_handler_ref,
                                false, //menu_handler_clone2
                            );
                            spawn_error_input_listener(menu_handler_clone2, 5);
                        } else if let Ok(val) = is_valid {
                            amounts_vec.fill(val);

                            display_funding_showcase_page(
                                wallets,
                                amounts_vec,
                                &mut loading_handler_ref,
                                funding_type,
                            );
                        }
                    }
                    WalletsFundingType::Distribution(amount) => {
                        let is_valid = is_valid_decimal_string(
                            amount,
                            0.001 * wallets.len() as f64,
                            100.0 * wallets.len() as f64,
                        );
                        if let Err(error) = is_valid {
                            display_error_page(
                                error,
                                Some(String::from("Funding Error")),
                                None,
                                Some(5),
                                &mut loading_handler_ref,
                                false, //menu_handler_clone2
                            );
                            spawn_error_input_listener(menu_handler_clone2, 5);
                        } else if let Ok(val) = is_valid {
                            let distributed_amounts =
                                get_random_normalized_amounts(wallets.len(), val, 0.001 as f64);

                            if let Err(error) = distributed_amounts {
                                display_error_page(
                                    error,
                                    Some(String::from("Funding Error")),
                                    None,
                                    Some(5),
                                    &mut loading_handler_ref,
                                    false, //menu_handler_clone2
                                );
                                spawn_error_input_listener(menu_handler_clone2, 5);
                            } else if let Ok(amounts) = distributed_amounts {
                                display_funding_showcase_page(
                                    wallets,
                                    amounts,
                                    &mut loading_handler_ref,
                                    funding_type,
                                );
                            }
                        }
                    }
                    WalletsFundingType::HumanLikeDistribution(amount) => {
                        let is_valid = is_valid_decimal_string(
                            amount,
                            1.0 * wallets.len() as f64,
                            100.0 * wallets.len() as f64,
                        );
                        if let Err(error) = is_valid {
                            display_error_page(
                                error,
                                Some(String::from("Funding Error")),
                                None,
                                Some(5),
                                &mut loading_handler_ref,
                                false, //menu_handler_clone2
                            );
                            spawn_error_input_listener(menu_handler_clone2, 5);
                        } else if let Ok(val) = is_valid {
                            let distributed_amounts =
                                get_human_readable_amounts(wallets.len(), val, 1.0 as f64);
                            if let Err(error) = distributed_amounts {
                                display_error_page(
                                    error,
                                    Some(String::from("Funding Error")),
                                    None,
                                    Some(5),
                                    &mut loading_handler_ref,
                                    false, //menu_handler_clone2
                                );
                                spawn_error_input_listener(menu_handler_clone2, 5);
                            } else if let Ok(amounts) = distributed_amounts {
                                display_funding_showcase_page(
                                    wallets,
                                    amounts,
                                    &mut loading_handler_ref,
                                    funding_type,
                                );
                            }
                        }
                    }
                    WalletsFundingType::MinMax(min_and_max) => {
                        let parts: Vec<&str> = min_and_max.split('-').collect();
                        let validation: Result<(f64, f64), String> = if parts.len() != 2 {
                            Err(String::from("invalid min-max input."))
                        } else {
                            let first_part_validation =
                                is_valid_decimal_string(parts[0].to_string(), 0.001, 100.0);
                            let second_part_validation =
                                is_valid_decimal_string(parts[1].to_string(), 0.001, 100.0);

                            if let Err(err) = first_part_validation {
                                Err(format!("Min input error: {}", err))
                            } else if let Err(err) = second_part_validation {
                                Err(format!("Max input error: {}", err))
                            } else {
                                let min = first_part_validation.ok().unwrap();
                                let max = second_part_validation.ok().unwrap();

                                let min_max_validtion = if min >= max {
                                    Err(String::from("min value cannot be >= max "))
                                } else {
                                    Ok((min, max))
                                };
                                min_max_validtion
                            }
                        };

                        if let Err(error) = validation {
                            display_error_page(
                                error,
                                Some(String::from("Funding Error")),
                                None,
                                Some(5),
                                &mut loading_handler_ref,
                                false, //menu_handler_clone2
                            );
                            spawn_error_input_listener(menu_handler_clone2, 5);
                        } else if let Ok((min, max)) = validation {
                            let distributed_amounts =
                                get_random_range_amounts(wallets.len(), min, max);

                            if let Err(error) = distributed_amounts {
                                display_error_page(
                                    error,
                                    Some(String::from("Funding Error")),
                                    None,
                                    Some(5),
                                    &mut loading_handler_ref,
                                    false, //menu_handler_clone2
                                );
                                spawn_error_input_listener(menu_handler_clone2, 5);
                            } else if let Ok(amounts) = distributed_amounts {
                                display_funding_showcase_page(
                                    wallets,
                                    amounts,
                                    &mut loading_handler_ref,
                                    funding_type,
                                );
                            }
                        }
                    }
                    WalletsFundingType::Interactive => {}
                    WalletsFundingType::Initiate(amounts) => {
                        let mut global_config_lock = global_config.write().await; // Get the write lock

                        let mut funding_type: String = String::from("");
                        if let Some(config_ref) = global_config_lock.as_mut() {
                            funding_type = config_ref.funding_strategy.clone();
                        }
                        drop(global_config_lock);
                        match funding_type.as_str() {
                            "pre-fund" => {
                                display_loading_page(
                                    String::from("Checking Funder balance."),
                                    &mut loading_handler_ref,
                                );
                                drop(loading_handler_ref);

                                let connection_ref = Arc::clone(&connection);
                                //let temp_connection_ref = Arc::clone(&connection);
                                let funder_ref = Arc::clone(&funding_wallet);
                                let pubkeys_vec = wallets
                                    .iter()
                                    .map(|pkey| Keypair::from_base58_string(&pkey).pubkey())
                                    .collect();

                                tokio::spawn(async move {
                                    let funder_balance_res =
                                        get_balance(connection_ref, &funder_ref.pubkey()).await;
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
                                        return;
                                    }
                                    let guard_data = BundleGuard::deserialize(
                                        &mut &guard_account.unwrap().value.unwrap().data[8..],
                                    )
                                    .unwrap();

                                    let mut handler_ref = menu_handler_clone1.lock().await;
                                    match funder_balance_res {
                                        Ok(funder_balance) => {
                                            // Convert lamports to SOL
                                            //let sol_balance = lamports as f64 / 1_000_000_000.0;

                                            let total_amount_to_send = (amounts.iter().sum::<f64>()
                                                * LAMPORTS_PER_SOL as f64)
                                                as u64;

                                            let tip = current_tip.load(Ordering::Relaxed);
                                            let num_groups = (amounts.len() + 5 - 1) / 5;

                                            let transaction_fees = (0.00005
                                                * (num_groups as f64)
                                                * LAMPORTS_PER_SOL as f64)
                                                as u64;

                                            if funder_balance
                                                <= tip + total_amount_to_send + transaction_fees
                                            {
                                                handler_ref.to_previous_page();
                                                display_insufficient_funds_error_page(
                                                    &mut handler_ref,
                                                );

                                                spawn_error_input_listener(menu_handler_clone2, 10);
                                            } else {
                                                display_loading_page(
                                                    String::from("Funding bundle wallets."),
                                                    &mut handler_ref,
                                                );
                                                drop(handler_ref);

                                                //here we perform the task of spamming bundles till success.
                                                let bundle_balancer =
                                                    Arc::new(BundleSenderBalancer::new());
                                                let stop_flag = Arc::new(AtomicBool::new(false));
                                                let timeout_flag = Arc::new(AtomicBool::new(false));
                                                // Task for sending bundles
                                                let bundle_sender_flag = Arc::clone(&stop_flag);
                                                let bundle_timeout_flag = Arc::clone(&timeout_flag);
                                                let bundle_balancer_ref =
                                                    Arc::clone(&bundle_balancer);
                                                let funder_keypair_ref =
                                                    Arc::clone(&funding_wallet);
                                                let notify_websocket_opened =
                                                    Arc::new(Notify::new());
                                                let notify_websocket_opened_ref =
                                                    Arc::clone(&notify_websocket_opened);

                                                tokio::spawn(async move {
                                                    blockhash_manager.start();
                                                    notify_websocket_opened_ref.notified().await;
                                                    let semaphore: Arc<Semaphore> =
                                                        Arc::new(Semaphore::new(100));

                                                    while !bundle_sender_flag
                                                        .load(Ordering::Relaxed)
                                                        && !bundle_timeout_flag
                                                            .load(Ordering::Relaxed)
                                                    {
                                                        let permit = Arc::clone(&semaphore)
                                                            .acquire_owned()
                                                            .await
                                                            .unwrap();

                                                        let bundle_balancer_ref =
                                                            Arc::clone(&bundle_balancer_ref);
                                                        let funder_keypair_ref =
                                                            Arc::clone(&funder_keypair_ref);
                                                        //info!("{:?}", amounts);
                                                        let bundle = get_fund_wallets_bundle(
                                                            Arc::clone(&blockhash_manager),
                                                            Arc::clone(&funder_keypair_ref),
                                                            &pubkeys_vec,
                                                            &amounts,
                                                            &bundle_guard,
                                                            guard_data.nonce,
                                                            tip,
                                                        );

                                                        //simulate_bundle(bundle, true).await;
                                                        tokio::spawn(async move {
                                                            let _ = bundle_balancer_ref
                                                                .send_bundle(bundle)
                                                                .await;
                                                            drop(permit);
                                                        });
                                                        sleep(Duration::from_millis(50)).await;
                                                    }
                                                    blockhash_manager.stop();
                                                });

                                                spawn_bundle_timeout_task(
                                                    Arc::clone(&global_config),
                                                    Arc::clone(&timeout_flag),
                                                    0,
                                                );

                                                let account_monitor_flag = Arc::clone(&stop_flag);
                                                let account_monitor_timeout =
                                                    Arc::clone(&timeout_flag);
                                                let connection_ref = Arc::clone(&connection);
                                                //let funder_ref = Arc::clone(&funding_wallet);
                                                let notify_websocket_opened_ref =
                                                    Arc::clone(&notify_websocket_opened);

                                                tokio::spawn(async move {
                                                    let (ws_stream, _) =
                                                        connect_async(wss_url.as_ref())
                                                            .await
                                                            .unwrap();
                                                    let (mut write, mut read) = ws_stream.split();

                                                    let subscription_message =
                                                        get_account_subscription_message(
                                                            &bundle_guard,
                                                        );

                                                    write
                                                        .send(Message::Text(
                                                            subscription_message.to_string(),
                                                        ))
                                                        .await
                                                        .unwrap();
                                                    notify_websocket_opened_ref.notify_one();
                                                    loop {
                                                        // Check exit conditions first
                                                        if account_monitor_flag
                                                            .load(Ordering::Relaxed)
                                                            || account_monitor_timeout
                                                                .load(Ordering::Relaxed)
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

                                                    let mut handler_ref =
                                                        menu_handler_clone2.lock().await;

                                                    handler_ref.to_previous_page();
                                                    handler_ref.to_previous_page();
                                                    handler_ref.to_previous_page();
                                                    if account_monitor_timeout
                                                        .load(Ordering::Relaxed)
                                                    {
                                                        display_bundle_timeout_error_page(
                                                            &mut handler_ref,
                                                        );
                                                        //spawn_error_input_listener(
                                                        //    menu_handler_clone1,
                                                        //    10,
                                                        //);
                                                    } else {
                                                        display_info_page(
                                                            vec![InfoSegment::Normal(format!(
                                                                "Funded Bundle Wallets."
                                                            ))],
                                                            String::from("Success."),
                                                            &mut handler_ref,
                                                            None,
                                                            None,
                                                            None,
                                                        ); //stdout_task.await;
                                                    }
                                                });
                                            };
                                        }
                                        Err(e) => {
                                            handler_ref.to_previous_page();
                                            handler_ref.to_previous_page();
                                            display_balance_fetch_error_page(&mut handler_ref);
                                            spawn_error_input_listener(menu_handler_clone2, 10);
                                        }
                                    }
                                });
                            }
                            "in-contract" => {
                                //first of all create the funding manifest and check the result

                                let res = create_funding_manifest(amounts);
                                loading_handler_ref.to_previous_page();
                                loading_handler_ref.to_previous_page();
                                if let Ok(_) = res {
                                    display_info_page(
                                        vec![
                                            InfoSegment::Normal(format!("Funding Manifest Created.")),
                                            InfoSegment::Emphasized(format!("All wallets will be automatically funded during the launch step.")),
                                        ],
                                        String::from("Success."),
                                        &mut loading_handler_ref,
                                        None,
                                        None,
                                        None,
                                    ); //stdout_task.await;
                                } else if let Err(e) = res {
                                    display_error_page(
                                        e,
                                        Some(String::from("Funding manifest Error")),
                                        None,
                                        Some(5),
                                        &mut loading_handler_ref,
                                        false, //menu_handler_clone2
                                    );

                                    spawn_error_input_listener(menu_handler_clone2, 5);
                                }
                            }
                            _ => {}
                        }
                    }
                };
            } else if let Err(loading_err) = bundle_wallets_validation {
                //loading_handler_ref.to_previous_page();
                display_error_page(
                    loading_err,
                    Some(String::from("Wallet Configuration Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );

                spawn_error_input_listener(menu_handler_clone2, 5);
            }
        }
        //wallet cleanup
        OptionCallback::ReceiverInputCallback(data) => {
            display_loading_page(
                String::from("Checking Address Validity."),
                &mut loading_handler_ref,
            );
            drop(loading_handler_ref);

            let connection_ref = Arc::clone(&connection);
            tokio::spawn(async move {
                let validation: Result<(), String> = if data.len() == 0 {
                    Err(String::from("Input cannot be empty"))
                } else {
                    let is_pubkey = Pubkey::from_str(&data);
                    if let Ok(key) = is_pubkey {
                        if !key.is_on_curve() {
                            Err(String::from("Address must be on-curve"))
                        } else {
                            let account_data = connection_ref.get_account(&key);
                            if let Ok(account) = account_data {
                                if system_program::check_id(&account.owner) {
                                    Ok(())
                                } else {
                                    Err(String::from("Address is not owned by the system program"))
                                }
                            } else {
                                Ok(())
                            }
                        }
                    } else {
                        Err(String::from("Invalid Pubkey provided"))
                    }
                };

                let mut handler_ref = menu_handler_clone1.lock().await;

                handler_ref.to_previous_page();
                if let Err(err) = validation {
                    display_error_page(
                        err,
                        Some(String::from("Address input error")),
                        Some(vec![String::from("- Check your input and try again")]),
                        Some(10),
                        &mut handler_ref,
                        false, //menu_handler_clone2
                    );

                    spawn_error_input_listener(menu_handler_clone2, 10);
                } else {
                    let senders_page =
                        get_wallet_cleanup_page(&mut handler_ref, WalletType::Another(data));
                    handler_ref.change_page(senders_page);
                }
            });
        }

        OptionCallback::CleanUpSol(sender, receiver) => {
            //first of all now which wallet is receiving the sol
            let mut wallet_to_receive: Pubkey = funding_wallet.pubkey();
            match receiver {
                WalletType::FundingWallet => wallet_to_receive = funding_wallet.pubkey(),
                WalletType::Another(wallet) => {
                    wallet_to_receive = Pubkey::from_str(&wallet).unwrap()
                }
                _ => {}
            };

            let mut is_single_wallet: bool = false;
            let mut single_wallet_sender: Option<Arc<Keypair>> = None;
            let mut multi_wallet_sender: Option<Result<Vec<String>, String>> = None;
            match sender {
                WalletType::DevWallet => {
                    single_wallet_sender = Some(Arc::clone(&dev_wallet));
                    is_single_wallet = true;
                }
                WalletType::BumpWallet => {
                    single_wallet_sender = Some(Arc::clone(&dev_wallet));
                    is_single_wallet = true;
                }
                WalletType::BundleWalletSol => {
                    multi_wallet_sender = Some(validate_and_retrieve_pump_bundler_keypairs(
                        &funding_wallet.pubkey(),
                        &dev_wallet.pubkey(),
                    ));
                    is_single_wallet = false;
                }
                _ => {}
            };

            if is_single_wallet {
                display_loading_page(
                    String::from("Checking wallet balance."),
                    &mut loading_handler_ref,
                );
            } else {
                display_loading_page(
                    String::from("Checking all wallet balances."),
                    &mut loading_handler_ref,
                );
            };
            drop(loading_handler_ref);

            let connection_ref = Arc::clone(&connection);
            let funder_ref = Arc::clone(&funding_wallet);

            tokio::spawn(async move {
                let mut senders: Vec<Keypair> = vec![];

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
                let guard_data =
                    BundleGuard::deserialize(&mut &guard_account.unwrap().value.unwrap().data[8..])
                        .unwrap();

                let balances: Result<Vec<u64>, ()> =
                    if let Some(wallet_to_send) = single_wallet_sender {
                        senders.push(wallet_to_send.insecure_clone());
                        let single_wallet_balance =
                            get_balance(connection, &wallet_to_send.pubkey()).await;
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        match single_wallet_balance {
                            Ok(balance) => Ok(vec![balance]),
                            Err(error) => {
                                handler_ref.to_previous_page();
                                display_balance_fetch_error_page(&mut handler_ref);
                                spawn_error_input_listener(menu_handler_clone2, 10);
                                Err(())
                            }
                        }
                    } else if let Some(multi_wallets_to_send) = multi_wallet_sender {
                        match multi_wallets_to_send {
                            Ok(wallets) => {
                                senders = wallets
                                    .iter()
                                    .map(|pair| Keypair::from_base58_string(pair))
                                    .collect::<Vec<Keypair>>();
                                let mult_wallet_balance =
                                    get_balances_for_wallets(connection, Arc::new(wallets)).await;
                                let mut handler_ref = menu_handler_clone1.lock().await;

                                match mult_wallet_balance {
                                    Ok(balances) => Ok(balances),
                                    Err(err) => {
                                        handler_ref.to_previous_page();
                                        display_balance_fetch_error_page(&mut handler_ref);
                                        spawn_error_input_listener(menu_handler_clone2, 10);
                                        Err(())
                                    }
                                }
                            }
                            Err(loading_err) => {
                                let mut handler_ref = menu_handler_clone1.lock().await;
                                handler_ref.to_previous_page();
                                display_error_page(
                                    loading_err,
                                    Some(String::from("Wallet Configuration Error")),
                                    None,
                                    Some(5),
                                    &mut handler_ref,
                                    false, //menu_handler_clone2
                                );
                                spawn_error_input_listener(menu_handler_clone2, 5);
                                Err(())
                            }
                        }
                    } else {
                        Err(())
                    };

                if let Ok(balances) = balances {
                    let mut handler_ref = menu_handler_clone1.lock().await;
                    handler_ref.to_previous_page();
                    if balances.iter().sum::<u64>() == 0 {
                        display_info_page(
                            vec![InfoSegment::Normal(format!("All Sol Retrieved.",))],
                            String::from("Success."),
                            &mut handler_ref,
                            None,
                            None,
                            None,
                        ); //stdout_task.await;
                        return;
                    };

                    display_loading_page(String::from("Retrieving Sol."), &mut handler_ref);
                    drop(handler_ref);

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
                    //info!("logging stuff");

                    tokio::spawn(async move {
                        blockhash_manager.start();
                        notify_websocket_opened_ref.notified().await;
                        let semaphore: Arc<Semaphore> = Arc::new(Semaphore::new(100));

                        while !bundle_sender_flag.load(Ordering::Relaxed)
                            && !bundle_timeout_flag.load(Ordering::Relaxed)
                        {
                            let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();

                            let bundle_balancer_ref = Arc::clone(&bundle_balancer_ref);
                            let funder_keypair_ref = Arc::clone(&funder_keypair_ref);
                            let bundle = get_retrieve_sol_bundle(
                                Arc::clone(&blockhash_manager),
                                Arc::clone(&funder_keypair_ref),
                                &senders,
                                wallet_to_receive,
                                &bundle_guard,
                                guard_data.nonce,
                                &balances,
                                tip,
                            );
                            //info!("{:?}", bundle.clone());
                            //simulate_bundle(bundle, true).await;
                            tokio::spawn(async move {
                                let _ = bundle_balancer_ref.send_bundle(bundle).await;
                                drop(permit);
                            });
                            sleep(Duration::from_millis(50)).await;
                        }
                        blockhash_manager.stop();
                    });

                    spawn_bundle_timeout_task(
                        Arc::clone(&global_config),
                        Arc::clone(&timeout_flag),
                        0,
                    );

                    let account_monitor_flag = Arc::clone(&stop_flag);
                    let account_monitor_timeout = Arc::clone(&timeout_flag);
                    let connection_ref = Arc::clone(&connection_ref);
                    //let funder_ref = Arc::clone(&funding_wallet);
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

                        let mut handler_ref = menu_handler_clone1.lock().await;
                        handler_ref.to_previous_page();
                        if account_monitor_timeout.load(Ordering::Relaxed) {
                            display_bundle_timeout_error_page(&mut handler_ref);
                            //drop(handler_ref);
                            //spawn_error_input_listener(menu_handler_clone1, 10);
                        } else {
                            display_info_page(
                                vec![InfoSegment::Normal(format!("All Sol Retrieved.",))],
                                String::from("Success."),
                                &mut handler_ref,
                                None,
                                None,
                                None,
                            ); //stdout_task.await;
                        }
                    });
                };
            });
        }

        OptionCallback::CleanUpTokens(data, sender, receiver) => {
            //first of all check for the validity of the inputted pubkey

            let token_addy = parse_token_address(data.as_str()).unwrap_or_default();
            let token_pubkey = Pubkey::from_str(token_addy.as_str());
            if let Ok(key) = token_pubkey {
                display_loading_page(
                    String::from("Checking Inputted Token Validity"),
                    &mut loading_handler_ref,
                );
                drop(loading_handler_ref);

                tokio::spawn(async move {
                    let pump_token_validation = is_pump_token(Arc::clone(&connection), &key).await;

                    if let Ok(_) = pump_token_validation {
                        let mut wallet_to_receive: Pubkey = funding_wallet.pubkey();
                        match receiver {
                            WalletType::DevWallet => wallet_to_receive = dev_wallet.pubkey(),
                            WalletType::Another(data) => {
                                wallet_to_receive = Pubkey::from_str(&data).unwrap()
                            }
                            _ => {}
                        };

                        let mut is_single_wallet: bool = false;
                        let mut single_wallet_sender: Option<Arc<Keypair>> = None;
                        let mut multi_wallet_sender: Option<Result<Vec<String>, String>> = None;
                        match sender {
                            WalletType::DevWallet => {
                                single_wallet_sender = Some(Arc::clone(&dev_wallet));
                                is_single_wallet = true;
                            }
                            WalletType::BundleWalletTokens => {
                                multi_wallet_sender =
                                    Some(validate_and_retrieve_pump_bundler_keypairs(
                                        &funding_wallet.pubkey(),
                                        &dev_wallet.pubkey(),
                                    ));
                                is_single_wallet = false;
                            }
                            _ => {}
                        };

                        let mut handler_ref = menu_handler_clone1.lock().await;
                        if is_single_wallet {
                            display_loading_page(
                                String::from("Checking wallet token balance."),
                                &mut handler_ref,
                            );
                        } else {
                            display_loading_page(
                                String::from("Checking all wallets token balances."),
                                &mut handler_ref,
                            );
                        };

                        drop(handler_ref);

                        let connection_ref = Arc::clone(&connection);
                        let funder_ref = Arc::clone(&funding_wallet);
                        tokio::spawn(async move {
                            let mut burners_or_senders: Vec<Arc<Keypair>> = vec![];

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
                                return;
                            }
                            let guard_data = BundleGuard::deserialize(
                                &mut &guard_account.unwrap().value.unwrap().data[8..],
                            )
                            .unwrap();

                            if let Some(ref senders) = multi_wallet_sender {
                                if let Err(ref e) = senders {
                                    let mut handler_ref = menu_handler_clone1.lock().await;
                                    handler_ref.to_previous_page();
                                    handler_ref.to_previous_page();
                                    handler_ref.to_previous_page();
                                    handler_ref.to_previous_page();
                                    display_error_page(
                                        e.clone(),
                                        Some(String::from("Wallet Config Error")),
                                        None,
                                        Some(5),
                                        &mut handler_ref,
                                        false, //menu_handler_clone2
                                    );
                                    spawn_error_input_listener(menu_handler_clone2, 5);
                                    return;
                                }
                            }

                            let token_account_data_res: Vec<Option<TokenAccountBalanceState>> =
                                if is_single_wallet {
                                    let single_burner_ata = get_associated_token_address(
                                        &single_wallet_sender.clone().unwrap().pubkey(),
                                        &key,
                                    );
                                    let account = connection_ref.get_account_with_commitment(
                                        &single_burner_ata,
                                        CommitmentConfig::processed(),
                                    );

                                    if let Ok(data) = account {
                                        let data = data.value;
                                        if let Some(account_data) = data {
                                            let token_account_data =
                                                Account::unpack(&account_data.data).unwrap();

                                            burners_or_senders
                                                .push(single_wallet_sender.clone().unwrap());

                                            if token_account_data.amount != 0 {
                                                vec![Some(
                                                    TokenAccountBalanceState::ExistsWithBalance(
                                                        token_account_data.amount,
                                                    ),
                                                )]
                                            } else {
                                                vec![Some(
                                                    TokenAccountBalanceState::ExistsWithNoBalance,
                                                )]
                                            }
                                        } else {
                                            vec![Some(TokenAccountBalanceState::DoesNotExist)]
                                        }
                                    } else {
                                        vec![None]
                                    }
                                } else {
                                    let wallets = multi_wallet_sender
                                        .unwrap()
                                        .unwrap()
                                        .iter()
                                        .map(|w| Arc::new(Keypair::from_base58_string(w)))
                                        .collect::<Vec<Arc<Keypair>>>();

                                    let atas = wallets
                                        .iter()
                                        .map(|w| get_associated_token_address(&w.pubkey(), &key))
                                        .collect::<Vec<Pubkey>>();

                                    let accounts = connection_ref
                                        .get_multiple_accounts_with_commitment(
                                            &atas,
                                            CommitmentConfig::processed(),
                                        );

                                    if let Ok(data) = accounts {
                                        let mut balance_states_vec: Vec<
                                            Option<TokenAccountBalanceState>,
                                        > = vec![];
                                        for (idx, data) in data.value.iter().enumerate() {
                                            if let Some(account_data) = data {
                                                let token_account_data =
                                                    Account::unpack(&account_data.data).unwrap();

                                                if token_account_data.amount != 0 {
                                                    balance_states_vec.push(Some(
                                                        TokenAccountBalanceState::ExistsWithBalance(
                                                            token_account_data.amount,
                                                        ),
                                                    ));
                                                } else {
                                                    balance_states_vec.push(Some(
                                                        TokenAccountBalanceState::ExistsWithNoBalance,
                                                    ));
                                                };

                                                burners_or_senders.push(wallets[idx].clone());
                                            } else {
                                                balance_states_vec.push(Some(
                                                    TokenAccountBalanceState::DoesNotExist,
                                                ));
                                            }
                                        }
                                        balance_states_vec
                                    } else {
                                        vec![None; wallets.len()]
                                    }
                                };

                            let has_error =
                                token_account_data_res.iter().any(|result| result.is_none());

                            let all_clear = token_account_data_res.iter().all(|result| {
                                if let Some(ref balance_state) = result {
                                    balance_state.eq(&TokenAccountBalanceState::DoesNotExist)
                                } else {
                                    false
                                }
                            });

                            //info!("{:#?}",token_account_data_res);

                            if has_error {
                                let mut handler_ref = menu_handler_clone1.lock().await;
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                display_balance_fetch_error_page(&mut handler_ref);
                                spawn_error_input_listener(menu_handler_clone2, 10);
                                return;
                            }

                            if all_clear {
                                let mut handler_ref = menu_handler_clone1.lock().await;
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                display_info_page(
                                    vec![InfoSegment::Normal(format!(
                                        "No Held tokens to transfer.",
                                    ))],
                                    String::from("Success."),
                                    &mut handler_ref,
                                    None,
                                    None,
                                    None,
                                ); //stdout_task.await;
                                return;
                            }

                            let token_data = fetch_pump_token_general_data(&key).await;
                            let token_symbol = if let Ok(data) = token_data {
                                format!("${}", data.symbol)
                            } else {
                                String::from("$???")
                            };

                            let filtered_balance_states: Vec<TokenAccountBalanceState> =
                                token_account_data_res
                                    .into_iter() // Consume the vector, moving the values
                                    .filter_map(|state_option| {
                                        match state_option {
                                            Some(TokenAccountBalanceState::ExistsWithNoBalance) => {
                                                Some(TokenAccountBalanceState::ExistsWithNoBalance)
                                            }
                                            Some(TokenAccountBalanceState::ExistsWithBalance(
                                                amount,
                                            )) => Some(
                                                TokenAccountBalanceState::ExistsWithBalance(amount),
                                            ),
                                            _ => None, // Filter out all other cases
                                        }
                                    })
                                    .collect();

                            //now just launch the bundle ig
                            //info!("{:#?}", filtered_balance_states);

                            let mut handler_ref = menu_handler_clone1.lock().await;
                            display_loading_page(
                                String::from("Transfering Tokens."),
                                &mut handler_ref,
                            );
                            drop(handler_ref);

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
                            //info!("logging stuff");

                            tokio::spawn(async move {
                                blockhash_manager.start();
                                notify_websocket_opened_ref.notified().await;
                                let semaphore: Arc<Semaphore> = Arc::new(Semaphore::new(100));

                                while !bundle_sender_flag.load(Ordering::Relaxed)
                                    && !bundle_timeout_flag.load(Ordering::Relaxed)
                                {
                                    let permit =
                                        Arc::clone(&semaphore).acquire_owned().await.unwrap();

                                    let bundle_balancer_ref = Arc::clone(&bundle_balancer_ref);
                                    let funder_keypair_ref = Arc::clone(&funder_keypair_ref);
                                    let bundle = get_burn_or_retrieve_tokens_bundle(
                                        Arc::clone(&blockhash_manager),
                                        &key,
                                        Arc::clone(&funder_keypair_ref),
                                        &burners_or_senders,
                                        wallet_to_receive,
                                        &bundle_guard,
                                        guard_data.nonce,
                                        &filtered_balance_states,
                                        tip,
                                        false,
                                    );
                                    //info!("{:?}", bundle.clone());
                                    //simulate_bundle(bundle, true).await;
                                    tokio::spawn(async move {
                                        let _ = bundle_balancer_ref.send_bundle(bundle).await;
                                        drop(permit);
                                    });
                                    sleep(Duration::from_millis(50)).await;
                                }
                                blockhash_manager.stop();
                            });

                            spawn_bundle_timeout_task(
                                Arc::clone(&global_config),
                                Arc::clone(&timeout_flag),
                                0,
                            );

                            let account_monitor_flag = Arc::clone(&stop_flag);
                            let account_monitor_timeout = Arc::clone(&timeout_flag);
                            let connection_ref = Arc::clone(&connection_ref);
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

                                let mut handler_ref = menu_handler_clone1.lock().await;
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                if account_monitor_timeout.load(Ordering::Relaxed) {
                                    display_bundle_timeout_error_page(&mut handler_ref);
                                    //drop(handler_ref);
                                    //spawn_error_input_listener(menu_handler_clone1, 10);
                                } else {
                                    display_info_page(
                                        vec![InfoSegment::Normal(format!(
                                            "Transfered all {token_symbol}.",
                                        ))],
                                        String::from("Success."),
                                        &mut handler_ref,
                                        None,
                                        None,
                                        None,
                                    ); //stdout_task.await;
                                }
                            });
                        });
                    } else if let Err(e) = pump_token_validation {
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        handler_ref.to_previous_page();
                        handler_ref.to_previous_page();
                        handler_ref.to_previous_page();
                        display_error_page(
                            e,
                            Some(String::from("Input Error")),
                            None,
                            Some(5),
                            &mut handler_ref,
                            false, //menu_handler_clone2
                        );
                        spawn_error_input_listener(menu_handler_clone2, 5);
                    }
                });
            } else if let Err(e) = token_pubkey {
                loading_handler_ref.to_previous_page();
                loading_handler_ref.to_previous_page();
                display_error_page(
                    "Invalid Publickey provided.".to_string(),
                    Some(String::from("Input Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 5);
            }
        }

        OptionCallback::OnDemandBurn((wallet_type, data)) => {
            //first of all check for the validity of the inputted pubkey

            //let connection_ref = Arc::clone(&connection);
            let token_addy = parse_token_address(data.as_str()).unwrap_or_default();
            let token_pubkey = Pubkey::from_str(token_addy.as_str());
            if let Ok(key) = token_pubkey {
                display_loading_page(
                    String::from("Checking Inputted Token Validity"),
                    &mut loading_handler_ref,
                );
                drop(loading_handler_ref);

                tokio::spawn(async move {
                    let pump_token_validation = is_pump_token(Arc::clone(&connection), &key).await;

                    if let Ok(_) = pump_token_validation {
                        let mut wallet_to_receive: Pubkey = funding_wallet.pubkey();
                        match wallet_type {
                            WalletType::FundingWallet => {
                                wallet_to_receive = funding_wallet.pubkey()
                            }
                            WalletType::DevWallet => wallet_to_receive = dev_wallet.pubkey(),
                            _ => {}
                        };

                        let mut is_single_wallet: bool = false;
                        let mut single_wallet_burner: Option<Arc<Keypair>> = None;
                        let mut multi_wallet_sender: Option<Result<Vec<String>, String>> = None;
                        match wallet_type {
                            WalletType::DevWallet => {
                                single_wallet_burner = Some(Arc::clone(&dev_wallet));
                                is_single_wallet = true;
                            }
                            WalletType::BundleWalletSol => {
                                multi_wallet_sender =
                                    Some(validate_and_retrieve_pump_bundler_keypairs(
                                        &funding_wallet.pubkey(),
                                        &dev_wallet.pubkey(),
                                    ));
                                is_single_wallet = false;
                            }
                            _ => {}
                        };

                        let mut handler_ref = menu_handler_clone1.lock().await;
                        if is_single_wallet {
                            display_loading_page(
                                String::from("Checking wallet token balance."),
                                &mut handler_ref,
                            );
                        } else {
                            display_loading_page(
                                String::from("Checking all wallets token balances."),
                                &mut handler_ref,
                            );
                        };

                        drop(handler_ref);

                        let connection_ref = Arc::clone(&connection);
                        let funder_ref = Arc::clone(&funding_wallet);
                        tokio::spawn(async move {
                            let mut burners_or_senders: Vec<Arc<Keypair>> = vec![];

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
                                return;
                            }
                            let guard_data = BundleGuard::deserialize(
                                &mut &guard_account.unwrap().value.unwrap().data[8..],
                            )
                            .unwrap();

                            let token_account_data_res: Vec<Option<TokenAccountBalanceState>> =
                                if is_single_wallet {
                                    burners_or_senders.push(single_wallet_burner.clone().unwrap());
                                    let single_burner_ata = get_associated_token_address(
                                        &single_wallet_burner.clone().unwrap().pubkey(),
                                        &key,
                                    );
                                    let account = connection_ref.get_account_with_commitment(
                                        &single_burner_ata,
                                        CommitmentConfig::processed(),
                                    );

                                    if let Ok(data) = account {
                                        let data = data.value;
                                        if let Some(account_data) = data {
                                            let token_account_data =
                                                Account::unpack(&account_data.data).unwrap();

                                            if token_account_data.amount != 0 {
                                                vec![Some(
                                                    TokenAccountBalanceState::ExistsWithBalance(
                                                        token_account_data.amount,
                                                    ),
                                                )]
                                            } else {
                                                vec![Some(
                                                    TokenAccountBalanceState::ExistsWithNoBalance,
                                                )]
                                            }
                                        } else {
                                            vec![Some(TokenAccountBalanceState::DoesNotExist)]
                                        }
                                    } else {
                                        vec![None]
                                    }
                                } else {
                                    //TODO: will implement later, not needed now
                                    vec![None]
                                };

                            let has_error =
                                token_account_data_res.iter().any(|result| result.is_none());

                            let all_clear = token_account_data_res.iter().all(|result| {
                                if let Some(ref balance_state) = result {
                                    balance_state.eq(&TokenAccountBalanceState::DoesNotExist)
                                } else {
                                    false
                                }
                            });

                            if has_error {
                                let mut handler_ref = menu_handler_clone1.lock().await;
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                display_balance_fetch_error_page(&mut handler_ref);
                                spawn_error_input_listener(menu_handler_clone2, 10);
                                return;
                            }

                            if all_clear {
                                let mut handler_ref = menu_handler_clone1.lock().await;
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                display_info_page(
                                    vec![InfoSegment::Normal(format!("No Held tokens to burn.",))],
                                    String::from("Success."),
                                    &mut handler_ref,
                                    None,
                                    None,
                                    None,
                                ); //stdout_task.await;
                                return;
                            }

                            let token_data = fetch_pump_token_general_data(&key).await;
                            let token_symbol = if let Ok(data) = token_data {
                                format!("${}", data.symbol)
                            } else {
                                String::from("$???")
                            };

                            let filtered_balance_states: Vec<TokenAccountBalanceState> =
                                token_account_data_res
                                    .into_iter() // Consume the vector, moving the values
                                    .filter_map(|state_option| {
                                        match state_option {
                                            Some(TokenAccountBalanceState::ExistsWithNoBalance) => {
                                                Some(TokenAccountBalanceState::ExistsWithNoBalance)
                                            }
                                            Some(TokenAccountBalanceState::ExistsWithBalance(
                                                amount,
                                            )) => Some(
                                                TokenAccountBalanceState::ExistsWithBalance(amount),
                                            ),
                                            _ => None, // Filter out all other cases
                                        }
                                    })
                                    .collect();

                            //now just launch the bundle ig

                            let mut handler_ref = menu_handler_clone1.lock().await;
                            display_loading_page(String::from("Burning Tokens."), &mut handler_ref);
                            drop(handler_ref);

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
                            //info!("logging stuff");

                            tokio::spawn(async move {
                                blockhash_manager.start();
                                notify_websocket_opened_ref.notified().await;
                                let semaphore: Arc<Semaphore> = Arc::new(Semaphore::new(100));

                                while !bundle_sender_flag.load(Ordering::Relaxed)
                                    && !bundle_timeout_flag.load(Ordering::Relaxed)
                                {
                                    let permit =
                                        Arc::clone(&semaphore).acquire_owned().await.unwrap();

                                    let bundle_balancer_ref = Arc::clone(&bundle_balancer_ref);
                                    let funder_keypair_ref = Arc::clone(&funder_keypair_ref);
                                    let bundle = get_burn_or_retrieve_tokens_bundle(
                                        Arc::clone(&blockhash_manager),
                                        &key,
                                        Arc::clone(&funder_keypair_ref),
                                        &burners_or_senders,
                                        wallet_to_receive,
                                        &bundle_guard,
                                        guard_data.nonce,
                                        &filtered_balance_states,
                                        tip,
                                        true,
                                    );
                                    //info!("{:?}", bundle.clone());
                                    //simulate_bundle(bundle, true).await;
                                    tokio::spawn(async move {
                                        let _ = bundle_balancer_ref.send_bundle(bundle).await;
                                        drop(permit);
                                    });
                                    sleep(Duration::from_millis(50)).await;
                                }
                                blockhash_manager.stop();
                            });

                            spawn_bundle_timeout_task(
                                Arc::clone(&global_config),
                                Arc::clone(&timeout_flag),
                                0,
                            );

                            let account_monitor_flag = Arc::clone(&stop_flag);
                            let account_monitor_timeout = Arc::clone(&timeout_flag);
                            let connection_ref = Arc::clone(&connection_ref);
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

                                let mut handler_ref = menu_handler_clone1.lock().await;
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                if account_monitor_timeout.load(Ordering::Relaxed) {
                                    display_bundle_timeout_error_page(&mut handler_ref);
                                    //drop(handler_ref);
                                    //spawn_error_input_listener(menu_handler_clone1, 10);
                                } else {
                                    display_info_page(
                                        vec![InfoSegment::Normal(format!(
                                            "Burnt all {token_symbol}.",
                                        ))],
                                        String::from("Success."),
                                        &mut handler_ref,
                                        None,
                                        None,
                                        None,
                                    ); //stdout_task.await;
                                }
                            });
                        });
                    } else if let Err(e) = pump_token_validation {
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        handler_ref.to_previous_page();
                        handler_ref.to_previous_page();
                        handler_ref.to_previous_page();
                        display_error_page(
                            e,
                            Some(String::from("Input Error")),
                            None,
                            Some(5),
                            &mut handler_ref,
                            false, //menu_handler_clone2
                        );
                        spawn_error_input_listener(menu_handler_clone2, 5);
                    }
                });
            } else if let Err(e) = token_pubkey {
                loading_handler_ref.to_previous_page();
                loading_handler_ref.to_previous_page();
                display_error_page(
                    "Invalid Publickey provided.".to_string(),
                    Some(String::from("Input Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 5);
            }
        }

        _ => {}
    }
}
