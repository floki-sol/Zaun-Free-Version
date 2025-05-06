use std::{
    fs,
    io::stdout,
    path::Path,
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
use crossterm::{style::SetBackgroundColor, terminal, ExecutableCommand};
use futures::{SinkExt, StreamExt};
use log::info;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use solana_account_decoder::UiDataSliceConfig;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcSendTransactionConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_sdk::{
    address_lookup_table::state::AddressLookupTable, commitment_config::CommitmentConfig,
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
use tokio::{
    sync::{Mutex, Notify, RwLock, Semaphore},
    time::sleep,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use crate::{
    cli::{
        error::{
            display_balance_fetch_error_page, display_bundle_guard_fetch_error_page,
            display_bundle_timeout_error_page, display_error_page, spawn_error_input_listener,
        },
        info::{display_info_page, InfoPage, InfoSegment},
        live_pausable::LivePausableInfoPage,
        loading_indicator::display_loading_page,
        menu::{MenuHandler, Page},
        options::OptionCallback,
        pages::pump::main_menu::bundle::launch_setup::{
            bundle_snipe_flavor::get_bundle_snipe_flavor_option_page, dev_buy,
            launch_options::get_launch_option_page,
            metadata_choice::get_metadata_configuration_page,
        },
    },
    constants::general::{
        BundleGuard, CircularKeypairBuffer, LaunchMode, SplitBundleConfig, SplitBundleFlavor,
        BOT_PROGRAM_ID, PUMP_PROGRAM_ID, PUMP_TOKEN_DECIMALS,
    },
    jito::bundles::{simulate_bundle, BundleSenderBalancer},
    loaders::{
        bundle_wallets_loader::validate_and_retrieve_pump_bundler_keypairs,
        global_config_loader::{GlobalConfig, JitoSplitBundlePercentages},
        launch_manifest_loader::{LaunchManifest, LaunchManifestWalletEntry, SimpleMetadata},
        metadata_loader::validate_and_retrieve_metadata,
    },
    utils::{
        backups::{backup_files, load_most_recent_lut, BackupType},
        blockhash_manager::RecentBlockhashManager,
        bonding_curve_provider::{run_curve_provider, BondingCurve, BondingCurveProvider},
        bundle_factory::{
            get_classic_launch_bundle,
            get_create_lookup_table_bundle ,
        },
        misc::{
            adjust_file_path, calculate_pump_tokens_to_buy, can_use_lut, create_launch_manifest,
            extract_data, extract_lamports, fetch_and_validate_metadata_uri, fix_ipfs_url,
            get_account_subscription_message, get_associated_accounts, get_balances_for_wallets,
            get_global_lut_data, get_lookup_table_creation_cost,
            get_session_lut_data, is_valid_decimal_string, is_valid_small_integer_string,
            parse_token_address, retrieve_funding_manifest, spawn_bundle_timeout_task, split_tip,
        },
        pdas::{get_bonding_curve, get_bundle_guard},
        pump_helpers::{derive_all_pump_keys, login, register, upload_metadata},
    },
};

pub async fn invoke_launch_callback(
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
        //launch
        OptionCallback::StartDustCoinsTask(should_stop_dust_task) => {
            ////
        }

        OptionCallback::StopDustCoinsTask(flag) => {
            ////
        }

        OptionCallback::StartTradingActivityTask => {
            ////
        }

        OptionCallback::StopTradingActivityTask(flag) => {
            ////
        }

        OptionCallback::SimulateLaunch => {
            ////
        }
        OptionCallback::SimulateHolderDistributions(val) => {
            ////
        }

        OptionCallback::GenerateCa => {
            let token_pkey = Keypair::new();
            let metadata_page =
                get_metadata_configuration_page(&mut loading_handler_ref, token_pkey.into());
            loading_handler_ref.change_page(metadata_page);
        }
        OptionCallback::ValidateBase58Ca(data) => {
            display_loading_page(
                String::from("Checking Base58 CA validity"),
                &mut loading_handler_ref,
            );
            drop(loading_handler_ref);

            let connection_ref = Arc::clone(&connection);

            tokio::spawn(async move {
                let data_validation: Result<Keypair, String> = {
                    if data.len() == 0 {
                        Err(String::from("Input cannot be empty."))
                    } else {
                        //validate base58 strings for wallets
                        let byte_array = bs58::decode(&data)
                            .into_vec()
                            .map_err(|e| String::from("Invalid base58 String."));

                        let validation = if let Err(reason) = byte_array {
                            Err(reason)
                        } else {
                            let token_keypair =
                                Keypair::from_bytes(&byte_array.unwrap().as_slice()).map_err(|e| {
                                    String::from("Invalid base58 private key supplied.")
                                });

                            if let Err(reason) = token_keypair {
                                Err(reason)
                            } else {
                                // here I also check if the keypair has data or not
                                let keypair = token_keypair.unwrap();
                                let pubkey = keypair.pubkey();
                                let key_data = connection_ref.get_account(&pubkey);
                                if key_data.is_ok() {
                                    Err(String::from(
                                        "Associated Account with CA is already in use.",
                                    ))
                                } else {
                                    Ok(keypair)
                                }
                            }
                        };

                        validation
                    }
                };

                let mut handler_ref = menu_handler_clone1.lock().await;

                handler_ref.to_previous_page();
                if let Ok(data) = data_validation {
                    //todo!();
                    let metadata_page =
                        get_metadata_configuration_page(&mut handler_ref, data.into());
                    handler_ref.change_page(metadata_page);
                    //here we do something about moving on to the next page
                } else if let Err(err) = data_validation {
                    //handler_ref.to_previous_page();
                    display_error_page(
                        err,
                        Some(String::from("CA input Error")),
                        None,
                        Some(5),
                        &mut handler_ref,
                        false, //menu_handler_clone2
                    );

                    spawn_error_input_listener(menu_handler_clone2, 5);
                };
            });
        }
        OptionCallback::ValidateCaFile => {
            display_loading_page(
                String::from("Checking CA keypair file validity"),
                &mut loading_handler_ref,
            );
            drop(loading_handler_ref);

            let connection_ref = Arc::clone(&connection);
            tokio::spawn(async move {
                let data_validation: Result<Keypair, String> = {
                    // Path to the CA file

                    let ca_file_str = &adjust_file_path("temp/ca.json");
                    let ca_file_path = Path::new(ca_file_str);

                    // Check if file exists
                    if !ca_file_path.exists() {
                        Err(String::from("CA file does not exist 'temp/ca.json'."))
                    } else {
                        // Read file contents
                        match fs::read_to_string(ca_file_path) {
                            Ok(file_contents) => {
                                // Parse JSON
                                match serde_json::from_str::<Vec<u8>>(&file_contents) {
                                    Ok(byte_array) => {
                                        // Validate keypair bytes length (Solana keypair is typically 64 bytes)
                                        if byte_array.len() != 64 {
                                            Err(String::from("Invalid keypair byte length."))
                                        } else {
                                            // Attempt to create Keypair from bytes
                                            match Keypair::from_bytes(&byte_array.as_slice()) {
                                                Ok(token_keypair) => {
                                                    // Get pubkey and check account
                                                    match connection_ref
                                                        .get_account(&token_keypair.pubkey())
                                                    {
                                                        Ok(acc) => {
                                                            info!("{:?}", acc);

                                                            Err(String::from("Associated Account with CA is already in use."))
                                                        }
                                                        Err(_) => Ok(token_keypair),
                                                    }
                                                }
                                                Err(_) => {
                                                    Err(String::from("Invalid keypair bytes."))
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => Err(String::from("Invalid keypair file format.")),
                                }
                            }
                            Err(_) => Err(String::from("Unable to read CA file.")),
                        }
                    }
                };

                let mut handler_ref = menu_handler.lock().await;

                handler_ref.to_previous_page();

                if let Ok(data) = data_validation {
                    let metadata_page =
                        get_metadata_configuration_page(&mut handler_ref, data.into());
                    handler_ref.change_page(metadata_page);
                } else if let Err(err) = data_validation {
                    display_error_page(
                        err,
                        Some(String::from("CA File Validation Error")),
                        None,
                        Some(5),
                        &mut handler_ref,
                        false,
                    );

                    spawn_error_input_listener(menu_handler.clone(), 5);
                };
            });
        }
        OptionCallback::VerifyAndUploadMetadata(keypair) => {
            let global_config_lock = global_config.read().await;
            let mut use_video = false;
            if let Some(config) = global_config_lock.as_ref() {
                use_video = config.use_video;
            };
            drop(global_config_lock);

            let metadata_validation = validate_and_retrieve_metadata(use_video);
            if let Ok((metadata, image_path)) = metadata_validation {
                display_loading_page(
                    String::from("Uploading metadata and media"),
                    &mut loading_handler_ref,
                );
                drop(loading_handler_ref);

                tokio::spawn(async move {
                    let temp_key = Arc::new(Keypair::new());
                    let login_res = login(Arc::clone(&temp_key)).await;

                    if let Ok(auth_token) = login_res {
                        //info!("{auth_token}");
                        let _ =
                            register(&auth_token, &(Arc::clone(&temp_key)).pubkey().to_string())
                                .await;

                        let name = metadata.name.clone();
                        let symbol = metadata.symbol.clone();
                        let upload_result =
                            upload_metadata(auth_token, image_path, metadata, use_video).await;
                        let mut handler_ref = menu_handler_clone1.lock().await;

                        if let Ok((image, uri, vid)) = upload_result {
                            let mut info_segments: Vec<InfoSegment> = vec![
                                InfoSegment::StringSplitInfo((
                                    String::from("Metadata link"),
                                    String::from(&uri),
                                )),
                                InfoSegment::StringSplitInfo((
                                    String::from("Image link"),
                                    String::from(image),
                                )),
                            ];
                            if use_video {
                                info_segments.push(InfoSegment::StringSplitInfo((
                                    String::from("Video link"),
                                    String::from(vid),
                                )));
                            };
                            handler_ref.to_previous_page();
                            display_info_page(
                                info_segments,
                                String::from("Upload Succesful"),
                                &mut handler_ref,
                                Some(OptionCallback::SetupLookUpTable((
                                    Arc::clone(&keypair),
                                    uri.trim_matches('"').to_string(),
                                    (name, symbol),
                                ))),
                                None,
                                None,
                            ); //stdout_task.await;
                        } else if let Err(e) = upload_result {
                            handler_ref.to_previous_page();
                            display_error_page(
                                e,
                                Some(String::from("Metadata Upload Error")),
                                Some(vec![String::from("Cause: metadata upload.")]),
                                Some(5),
                                &mut handler_ref,
                                false, //menu_handler_clone2
                            );
                            spawn_error_input_listener(menu_handler_clone2, 5);
                        }
                    } else if let Err(e) = login_res {
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        handler_ref.to_previous_page();
                        display_error_page(
                            e,
                            Some(String::from("Metadata Upload Error")),
                            Some(vec![String::from("Cause: Temporary login attempt.")]),
                            Some(5),
                            &mut handler_ref,
                            false, //menu_handler_clone2
                        );
                        spawn_error_input_listener(menu_handler_clone2, 5);
                    }
                });
            } else if let Err(e) = metadata_validation {
                loading_handler_ref.to_previous_page();
                display_error_page(
                    e,
                    Some(String::from("Metadata Validation Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 5);
            }
        }
        OptionCallback::CloneTokenMetadata((keypair, input)) => {
            let token_addy = parse_token_address(input.as_str()).unwrap_or_default();
            //info!("{}", token_addy);
            let token_pubkey = Pubkey::from_str(token_addy.as_str());
            if let Ok(key) = token_pubkey {
                display_loading_page(
                    String::from("Checking Inputted Token Validity"),
                    &mut loading_handler_ref,
                );
                drop(loading_handler_ref);
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
        }
        OptionCallback::VerifyMetadataLinkInput((keypair, input)) => {
            let is_valid = Url::parse(&input);
            if let Ok(url) = is_valid {
                display_loading_page(String::from("Checking metadata"), &mut loading_handler_ref);
                drop(loading_handler_ref);

                tokio::spawn(async move {
                    let metadata_validation =
                        fetch_and_validate_metadata_uri(&fix_ipfs_url(url.as_str())).await;

                    let mut handler_ref = menu_handler_clone1.lock().await;
                    if let Ok((name, symbol, image)) = metadata_validation {
                        //info!("{name},{symbol},{image}",);
                        let info_segments: Vec<InfoSegment> = vec![
                            InfoSegment::StringSplitInfo((
                                String::from("Metadata link"),
                                String::from(url.as_str()),
                            )),
                            InfoSegment::StringSplitInfo((
                                String::from("Image link"),
                                String::from(image),
                            )),
                        ];

                        handler_ref.to_previous_page();
                        handler_ref.to_previous_page();
                        display_info_page(
                            info_segments,
                            String::from("Metadata Valid"),
                            &mut handler_ref,
                            Some(OptionCallback::SetupLookUpTable((
                                Arc::clone(&keypair),
                                url.as_str().to_string(),
                                (name, symbol),
                            ))),
                            None,
                            None,
                        ); //stdout_task.await;
                    } else if let Err(e) = metadata_validation {
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
            } else if let Err(e) = is_valid {
                //loading_handler_ref.to_previous_page();
                display_error_page(
                    format!("Invalid Url provided: {}", e),
                    Some(String::from("Input Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 5);
            }
        }
        OptionCallback::SetupLookUpTable((keypair, metadata, (name, symbol))) => {
            //first of all I'll check if there is an already existing lookup table and if its valid for usage

            let bundle_wallets_validation = validate_and_retrieve_pump_bundler_keypairs(
                &funding_wallet.pubkey(),
                &dev_wallet.pubkey(),
            );
            if let Ok(wallets) = bundle_wallets_validation {
                display_loading_page(
                    String::from("Checking lookup table backups"),
                    &mut loading_handler_ref,
                );
                drop(loading_handler_ref);

                let lookup_table = load_most_recent_lut();

                tokio::spawn(async move {
                    let mut should_create_lut = false;
                    if let Ok(table) = lookup_table {
                        let lut_address = Pubkey::from_str(&table.lookup_table).unwrap();
                        let mint = Pubkey::from_str(&table.mint).unwrap();

                        let lut_data = connection.get_account_data(&lut_address);

                        if let Ok(data) = lut_data {
                            let lut_state = AddressLookupTable::deserialize(&data).unwrap();
                            let stored_addresses = lut_state.addresses;

                            should_create_lut =
                                !can_use_lut(&wallets, stored_addresses.to_vec(), mint);
                        } else {
                            should_create_lut = true;
                        }
                    } else {
                        should_create_lut = true;
                    };

                    if should_create_lut {
                        let connection_ref = Arc::clone(&connection);
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        display_loading_page(String::from("Calculating costs"), &mut handler_ref);
                        drop(handler_ref);

                        tokio::spawn(async move {
                            let creation_costs =
                                get_lookup_table_creation_cost(wallets.len(), connection_ref).await;
                            let mut handler_ref = menu_handler_clone1.lock().await;

                            if let Ok(cost) = creation_costs {
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();
                                display_info_page(
                                        vec![
                                            InfoSegment::StringSplitInfo((
                                                format!("Creation Costs"),
                                                format!("{:.4} Sol", (cost as f64 / LAMPORTS_PER_SOL as f64)),
                                            )),
                                            InfoSegment::Normal(String::from(
                                                "-- Can be claimed later with the `Manage lookup tables` option.",
                                            )),
                                            InfoSegment::Normal(String::from("\n")),
                                            InfoSegment::Emphasized(String::from("Confirm?")),
                                        ],
                                        String::from("Lookup table creation"),
                                        &mut handler_ref,
                                        Some(OptionCallback::CreateLookUpTable((
                                            Arc::clone(&keypair),
                                            metadata,
                                            (name, symbol),
                                        ))),
                                        None,
                                        None,
                                    );
                            } else if let Err(e) = creation_costs {
                                handler_ref.to_previous_page();
                                handler_ref.to_previous_page();

                                display_error_page(
                                    e,
                                    Some(String::from("Lookup table error")),
                                    None,
                                    Some(5),
                                    &mut handler_ref,
                                    false, //menu_handler_clone2
                                );
                                spawn_error_input_listener(menu_handler_clone2, 5);
                            }
                        });
                    } else {
                        let mut handler_ref = menu_handler_clone1.lock().await;

                        let launch_option_page = get_launch_option_page(
                            &mut handler_ref,
                            keypair,
                            metadata,
                            name,
                            symbol,
                        );
                        handler_ref.to_previous_page();
                        handler_ref.to_previous_page();
                        display_info_page(
                            vec![
                                InfoSegment::Normal(String::from(
                                    "Loaded lookup table from backups.",
                                )),
                                InfoSegment::Emphasized(String::from("Valid for usage.")),
                            ],
                            String::from("Look-up table valid"),
                            &mut handler_ref,
                            None,
                            Some(Box::new(launch_option_page)),
                            None,
                        );
                    }
                });
            } else if let Err(loading_err) = bundle_wallets_validation {
                loading_handler_ref.to_previous_page();
                display_error_page(
                    loading_err,
                    Some(String::from("Wallet Configuration Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 5);
            };
        }
        OptionCallback::CreateLookUpTable((keypair, metadata, (name, symbol))) => {
            let bundle_wallets_validation = validate_and_retrieve_pump_bundler_keypairs(
                &funding_wallet.pubkey(),
                &dev_wallet.pubkey(),
            );
            if let Ok(wallets) = bundle_wallets_validation {
                display_loading_page(
                    String::from("Creating Lookup table"),
                    &mut loading_handler_ref,
                );
                drop(loading_handler_ref);
                let connection_ref = Arc::clone(&connection);
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
                    let tip = current_tip.load(Ordering::Relaxed);

                    let confirmed_slots: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
                    let confirmed_slots_ref = Arc::clone(&confirmed_slots);
                    let slot_sender_flag = Arc::clone(&stop_flag);
                    let slot_timeout_flag = Arc::clone(&timeout_flag);

                    let wallets = wallets
                        .iter()
                        .map(|wallet| Keypair::from_base58_string(wallet).pubkey())
                        .collect::<Vec<Pubkey>>();

                    let lut_addresses = get_associated_accounts(&wallets, keypair.pubkey());

                    //create an extra task to store confirmed slots
                    tokio::spawn(async move {
                        while !slot_sender_flag.load(Ordering::Relaxed)
                            && !slot_timeout_flag.load(Ordering::Relaxed)
                        {
                            let slot = connection_ref
                                .get_slot_with_commitment(CommitmentConfig::confirmed())
                                .unwrap_or(0);
                            if slot != 0 {
                                confirmed_slots.store(slot, Ordering::Relaxed);
                            }

                            //info!("fetched new slot: {slot}");
                            sleep(Duration::from_millis(200)).await;
                        }
                    });

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

                            let bundle = get_create_lookup_table_bundle(
                                Arc::clone(&blockhash_manager),
                                &bundle_guard,
                                confirmed_slots_ref.load(Ordering::Relaxed),
                                guard_data.nonce,
                                Arc::clone(&funder_keypair_ref),
                                &lut_addresses,
                                tip,
                            );

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
                    let connection_ref = Arc::clone(&connection);
                    //let funder_ref = Arc::clone(&funding_wallet);
                    let notify_websocket_opened_ref = Arc::clone(&notify_websocket_opened);
                    //let mint_ref = Arc::clone(&keypair);

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

                                                //info!("account data: {}", text);
                                                let data = extract_data(&text);
                                                //info!("account data base64 {:?}", &data );
                                                let raw_data = decode(data.unwrap().trim_matches('"')).unwrap();
                                                let guard_data =
                                                BundleGuard::deserialize(&mut &raw_data[8..]).unwrap();
                                                //info!("deserialized account data {:?}", guard_data );

                                                let _ = backup_files(BackupType::LookupTables((guard_data.lut_guard.unwrap_or_default().to_string(), keypair.pubkey().to_string())));
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
                            let launch_option_page = get_launch_option_page(
                                &mut handler_ref,
                                keypair,
                                metadata,
                                name,
                                symbol,
                            );

                            display_info_page(
                                vec![InfoSegment::Normal(format!(
                                    "Address Lookup table created."
                                ))],
                                String::from("Success."),
                                &mut handler_ref,
                                None,
                                Some(Box::new(launch_option_page)),
                                None,
                            ); //stdout_task.await;
                        }
                    });
                });
            } else if let Err(loading_err) = bundle_wallets_validation {
                loading_handler_ref.to_previous_page();
                display_error_page(
                    loading_err,
                    Some(String::from("Wallet Configuration Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 5);
            };
        }
        OptionCallback::ValidateCTOInput((
            keypair,
            metadata,
            (name, symbol),
            launch_mode,
            data,
        )) => {
            ////
        }
        OptionCallback::ValidateDevBuy((
            keypair,
            metadata,
            (name, symbol),
            launch_mode,
            dev_buy_input,
        )) => {
            //first of all check if input is valid
            let is_valid_input = is_valid_decimal_string(dev_buy_input, 0.0, 100.0);

            if let Ok(amount) = is_valid_input {
                //load the screen

                display_loading_page(String::from("Pre-launch Checks"), &mut loading_handler_ref);
                drop(loading_handler_ref);

                //then spawn a task to check the balance of dev
                tokio::spawn(async move {
                    let dev_balance = connection.get_balance_with_commitment(
                        &dev_wallet.pubkey(),
                        CommitmentConfig::processed(),
                    );

                    if let Ok(balance) = dev_balance {
                        //now we check for balance validity for dev

                        let err: Option<String> = if balance.value
                            < (0.03 * LAMPORTS_PER_SOL as f64) as u64
                        {
                            Some(String::from("Dev balance cannot be less than 0.03 SOL"))
                        } else {
                            let amount_to_remove = ((amount * LAMPORTS_PER_SOL as f64) * 1.01)
                                as u64
                                + (0.03 * LAMPORTS_PER_SOL as f64) as u64;
                            if amount_to_remove >= balance.value {
                                let amount_to_remove_f64 =
                                    amount_to_remove as f64 / LAMPORTS_PER_SOL as f64;
                                let balance_f64 = balance.value as f64 / LAMPORTS_PER_SOL as f64;

                                Some(format!(
                                            "Insufficient Dev balance: Needed: {:.4} Sol, balance: {:.4} Sol",
                                            amount_to_remove_f64, balance_f64,
                                        ))
                            } else {
                                None
                            }
                        };

                        if let Some(e) = err {
                            let mut handler_ref = menu_handler_clone1.lock().await;
                            handler_ref.to_previous_page();
                            display_error_page(
                                e,
                                Some(String::from("Dev Input Error")),
                                None,
                                Some(5),
                                &mut handler_ref,
                                false, //menu_handler_clone2
                            );
                            spawn_error_input_listener(menu_handler_clone2, 5);
                        } else {
                            let global_config_lock = global_config.read().await; // Get the write lock

                            let mut funding_type: String = String::from("");
                            if let Some(config_ref) = global_config_lock.as_ref() {
                                funding_type = config_ref.funding_strategy.clone();
                            }
                            drop(global_config_lock);

                            //info!("sol balances: {:#?}", sol_balances_array);
                            let mut info_segments: Vec<InfoSegment> = vec![
                                //launch details
                                InfoSegment::Normal(String::from("Launch details:")),
                                InfoSegment::StringSplitInfo((
                                    String::from("Mode"),
                                    String::from(launch_mode.to_string()),
                                )),
                                InfoSegment::StringSplitInfo((
                                    String::from("Funding"),
                                    String::from(&funding_type),
                                )),
                                InfoSegment::NumericSplitInfo((
                                    String::from("Jito Tip"),
                                    format!(
                                        "{}",
                                        current_tip.load(Ordering::Relaxed) as f64
                                            / LAMPORTS_PER_SOL as f64
                                    ),
                                )),
                                InfoSegment::Normal(String::from("")),
                                //token details
                                InfoSegment::Normal(String::from("Token details:")),
                                InfoSegment::StringSplitInfo((
                                    String::from("Address"),
                                    String::from(&keypair.pubkey().to_string()),
                                )),
                                InfoSegment::StringSplitInfo((
                                    String::from("Name"),
                                    String::from(&name),
                                )),
                                InfoSegment::StringSplitInfo((
                                    String::from("Symbol"),
                                    String::from(&symbol),
                                )),
                                InfoSegment::StringSplitInfo((
                                    String::from("Uri"),
                                    String::from(&metadata),
                                )),
                                InfoSegment::Normal(String::from("")),
                                //dev buy
                                InfoSegment::Normal(String::from("Dev buy:")),
                                InfoSegment::StringSplitInfo((
                                    String::from("Amount"),
                                    amount.to_string(),
                                )),
                                InfoSegment::Normal(String::from("")),
                            ];

                            let bundle_wallets_validation =
                                validate_and_retrieve_pump_bundler_keypairs(
                                    &funding_wallet.pubkey(),
                                    &dev_wallet.pubkey(),
                                );

                            if let Err(ref e) = bundle_wallets_validation {
                                let mut handler_ref = menu_handler_clone1.lock().await;
                                handler_ref.to_previous_page();
                                display_error_page(
                                    e.clone(),
                                    Some(String::from("Wallet Configuration Error")),
                                    None,
                                    Some(5),
                                    &mut handler_ref,
                                    false, //menu_handler_clone2
                                );
                                spawn_error_input_listener(menu_handler_clone2, 5);
                                return;
                            }

                            let mut sol_balances_array: Vec<u64> = vec![];
                            match launch_mode {
                                // we do nothing if its dev only,
                                LaunchMode::DevOnly => {}
                                //in all launch mode execpt dev only we display overview of bundle wallets
                                _ => {
                                    if funding_type == "in-contract" {
                                        //here im gonna get the balance from the funding manifest
                                        let funding_manifest = retrieve_funding_manifest();
                                        if let Err(ref e) = funding_manifest {
                                            let mut handler_ref = menu_handler_clone1.lock().await;
                                            handler_ref.to_previous_page();
                                            display_error_page(
                                                e.clone(),
                                                Some(String::from("In-contract funding Error")),
                                                None,
                                                Some(5),
                                                &mut handler_ref,
                                                false, //menu_handler_clone2
                                            );
                                            spawn_error_input_listener(menu_handler_clone2, 5);
                                            return;
                                        } else {
                                            sol_balances_array = funding_manifest
                                                .unwrap()
                                                .iter()
                                                .map(|val| (*val * LAMPORTS_PER_SOL as f64) as u64)
                                                .collect();
                                        }
                                    } else {
                                        if let Ok(ref bundle_wallets) = bundle_wallets_validation {
                                            let arced_vec = Arc::new(bundle_wallets.to_vec());
                                            let balance_task_vec_ref = Arc::clone(&arced_vec);
                                            //here im gonna fetch the balances of the wallets
                                            let fetch_res = get_balances_for_wallets(
                                                Arc::clone(&connection),
                                                balance_task_vec_ref,
                                            )
                                            .await;

                                            if let Err(e) = fetch_res {
                                                let mut handler_ref =
                                                    menu_handler_clone1.lock().await;
                                                handler_ref.to_previous_page();
                                                display_balance_fetch_error_page(&mut handler_ref);
                                                spawn_error_input_listener(menu_handler_clone2, 10);
                                                return;
                                            } else {
                                                sol_balances_array = fetch_res.unwrap();
                                            }
                                        }
                                    }

                                    //wallet buys
                                    info_segments
                                        .push(InfoSegment::Normal(String::from("Wallet buys:")));

                                    let wallets: Vec<String> = bundle_wallets_validation
                                        .unwrap()
                                        .iter()
                                        .map(|bs58| {
                                            Keypair::from_base58_string(&bs58).pubkey().to_string()
                                        })
                                        .collect();
                                    //info!("{:#?}", wallets);
                                    for (idx, address) in wallets.iter().enumerate() {
                                        info_segments.push(InfoSegment::Normal(format!(
                                            "Wallet {}",
                                            idx + 1
                                        )));
                                        info_segments.push(InfoSegment::StringSplitInfo((
                                            String::from("-- Address: "),
                                            address.clone(),
                                        )));
                                        info_segments.push(InfoSegment::NumericSplitInfo((
                                            String::from("-- Amount: "),
                                            format!(
                                                "{:.4}",
                                                (sol_balances_array[idx] as f64
                                                    / LAMPORTS_PER_SOL as f64)
                                            ),
                                        )));
                                        info_segments.push(InfoSegment::Normal("".to_string()));
                                    }
                                }
                            }
                            let mut handler_ref = menu_handler_clone1.lock().await;
                            handler_ref.to_previous_page();

                            match launch_mode {
                                LaunchMode::BundleSnipe => {
                                    let bundle_flavor_option_page =
                                        get_bundle_snipe_flavor_option_page(
                                            &mut handler_ref,
                                            keypair,
                                            metadata,
                                            name,
                                            symbol,
                                            launch_mode,
                                            (amount * LAMPORTS_PER_SOL as f64) as u64,
                                            sol_balances_array,
                                        );
                                    handler_ref.change_page(bundle_flavor_option_page);
                                }
                                _ => {
                                    display_info_page(
                                        info_segments,
                                        String::from("Overview"),
                                        &mut handler_ref,
                                        Some(OptionCallback::LaunchToken((
                                            keypair,
                                            metadata,
                                            (name, symbol),
                                            launch_mode,
                                            (amount * LAMPORTS_PER_SOL as f64) as u64,
                                            sol_balances_array,
                                            None,
                                            None,
                                        ))),
                                        None,
                                        Some(20),
                                    );
                                }
                            }

                            //here we move the to the final showcase page after inputting dev balance and it being validated
                        }
                    } else if let Err(e) = dev_balance {
                        let mut handler_ref = menu_handler_clone1.lock().await;
                        handler_ref.to_previous_page();
                        display_balance_fetch_error_page(&mut handler_ref);
                        spawn_error_input_listener(menu_handler_clone2, 10);
                    }
                    //    let dev_balance =
                });
            } else if let Err(e) = is_valid_input {
                //loading_handler_ref.to_previous_page();
                display_error_page(
                    e,
                    Some(String::from("Input Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );

                spawn_error_input_listener(menu_handler_clone2, 5);
            }
        }

        OptionCallback::ValidateSplitBundleDelay((
            keypair,
            metadata,
            (name, symbol),
            launch_mode,
            dev_buy,
            amounts,
            inputted_delay,
        )) => {
            ////
        }

        OptionCallback::LaunchToken((
            token_keypair,
            metadata_uri,
            (name, symbol),
            launch_mode,
            dev_buy,
            amounts,
            cto_coin,
            split_bundle_config,
        )) => {
            let session_alu_format = load_most_recent_lut();
            if let Err(ref e) = session_alu_format {
                //let mut handler_ref = menu_handler_clone1.lock().await;
                loading_handler_ref.to_previous_page();
                display_error_page(
                    e.clone(),
                    Some(String::from("lookup Table Error")),
                    None,
                    Some(5),
                    &mut loading_handler_ref,
                    false, //menu_handler_clone2
                );
                spawn_error_input_listener(menu_handler_clone2, 5);
                return;
            }

            let min_threshold: u64 = 1_000_000_0; // 1_000_000

            //check validity of amounts in wallets if its not a dev or decoy coin
            match launch_mode {
                LaunchMode::Decoy => {}
                LaunchMode::DevOnly => {}
                _ => {
                    let has_invalid_amounts = amounts.iter().all(|&value| value >= min_threshold);
                    if has_invalid_amounts {
                        //let mut handler_ref = menu_handler_clone1.lock().await;
                        loading_handler_ref.to_previous_page();
                        display_error_page(
                            String::from("Some Wallets have invalid buy amounts"),
                            Some(String::from("Buy Amounts Error")),
                            Some(vec![String::from(
                                "All buy amounts need to be >= 0.001 SOL",
                            )]),
                            Some(5),
                            &mut loading_handler_ref,
                            false, //menu_handler_clone2
                        );
                        spawn_error_input_listener(menu_handler_clone2, 5);
                        return;
                    }
                }
            };

            display_loading_page(
                String::from("Preparing Bundle..."),
                &mut loading_handler_ref,
            );
            drop(loading_handler_ref);
            //info!("after pushing the loading page and droping the loading lock");
            tokio::spawn(async move {
                let table = session_alu_format.unwrap();

                let lut_address = Pubkey::from_str(&table.lookup_table).unwrap();
                //let mint = Pubkey::from_str(&table.mint).unwrap();
                let lut_data = connection.get_account_data(&lut_address).unwrap();
                let deserialized_lut_data = AddressLookupTable::deserialize(&lut_data).unwrap();

                let keys: Vec<Pubkey> = deserialized_lut_data
                    .addresses
                    .iter()
                    .map(|wallet| wallet.clone())
                    .collect();
                let session_lut = Arc::new(get_session_lut_data(keys, lut_address));
                let global_lut = Arc::new(get_global_lut_data());

                let manifest_name_copy = match launch_mode {
                    LaunchMode::CTO => name.clone(),
                    _ => String::from("-----"),
                };
                let manifest_symbol_copy = match launch_mode {
                    LaunchMode::CTO => symbol.clone(),
                    _ => String::from("-----"),
                };
                let manifest_uri_copy = match launch_mode {
                    LaunchMode::CTO => metadata_uri.clone(),
                    _ => String::from("-----"),
                };
                let manifest_mint = match launch_mode {
                    LaunchMode::CTO => cto_coin.unwrap(),
                    _ => token_keypair.pubkey(),
                };

                let url_link = format!("https://pump.fun/coin/{}", &manifest_mint);

                match launch_mode {
                    LaunchMode::Classic => {
                        //first of all im gonna do some usual preflight checks,
                        let bundle_wallets_validation = validate_and_retrieve_pump_bundler_keypairs(
                            &funding_wallet.pubkey(),
                            &dev_wallet.pubkey(),
                        );

                        if let Err(ref e) = bundle_wallets_validation {
                            let mut handler_ref = menu_handler_clone1.lock().await;
                            handler_ref.to_previous_page();
                            display_error_page(
                                e.clone(),
                                Some(String::from("Wallet Configuration Error")),
                                None,
                                Some(5),
                                &mut handler_ref,
                                false, //menu_handler_clone2
                            );
                            spawn_error_input_listener(menu_handler_clone2, 5);
                            return;
                        }

                        let wallets: Vec<Arc<Keypair>> = bundle_wallets_validation
                            .unwrap()
                            .iter()
                            .map(|s| Arc::new(Keypair::from_base58_string(&s)))
                            .collect();

                        let mut manifest_wallet_entries: Vec<LaunchManifestWalletEntry> =
                            vec![LaunchManifestWalletEntry {
                                wallet: Arc::clone(&dev_wallet),
                                initial_sol_investment: dev_buy,
                                return_on_investment: 0,
                            }];

                        manifest_wallet_entries.extend_from_slice(
                            &wallets
                                .iter()
                                .zip(amounts.iter())
                                .map(|(wallet, amount)| {
                                    let keypair = Arc::clone(wallet);
                                    LaunchManifestWalletEntry {
                                        wallet: keypair,
                                        initial_sol_investment: *amount,
                                        return_on_investment: 0,
                                    }
                                })
                                .collect::<Vec<LaunchManifestWalletEntry>>(),
                        );

                        let mut handler_ref = menu_handler_clone1.lock().await;
                        display_loading_page(
                            String::from("Sending Classic bundle..."),
                            &mut handler_ref,
                        );
                        drop(handler_ref);
                        // let connection_ref = Arc::clone(&connection);
                        let funder_ref = Arc::clone(&funding_wallet);
                        let dev_ref = Arc::clone(&dev_wallet);
                        tokio::spawn(async move {
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

                            let global_config_lock = global_config.read().await; // Get the write lock
                            let mut funding_type: String = String::from("");
                            if let Some(config_ref) = global_config_lock.as_ref() {
                                funding_type = config_ref.funding_strategy.clone();
                            }
                            drop(global_config_lock);
                            let is_in_contract = funding_type.as_str() == "in-contract";

                            let token_keypair_ref = Arc::clone(&token_keypair);
                            let pump_keys = derive_all_pump_keys(
                                &funder_keypair_ref.pubkey(),
                                token_keypair_ref.pubkey(),
                            );
                            let default_curve = BondingCurve::default();

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

                                    let bundle = get_classic_launch_bundle(
                                        Arc::clone(&token_keypair_ref),
                                        Arc::clone(&blockhash_manager),
                                        &default_curve,
                                        &wallets,
                                        &amounts,
                                        is_in_contract,
                                        &metadata_uri,
                                        &name,
                                        &symbol,
                                        &pump_keys,
                                        Arc::clone(&global_lut),
                                        Arc::clone(&session_lut),
                                        Arc::clone(&dev_ref),
                                        if dev_buy == 0 { None } else { Some(dev_buy) },
                                        Arc::clone(&funder_ref),
                                        tip,
                                    );

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
                            let notify_websocket_opened_ref = Arc::clone(&notify_websocket_opened);

                            tokio::spawn(async move {
                                let (ws_stream, _) = connect_async(wss_url.as_ref()).await.unwrap();
                                let (mut write, mut read) = ws_stream.split();

                                let subscription_message =
                                    get_account_subscription_message(&token_keypair.pubkey());

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
                                handler_ref.return_to_pump_main_menu();
                                if account_monitor_timeout.load(Ordering::Relaxed) {
                                    display_bundle_timeout_error_page(&mut handler_ref);
                                    drop(handler_ref);
                                    //spawn_error_input_listener(menu_handler_clone1, 10);
                                } else {
                                    //here we create the launch manifest
                                    let manifest = LaunchManifest::new(
                                        launch_mode,
                                        funding_type.clone(),
                                        lut_address,
                                        manifest_mint,
                                        true,
                                        SimpleMetadata {
                                            name: manifest_name_copy,
                                            symbol: manifest_symbol_copy,
                                            uri: manifest_uri_copy,
                                        },
                                        manifest_wallet_entries,
                                        tip,
                                    );
                                    //back any existing manifest if any and then create save the new one
                                    let _ = backup_files(BackupType::LaunchManifest);
                                    let _ = create_launch_manifest(manifest);

                                    //then we save the funding manifest
                                    display_info_page(
                                        vec![
                                            InfoSegment::Emphasized(format!("Success!")),
                                            InfoSegment::StringSplitInfo((String::from("Link"), url_link)),
                                            InfoSegment::Normal(format!("")),
                                            InfoSegment::Normal(format!("Launch manifest created, you can now start tracking your launch")),
                                            InfoSegment::Normal(format!("metrics live and manage wallets, run bumps, comment and more with")),
                                            InfoSegment::Normal(format!("the launch tracker UI.")),
                                            ],
                                        String::from("Success."),
                                        &mut handler_ref,
                                        Some(OptionCallback::ReturnToMenu),
                                        None,
                                        None,
                                    );
                                }
                            });
                        });
                    }

                    _ => {}
                }
            });
        }

        OptionCallback::SignalManualBuy((flag, _curve_provider)) => {
            ////
        }

        _ => {}
    }
}
