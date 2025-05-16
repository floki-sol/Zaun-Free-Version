use borsh::BorshDeserialize;
use bs58::encode;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use event::PopKeyboardEnhancementFlags;
use log::info;
use num_traits::Zero;
use rand::seq::SliceRandom;
use rand::Rng;
use regex::Regex;
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::RpcError;
use solana_client::rpc_response::RpcResult;
use solana_sdk::account::Account;
use solana_sdk::address_lookup_table::AddressLookupTableAccount;
use solana_sdk::borsh::try_from_slice_unchecked;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_program;
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Mint;
use std::borrow::{Borrow, Cow};
use std::fmt::{self, format};
use std::fs::{self, File, OpenOptions};
use std::io::{self, stdout, Read, Write};
use std::ops::{Add, Div, Mul};
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::sleep;

use crossterm::*;
use terminal::disable_raw_mode;

use crate::cli::menu::MenuHandler;
use crate::constants::general::{
    AMM_V4, AMM_V4_AUTHORITY, AMM_V4_CONFIG, AMM_V4_FEES, DEFAULT_WEBHOOK_IMAGE_PLACEHORLDER,
    EVENT_AUTH, FEE_RECEPIENT, GENERAL_COINS_ENDPOINT, GLOBAL_LUT_ADDRESS, GLOBAL_STATE,
    MEMO_PROGRAM_ID, METAPLEX_METADATA, MINT_AUTH, OPENBOOK, PUMP_PROGRAM_ID, PUMP_TOKEN_DECIMALS,
    SYSVAR_RENT_ID,
};
use crate::loaders::global_config_loader::GlobalConfig;
use crate::loaders::launch_manifest_loader::LaunchManifest;
use crate::loaders::metadata_loader::Metadata;

use super::backups::backup_files;
use super::blockhash_manager::RecentBlockhashManager;
use super::bonding_curve_provider::BondingCurve;
use super::pdas::{get_bonding_curve, get_pump_creator_vault};

#[derive(Clone, Debug)]
pub enum WalletType {
    DevWallet,
    FundingWallet,
    BumpWallet,
    BundleWalletSol,
    BundleWalletTokens,
    Another(String),
}

#[derive(Clone, Debug)]
pub enum WalletsFundingType {
    Static(String),
    Distribution(String),
    HumanLikeDistribution(String),
    MinMax(String),
    Interactive,
    Initiate(Vec<f64>),
}

impl WalletsFundingType {
    // Method to update the inner value if the variant is `ConfirmGenerationInput`
    pub fn update_inner_value(&mut self, new_value: String) {
        if let WalletsFundingType::Static(ref mut inner_value) = self {
            *inner_value = new_value;
        } else if let WalletsFundingType::Distribution((ref mut inner_value)) = self {
            *inner_value = new_value;
        } else if let WalletsFundingType::HumanLikeDistribution((ref mut inner_value)) = self {
            *inner_value = new_value;
        } else if let WalletsFundingType::MinMax((ref mut inner_value)) = self {
            *inner_value = new_value;
        }
    }
}

#[derive(Clone, Debug)]
enum AmountCategory {
    Fractional,  // 0.1, 0.25, 0.5
    WholeNumber, // 1, 5, 10, 25, etc.
}

#[derive(Clone, Debug)]

pub enum PercentileGroup {
    P25, // 25th Percentile (very low tips | very low landing rates)
    P50, // 50th Percentile (low tips | low landing rates)
    P75, // 75th Percentile (default | decent landing rates)
    P95, // 95th Percentile (high tips | high landing rates)
    P99, // 100th Percentile (VERY high tips | Near guaranteed landing rates)
}

#[derive(Clone, Debug)]
pub enum FundingStrategy {
    PreFund,
    InContract,
}
impl fmt::Display for FundingStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            FundingStrategy::InContract => "In-contract",
            FundingStrategy::PreFund => "Pre-fund",
        };
        write!(f, "{}", text)
    }
}

// Function to perform shutdown tasks (restore terminal, etc.)
pub fn graceful_shutdown(
    menu_handler: &mut MenuHandler,
    blockhash_manager: &Arc<RecentBlockhashManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Disable raw mode to restore terminal settings
    disable_raw_mode()?;
    blockhash_manager.stop();

    // Clear the screen and reset cursor position
    menu_handler.terminal.clear()?;
    menu_handler.terminal.show_cursor()?;
    menu_handler.terminal.set_cursor(0, 0)?;

    // Create stdout and the backend for tui.
    let mut stdout = stdout();
    //execute!(stdout, PopKeyboardEnhancementFlags)?;
    execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    std::process::exit(0);
    //println!("Application shutdown complete.");
}

pub fn process_vanity_file(file_path: &str) {
    match std::fs::canonicalize(&adjust_file_path(file_path)) {
        Ok(absolute_path) => {
            // Read file content
            match fs::read_to_string(&absolute_path) {
                Ok(data) => {
                    // Parse JSON data
                    match serde_json::from_str::<Value>(&data) {
                        Ok(keypair) => {
                            let bytes: Vec<u8> = keypair
                                .as_array() // Attempt to convert the Value to an array
                                .map(|arr| {
                                    arr.iter() // Iterate over the array elements
                                        .filter_map(|v| v.as_u64()) // Ensure each element is a u64 and filter out non-u64 values
                                        .map(|v| v as u8) // Cast each u64 value to u8
                                        .collect() // Collect into a Vec<u8>
                                })
                                .unwrap();

                            //info!("final bytes: {:?}", &bytes);

                            // Convert to Base58
                            //info!("generated pair array: {:?}", &keypair);
                            //let keypair_bytes = serde_json::to_vec(&keypair).unwrap();
                            //info!("rusted pair vec: {:?}", &keypair_bytes);
                            //let keypair_obj = Keypair::from_bytes(&keypair_bytes).unwrap();
                            //info!("actual keypair object: {:?}", &keypair_obj);
                            //let keypair_bytes = keypair_obj.to_bytes();

                            let base58_keypair = encode(bytes).into_string();

                            //info!("base58_keypair: {base58_keypair}");

                            // Print formatted message
                            //let base_name = absolute_path
                            //    .file_name()
                            //    .unwrap_or_default()
                            //    .to_string_lossy()
                            //    .split('.')
                            //    .next()
                            //    .unwrap_or_default();
                            //println!(
                            //    "{}",
                            //    format!("Found keypair for address: {}", base_name)
                            //);

                            // Update the JSON file (example function, you need to implement it)
                            update_vanity_json_file(&base58_keypair);

                            // Remove the file after processing
                            if let Err(e) = fs::remove_file(&absolute_path) {
                                eprintln!("Failed to delete keypair file: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to parse JSON from file: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read file {}: {}", file_path, e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to resolve path {}: {}", file_path, e);
        }
    }
}

fn update_vanity_json_file(base58_keypair: &str) {
    let file_path = &adjust_file_path("temp/vanity-keypairs.json");

    let dir_path_str = &adjust_file_path("temp/");
    let dir_path = Path::new(dir_path_str);
    if !dir_path.exists() {
        fs::create_dir_all(dir_path)
            .map_err(|err| format!("Failed to create directories: {}", err));
    }

    let mut json_array: Vec<String> = Vec::new();

    // Check if the file exists
    if Path::new(file_path).exists() {
        // Read the existing data from the file
        match fs::read_to_string(file_path) {
            Ok(existing_data) => {
                // Parse the existing JSON data into a Vec<String>
                match serde_json::from_str::<Vec<String>>(&existing_data) {
                    Ok(existing_json_array) => {
                        json_array.extend(existing_json_array);
                    }
                    Err(e) => {
                        //eprintln!("Failed to parse existing JSON: {}", e);
                    }
                }
            }
            Err(e) => {
                //eprintln!("Failed to read file {}: {}", json_file_path, e);
            }
        }
    }

    // Add the new base58 keypair to the array
    json_array.push(base58_keypair.to_string());

    // Write the updated JSON array back to the file
    match fs::write(
        file_path,
        serde_json::to_string_pretty(&json_array).unwrap(),
    ) {
        Ok(_) => {
            //println!(
            //    "{}",
            //    format!(
            //        "Updated stash with new private key: {}",
            //        &base58_keypair[0..18]
            //    ).color(Color::BrightMagenta)
            //);
        }
        Err(e) => {
            //eprintln!("Failed to write to file {}: {}", file_path, e);
        }
    }
}

pub fn create_funding_manifest(amounts: Vec<f64>) -> Result<(), String> {
    // Define the file path
    let file_path = &adjust_file_path("temp/funding-manifest.json");

    let dir_path_str = &adjust_file_path("temp/");
    let dir_path = Path::new(dir_path_str);
    if !dir_path.exists() {
        fs::create_dir_all(dir_path)
            .map_err(|err| format!("Failed to create directories: {}", err))?;
    }

    // Write the JSON to the file
    let json_data = serde_json::to_string_pretty(&amounts)
        .map_err(|err| format!("Failed to serialize data: {}", err))?;
    let mut file =
        File::create(file_path).map_err(|err| format!("Failed to create file: {}", err))?;
    file.write_all(json_data.as_bytes())
        .map_err(|err| format!("Failed to write to file: {}", err))?;

    Ok(())
}

pub fn retrieve_funding_manifest() -> Result<Vec<f64>, String> {
    // Define the file path
    let file_path = &adjust_file_path("temp/funding-manifest.json");

    // Check if the file exists
    if !Path::new(file_path).exists() {
        return Err("Funding manifest file does not exist.".to_string());
    }

    // Read the file contents
    let file_content =
        fs::read_to_string(file_path).map_err(|err| format!("Failed to read the file: {}", err))?;

    // Deserialize the JSON data
    let amounts: Vec<f64> = serde_json::from_str(&file_content)
        .map_err(|err| format!("Failed to parse JSON: {}", err))?;
    //info!("{:#?}", amounts);

    let min_threshold: f64 = 0.001; // 1_000_000
    let has_invalid_amounts = amounts.iter().any(|&value| value < min_threshold);

    if has_invalid_amounts {
        return Err(String::from(
            "Invalid Buy Amounts in manifest. All amounts need to be >= 0.001 Sol.",
        ));
    }

    //info!("{:?}", &amounts);

    Ok(amounts)
}

pub fn can_use_lut(
    wallets: &Vec<String>,
    dev_address: &Pubkey,
    lut_addresses: Vec<Pubkey>,
    token: Pubkey,
) -> bool {
    let mut wallets_atas: Vec<Pubkey> = wallets
        .iter()
        .map(|wallet| {
            get_associated_token_address(&Keypair::from_base58_string(wallet).pubkey(), &token)
        })
        .collect();

    let creator_vault =
        get_pump_creator_vault(dev_address, &Pubkey::from_str(PUMP_PROGRAM_ID).unwrap());
    wallets_atas.push(creator_vault);

    for ata in wallets_atas {
        if !lut_addresses.contains(&ata) {
            return false;
        }
    }

    true
}

pub fn write_pump_keypairs_to_bundler_file(num_keypairs: usize) -> Result<(), String> {
    //first Im gonna backup existing wallets
    backup_files(super::backups::BackupType::BundleWallets)
        .map_err(|e| format!("Failed to backup wallets: {}", e))?;

    let mut keypair_list: Vec<String> = Vec::new();

    for _ in 0..num_keypairs {
        // Generate a new keypair
        let keypair = Keypair::new();

        // Encode the keypair in Base58 and add to the list as a string
        let encoded_keypair = bs58::encode(keypair.to_bytes()).into_string();
        keypair_list.push(encoded_keypair);
    }

    // Open the file (create if it doesn't exist)
    let file_path: &str = &adjust_file_path("configurations/pump/bundler-wallets.json");
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(file_path)
        .map_err(|e| format!("{}", e))?;

    // Serialize and write the plain list of strings to the file
    let json_data = serde_json::to_string_pretty(&keypair_list).map_err(|e| format!("{e}"))?;

    // Write to file
    let mut file = file;
    file.write_all(json_data.as_bytes())
        .map_err(|e| format!("{e}"))?;

    Ok(())
}

pub fn create_launch_manifest(manifest: LaunchManifest) -> Result<(), String> {
    // Open the file (create if it doesn't exist)
    let file_path: &str = &adjust_file_path("configurations/pump/launch-manifest.json");
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(file_path)
        .map_err(|e| String::from("Failed to create launch manifest file"))?;

    // Serialize and write the plain list of strings to the file
    let serialized_manifest = serde_json::to_string_pretty(&manifest)
        .map_err(|e| String::from("failed to serialize launch manifest"))?;
    // Write to file
    let mut file = file;
    file.write_all(serialized_manifest.as_bytes())
        .map_err(|e| String::from("Failed to write to launch manifest file"))?;

    Ok(())
}

pub fn parse_token_address(input: &str) -> Result<String, String> {
    // Define a regex pattern to match token URLs
    let pattern = r"(?:https?://)?(?:www\.)?pump\.fun\/(?:coin\/)?([a-zA-Z0-9]{32,44})|(?:https?://)?(?:www\.)?bullx\.io/.*?address=([a-zA-Z0-9]{32,44})";
    let re =
        Regex::new(pattern).map_err(|_| format!("Invalid regex pattern for input: {}", input))?;

    // Try to capture the address from the URL pattern
    if let Some(captures) = re.captures(input) {
        // Extract the token address from the first or second capture group
        if let Some(token_address) = captures.get(1) {
            return Ok(token_address.as_str().to_string());
        } else if let Some(token_address) = captures.get(2) {
            return Ok(token_address.as_str().to_string());
        }
    }

    // If no pattern is matched, treat the input as a regular address
    //if input.len() == 32 || input.len() == 44 {
    Ok(input.to_string())
    //}

    // Return a generic error message if no valid pattern is found
}

pub async fn is_pump_token(client: Arc<RpcClient>, token: &Pubkey) -> Result<(), String> {
    // Define the bonding curve public key
    let bonding_curve = get_bonding_curve(token, &Pubkey::from_str(PUMP_PROGRAM_ID).unwrap());

    // Fetch the account data
    match client.get_account_data(bonding_curve.borrow()) {
        Ok(data) => {
            // If account data exists, return success
            Ok(())
        }
        Err(err) => {
            // Match specific error types
            match err.kind() {
                solana_client::client_error::ClientErrorKind::RpcError(e) => match e {
                    RpcError::ForUser(err) => {
                        Err(format!("Invalid Pump token address: {:?} ", err))
                    }
                    _ => Err("An Unknown error has occured".to_string()),
                },
                _ => Err(format!("{:?}", err)),
            }
        }
    }
}

pub fn extract_lamports(notification: &str) -> Option<u64> {
    let json: Value = serde_json::from_str(notification).ok()?;
    // Access the lamports field in the known structure
    json.get("params")
        .and_then(|params| params.get("result"))
        .and_then(|result| result.get("value"))
        .and_then(|value| value.get("lamports"))
        .and_then(|lamports| lamports.as_u64())
}

pub fn extract_data(notification: &str) -> Option<String> {
    let json: Value = serde_json::from_str(notification).ok()?;
    // Access the lamports field in the known structure
    json.get("params")
        .and_then(|params| params.get("result"))
        .and_then(|result| result.get("value"))
        .and_then(|value| value.get("data"))
        .and_then(|data| data.as_array())
        .and_then(|v| Some(v[0].to_string()))
}

pub fn get_account_subscription_message(key: &Pubkey) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "accountSubscribe",
        "params": [
            key.to_string(),
            {
                "encoding": "jsonParsed",
                "commitment": "processed",
            }
        ]
    })
}

pub fn get_transaction_logs_subscription_message(key: &Pubkey, commitment: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "logsSubscribe",
        "params": [
            {
                "mentions": [key.to_string()]
            },
            {
                "encoding": "jsonParsed",
                "commitment": commitment
            }
        ]
    })
}

pub fn is_valid_decimal_string(
    amount: String,
    min_amount: f64,
    max_amount: f64,
) -> Result<f64, String> {
    match amount.trim().parse::<f64>() {
        Ok(number) => {
            if number >= min_amount && number <= max_amount {
                Ok(number)
            } else {
                Err(format!(
                    "Amount must be between {} and {}.",
                    min_amount, max_amount
                ))
            }
        }
        Err(_) => Err(String::from(
            "Input must be a numeric decimal number. (ex: 0.001, 6.9, 10.0)",
        )),
    }
}

pub fn is_valid_small_integer_string(
    amount: String,
    min_amount: u8,
    max_amount: u8,
) -> Result<u8, String> {
    match amount.parse::<u8>() {
        Ok(number) => {
            if number >= min_amount && number <= max_amount {
                Ok(number)
            } else {
                Err(format!(
                    "Amount must be between {} and {}.",
                    min_amount, max_amount
                ))
            }
        }
        Err(_) => {
            // Handle the case where the input is not a valid number
            Err(format!(
                "Input must be a numeric whole number between {} and {}.",
                min_amount, max_amount
            ))
        }
    }
}

pub fn get_random_normalized_amounts(
    number_of_wallets: usize,
    total_amount: f64,
    min_amount: f64,
) -> Result<Vec<f64>, String> {
    // Validate input
    if number_of_wallets as f64 * min_amount > total_amount {
        return Err(format!("Invalid amount: Must be at least {}", min_amount));
    }

    // Initialize the vector with minimum amounts
    let mut amounts = vec![min_amount; number_of_wallets];
    // Calculate the remaining amount to be distributed
    let mut remaining_amount = total_amount - (number_of_wallets as f64 * min_amount);
    // Random number generator
    let mut rng = rand::thread_rng();
    // Randomly distribute the remaining amount
    for i in 0..number_of_wallets - 1 {
        let max_allocatable = remaining_amount / (number_of_wallets - i) as f64;
        let random_allocation = rng.gen::<f64>() * max_allocatable;

        amounts[i] += random_allocation;
        remaining_amount -= random_allocation;
    }

    // Add the remaining amount to the last wallet
    amounts[number_of_wallets - 1] += remaining_amount;

    // Round to 4 decimal places
    let rounded_amounts: Vec<f64> = amounts
        .into_iter()
        .map(|amount| (amount * 10000.0).floor() / 10000.0)
        .collect();

    Ok(rounded_amounts)
}

pub fn get_random_range_amounts(
    number_of_wallets: usize,
    min_amount: f64,
    max_amount: f64,
) -> Result<Vec<f64>, String> {
    // Validate input
    if min_amount >= max_amount {
        return Err(String::from("Min amount must be less than max amount"));
    }

    let mut rng = rand::thread_rng();
    let amounts: Vec<f64> = (0..number_of_wallets)
        .map(|_| {
            // Generate random amount between min and max
            let random = rng.gen::<f64>();
            let amount = min_amount + (random * (max_amount - min_amount));

            // Round to 4 decimal places
            (amount * 10000.0).floor() / 10000.0
        })
        .collect();

    Ok(amounts)
}

fn determine_distribution_strategy(
    total_amount: f64,
    number_of_wallets: usize,
) -> Vec<(AmountCategory, f64)> {
    let average_amount_per_wallet = total_amount / number_of_wallets as f64;

    match average_amount_per_wallet {
        // Very low total amount: mostly fractional
        x if x < 1.0 => vec![
            (AmountCategory::Fractional, 0.7),
            (AmountCategory::WholeNumber, 0.3),
        ],

        // Small amounts (1-10 per wallet): mix of fractional and small whole numbers
        x if x >= 1.0 && x < 10.0 => vec![
            (AmountCategory::Fractional, 0.4),
            (AmountCategory::WholeNumber, 0.6),
        ],

        // Medium amounts (10-50 per wallet): mostly whole numbers
        x if x >= 10.0 && x < 50.0 => vec![
            (AmountCategory::WholeNumber, 0.8),
            (AmountCategory::Fractional, 0.2),
        ],

        // Large amounts (50+ per wallet): larger whole numbers
        _ => vec![
            (AmountCategory::WholeNumber, 0.9),
            (AmountCategory::Fractional, 0.1),
        ],
    }
}

pub fn get_human_readable_amounts(
    number_of_wallets: usize,
    total_amount: f64,
    min_amount: f64,
) -> Result<Vec<f64>, String> {
    // Validate input
    if number_of_wallets as f64 * min_amount > total_amount {
        return Err(format!("Invalid amount: Must be at least {}", min_amount));
    }

    // Determine distribution strategy based on total amount and wallets
    let distribution_strategy = determine_distribution_strategy(total_amount, number_of_wallets);

    // Predefined human-readable amounts
    let fractional_amounts = vec![0.1, 0.25, 0.5, 0.75];
    let whole_number_amounts = vec![1, 2, 3, 5, 10, 25];

    // Random number generator
    let mut rng = rand::thread_rng();

    // Initialize amounts array with minimum amounts
    let mut amounts = vec![min_amount; number_of_wallets];

    // Calculate the remaining amount to be distributed
    let mut remaining_amount = total_amount - (number_of_wallets as f64 * min_amount);

    // Prepare distribution strategy amounts
    let mut strategy_amounts: Vec<f64> = Vec::new();
    for (category, weight) in distribution_strategy {
        match category {
            AmountCategory::Fractional => {
                strategy_amounts.extend(fractional_amounts.iter().map(|&a| a * weight));
            }
            AmountCategory::WholeNumber => {
                strategy_amounts.extend(whole_number_amounts.iter().map(|&a| a as f64 * weight));
            }
        }
    }

    // Distribute remaining amount
    for i in 0..number_of_wallets - 1 {
        // Filter amounts that don't exceed remaining amount
        let possible_amounts: Vec<&f64> = strategy_amounts
            .iter()
            .filter(|&&x| x <= remaining_amount)
            .collect();

        if possible_amounts.is_empty() {
            break;
        }

        // Choose a random amount from possible amounts
        let &chosen_amount = possible_amounts
            .choose(&mut rng)
            .expect("Should have at least one possible amount");

        amounts[i] += chosen_amount;
        remaining_amount -= chosen_amount;
    }

    // Add remaining amount to last wallet
    amounts[number_of_wallets - 1] += remaining_amount;

    // Round to 2 decimal places
    let rounded_amounts: Vec<f64> = amounts
        .into_iter()
        .map(|amount| (amount * 100.0).round() / 100.0)
        .collect();

    // Final validation
    let total_rounded: f64 = rounded_amounts.iter().sum();
    if (total_rounded - total_amount).abs() > 0.01 {
        return Err("Failed to distribute amounts exactly".to_string());
    }

    Ok(rounded_amounts)
}

pub async fn get_lookup_table_creation_cost(
    wallet_count: usize,
    connection: Arc<RpcClient>,
) -> Result<u64, String> {
    let total_bytes = 56 + ((wallet_count) * 32) + 96; // ;;
    let rent_cost = connection
        .get_minimum_balance_for_rent_exemption(total_bytes)
        .map_err(|e| {
            format!(
                "failed to get lookup table creation costs: {}",
                e.to_string()
            )
        })?;
    Ok(rent_cost)
}

pub async fn get_balance(connection: Arc<RpcClient>, addy: &Pubkey) -> Result<(u64), String> {
    // Get the balance of the wallet in lamports
    let balance: RpcResult<u64> = connection.get_balance_with_commitment(
        addy,
        CommitmentConfig {
            commitment: CommitmentLevel::Processed,
        },
    );

    match balance {
        Ok(lamports) => Ok(lamports.value),
        Err(e) => Err(String::from("Failed to fetch Sol balance.")),
    }
}

pub async fn get_balances_for_wallets(
    rpc_client: Arc<RpcClient>,
    wallet_keypairs: Arc<Vec<String>>,
) -> Result<Vec<u64>, String> {
    let pkey_vec: Vec<Pubkey> = wallet_keypairs
        .iter()
        .map(|pkey| Keypair::from_base58_string(&pkey).pubkey())
        .collect();

    // Get all account info in a single RPC call
    let accounts = rpc_client
        .get_multiple_accounts(&pkey_vec)
        .map_err(|err| format!("Failed to fetch balances {}", err))?;

    // Collect the results into a vector of balances or return an error if any failed
    let mut balances_res: Vec<u64> = Vec::new();
    for account in accounts {
        match account {
            Some(acc) => {
                balances_res.push(acc.lamports);
            }
            None => balances_res.push(0), //return Err(err), // RPC or decoding error
        }
    }

    Ok(balances_res)
}

pub async fn get_token_balances_for_wallets(
    rpc_client: Arc<RpcClient>,
    wallet_keypairs: Arc<Vec<String>>,
    token_mint: Pubkey,
) -> Result<Vec<u64>, String> {
    // First, we'll get all the associated token account addresses
    let mut token_account_addresses = Vec::with_capacity(wallet_keypairs.len());

    for encoded_keypair in wallet_keypairs.iter() {
        // Decode the Base58 string and create a Keypair
        let decoded_bytes = bs58::decode(encoded_keypair)
            .into_vec()
            .map_err(|_| format!("Failed to decode Base58 for keypair: {}", encoded_keypair))?;

        let keypair = Keypair::from_bytes(&decoded_bytes).map_err(|_| {
            format!(
                "Invalid Solana keypair for decoded bytes: {}",
                encoded_keypair
            )
        })?;

        let wallet_pubkey = keypair.pubkey();

        // Get the associated token account address
        let associated_token_address =
            spl_associated_token_account::get_associated_token_address(&wallet_pubkey, &token_mint);

        token_account_addresses.push(associated_token_address);
    }

    // Get all account info in a single RPC call
    let accounts = rpc_client
        .get_multiple_accounts(&token_account_addresses)
        .map_err(|err| format!("Failed to fetch token accounts: {}", err))?;

    // Process the results
    let mut balances = Vec::with_capacity(accounts.len());

    for account in accounts {
        match account {
            Some(account_info) => {
                // Use the SPL Token Account deserializer
                match spl_token::state::Account::unpack_unchecked(&account_info.data) {
                    Ok(token_account) => balances.push(token_account.amount),
                    Err(_) => balances.push(0), // Account exists but failed to parse
                }
            }
            None => balances.push(0), // Account doesn't exist
        }
    }

    Ok(balances)
}

pub fn get_associated_accounts(wallets: &Vec<Pubkey>, mint: Pubkey) -> Vec<Pubkey> {
    wallets
        .iter()
        .map(|wallet| get_associated_token_address(wallet, &mint))
        .collect::<Vec<Pubkey>>()
}

pub fn spawn_bundle_timeout_task(
    global_config: Arc<RwLock<Option<GlobalConfig>>>,
    timeout_flag: Arc<AtomicBool>,
    extra_seconds: u64,
) {
    tokio::spawn(async move {
        let global_config_lock = global_config.read().await; // Get the write lock
        let mut timeout: u8 = 20;
        if let Some(config_ref) = global_config_lock.as_ref() {
            timeout = config_ref.bundle_timeout
        };
        drop(global_config_lock);
        sleep(Duration::from_secs(timeout as u64 + extra_seconds)).await;
        timeout_flag.store(true, Ordering::Relaxed);
    });
}

pub fn get_global_lut_data() -> AddressLookupTableAccount {
    AddressLookupTableAccount {
        key: Pubkey::from_str(GLOBAL_LUT_ADDRESS).unwrap(),
        addresses: vec![
            spl_token::ID,
            Pubkey::from_str(SYSVAR_RENT_ID).unwrap(),
            system_program::ID,
            spl_associated_token_account::ID,
            Pubkey::from_str(PUMP_PROGRAM_ID).unwrap(),
            Pubkey::from_str(METAPLEX_METADATA).unwrap(),
            Pubkey::from_str(MINT_AUTH).unwrap(),
            Pubkey::from_str(EVENT_AUTH).unwrap(),
            Pubkey::from_str("CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM").unwrap(),
            Pubkey::from_str(GLOBAL_STATE).unwrap(),
            Pubkey::from_str(MEMO_PROGRAM_ID).unwrap(),
            Pubkey::from_str(AMM_V4).unwrap(),
            Pubkey::from_str(AMM_V4_AUTHORITY).unwrap(),
            Pubkey::from_str(AMM_V4_FEES).unwrap(),
            Pubkey::from_str(AMM_V4_CONFIG).unwrap(),
            Pubkey::from_str(OPENBOOK).unwrap(),
            Pubkey::from_str(FEE_RECEPIENT).unwrap(),
        ],
    }
}

pub fn get_session_lut_data(addresses: Vec<Pubkey>, key: Pubkey) -> AddressLookupTableAccount {
    AddressLookupTableAccount { key, addresses }
}

pub fn calculate_pump_tokens_to_buy(
    sol_amount: u64,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    real_token_reserves: u64,
) -> u64 {
    if sol_amount == 0 || virtual_sol_reserves == 0 || virtual_token_reserves == 0 {
        return 0;
    }

    let sol_amount = sol_amount as u128;
    let virtual_sol_reserves = virtual_sol_reserves as u128;
    let virtual_token_reserves = virtual_token_reserves as u128;
    let real_token_reserves = real_token_reserves as u128;

    let mut token_amount: u128;

    let product = virtual_sol_reserves * virtual_token_reserves;
    //info!("{}", product);
    let new_sol_reserves = virtual_sol_reserves + sol_amount;
    //info!("{}", new_sol_reserves);
    let new_token_amount = product / new_sol_reserves + 1;
    //info!("{}", new_token_amount);
    token_amount = virtual_token_reserves - new_token_amount;
    //info!("{}", token_amount);
    token_amount = token_amount.min(real_token_reserves);

    token_amount as u64
}

pub fn fix_ipfs_url(url: &str) -> String {
    // Define the prefix to check
    let prefix = "https://ipfs.io/ipfs/";

    // Check if the URL starts with the prefix
    if url.starts_with(prefix) {
        // Extract the part after the prefix
        let ipfs_hash = &url[prefix.len()..];

        // Append it to the new base URL
        format!("https://pump.mypinata.cloud/ipfs/{}", ipfs_hash)
    } else {
        // Return the original string if it doesn't match
        url.to_string()
    }
}

pub async fn fetch_and_validate_metadata_uri(
    uri: &str,
) -> Result<(String, String, String), String> {
    // Fetch the URL
    let response = reqwest::get(uri).await.map_err(|e| e.to_string())?;
    //info!("{:#?}", response);

    // Check if the response status is OK (200)
    if !response.status().is_success() {
        return Err(format!("Failed to fetch URL: {}", response.status()));
    }

    // Get the response text
    let body = response.text().await.map_err(|e| e.to_string())?;
    let json: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    //info!("{:#?}", json);

    // Validate required fields

    let name = json.get("name").and_then(|v| v.as_str());
    if name.is_none() {
        return Err("Missing or invalid 'name' field".into());
    }
    let symbol = json.get("symbol").and_then(|v| v.as_str());
    if symbol.is_none() {
        return Err("Missing or invalid 'symbol' field".into());
    }

    let image = json.get("image").and_then(|v| v.as_str());
    if image.is_none() {
        return Err("Missing or invalid 'image' field".into());
    }
    // Optional fields - must be present, can be null
    for field in ["description", "twitter", "telegram", "website"] {
        match json.get(field) {
            Some(value) => {
                if !value.is_string() && !value.is_null() {
                    return Err(format!("'{}' must be a string or null.", field));
                }
            }
            None => {}
        }
    }

    //info!("{:?},{:?},{:?}", &name, &symbol, &image);
    Ok((
        name.unwrap().to_string(),
        symbol.unwrap().to_string(),
        image.unwrap().to_string(),
    ))
}

pub fn adjust_file_path(file_path: &str) -> String {
    let separator = if cfg!(target_os = "windows") {
        "\\"
    } else {
        "/"
    };

    file_path.replace("/", separator)
}

pub fn validate_discord_webhook_url(url: &str) -> Result<(), String> {
    // Check if URL is empty
    if url.is_empty() {
        return Err("Webhook URL cannot be empty".to_string());
    }

    let parsed_url = url::Url::parse(url).map_err(|_| "Invalid URL format".to_string())?;

    // Check if it's a Discord webhook URL
    if !is_valid_discord_webhook_domain(&parsed_url) {
        return Err("Invalid Discord domain".to_string());
    }

    Ok(())
}

// Helper function to validate Discord webhook domains
pub fn is_valid_discord_webhook_domain(url: &url::Url) -> bool {
    let valid_domains = [
        "discord.com",
        "discordapp.com",
        "ptb.discord.com",
        "ptb.discordapp.com",
        "canary.discord.com",
        "canary.discordapp.com",
    ];

    valid_domains.contains(&url.domain().unwrap_or(""))
}

pub async fn send_create_event_embed(
    webhook_url: &str,
    name: &str,
    ticker: &str,
    mint: &str,
    creator: &str,
    image_url: Option<&str>,
    tx_sig: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let current_timestamp = Utc::now().to_rfc3339();
    let embed = json!({
        "embeds": [{
            "title": "New coin detected",
            "description": "[Powered by Zaun](https://discord.gg/KJ8dHSvXZd)", // Clickable link
            "color": 800080, // Purple color
            "fields": [
                {
                    "name": "Name",
                    "value": name,
                    "inline": true
                },
                {
                    "name": "Ticker",
                    "value": ticker,
                    "inline": true
                },
                {
                    "name": "Mint",
                    "value": mint,
                    "inline": false
                },
                {
                    "name": "Creator",
                    "value": creator,
                    "inline": false
                },
                {
                    "name": "Links",
                    "value": format!("[Pump.fun](https://pump.fun/coin/{})  [Solscan](https://solscan.io/tx/{})", mint ,tx_sig),
                    "inline": false
                },
            ],
            "image": if let Some(image) = image_url {
                    serde_json::json!({
                        "url": image
                    })
                } else {
                    serde_json::json!({
                        "url": DEFAULT_WEBHOOK_IMAGE_PLACEHORLDER
                    })
            },
            "timestamp": current_timestamp,
            "author": {
                "name": "Zaun Monitor",
                "icon_url": "https://ipfs.io/ipfs/QmNa77rSVmsvxzZ9bBHdVnZDi4STX8iZbdUccW8sUsai8z" // Optional
            }
        }]
    });

    let _ = client.post(webhook_url).json(&embed).send().await?;

    //println!("Discord response status: {}", response.status());
    Ok(())
}

pub async fn send_new_koth_event_embed(
    webhook_url: &str,
    name: &str,
    ticker: &str,
    mint: &str,
    creator: &str,
    image_url: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let current_timestamp = Utc::now().to_rfc3339();
    let embed = json!({
        "embeds": [{
            "title": "New King of The Hill",
            "description": "[Powered by Zaun](https://discord.gg/KJ8dHSvXZd)", // Clickable link
            "color": 800080, // Purple color
            "fields": [
                {
                    "name": "Name",
                    "value": name,
                    "inline": true
                },
                {
                    "name": "Ticker",
                    "value": ticker,
                    "inline": true
                },
                {
                    "name": "Mint",
                    "value": mint,
                    "inline": false
                },
                {
                    "name": "Creator",
                    "value": creator,
                    "inline": false
                },
                {
                    "name": "Links",
                    "value": format!("[Pump.fun](https://pump.fun/coin/{}) [Solscan](https://solscan.io/token/{})", mint, mint),
                    "inline": false
                },
            ],
            "image": if let Some(image) = image_url {
                    serde_json::json!({
                        "url": image
                    })
                } else {
                    serde_json::json!({
                        "url": DEFAULT_WEBHOOK_IMAGE_PLACEHORLDER
                    })
            },
            "timestamp": current_timestamp,
            "author": {
                "name": "Zaun Monitor",
                "icon_url": "https://ipfs.io/ipfs/QmNa77rSVmsvxzZ9bBHdVnZDi4STX8iZbdUccW8sUsai8z" // Optional
            }
        }]
    });

    let _ = client.post(webhook_url).json(&embed).send().await?;

    //println!("Discord response status: {}", response.status());
    Ok(())
}

pub async fn send_new_migration_event(
    webhook_url: &str,
    name: &str,
    ticker: &str,
    mint: &str,
    image_url: Option<&str>,
    pool_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let current_timestamp = Utc::now().to_rfc3339();
    let embed = json!({
        "embeds": [{
            "title": "New Migration Detected",
            "description": "[Powered by Zaun](https://discord.gg/KJ8dHSvXZd)", // Clickable link
            "color": 800080, // Purple color
            "fields": [
                {
                    "name": "Name",
                    "value": name,
                    "inline": true
                },
                {
                    "name": "Ticker",
                    "value": ticker,
                    "inline": true
                },
                {
                    "name": "Mint",
                    "value": mint,
                    "inline": false
                },
                {
                    "name": "Pool ID",
                    "value": pool_id,
                    "inline": false
                },
                {
                    "name": "Links",
                    "value": format!("[Pump.fun](https://pump.fun/coin/{}) [Solscan](https://solscan.io/token/{})", mint, mint),
                    "inline": false
                },
            ],
            "image": if let Some(image) = image_url {
                    serde_json::json!({
                        "url": image
                    })
                } else {
                    serde_json::json!({
                        "url": DEFAULT_WEBHOOK_IMAGE_PLACEHORLDER
                    })
            },
            "timestamp": current_timestamp,
            "author": {
                "name": "Zaun Monitor",
                "icon_url": "https://ipfs.io/ipfs/QmNa77rSVmsvxzZ9bBHdVnZDi4STX8iZbdUccW8sUsai8z" // Optional
            }
        }]
    });

    let _ = client.post(webhook_url).json(&embed).send().await?;

    //println!("Discord response status: {}", response.status());
    Ok(())
}

pub fn split_tip(dev_percentage: f32, buy_percentage: f32, tip: u64) -> (u64, u64) {
    let dev_share = (tip as f64 * dev_percentage as f64).floor() as u64;
    let buy_share = tip - dev_share;
    (dev_share, buy_share)
}
