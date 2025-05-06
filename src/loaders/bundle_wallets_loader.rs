use bs58;
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::utils::misc::adjust_file_path;

pub fn validate_and_retrieve_pump_bundler_keypairs(
    funder: &Pubkey,
    dev: &Pubkey,
) -> Result<Vec<String>, String> {
    let file_path: &str = &adjust_file_path("configurations/pump/bundler-wallets.json");

    // Check if the file exists
    if !Path::new(file_path).exists() {
        return Err("bundler-wallets.json File not found.".to_string());
    }

    // Read the file content
    let file_content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(err) => return Err(format!("Failed to read the file: {}", err)),
    };

    // Parse the content as JSON
    let json_data: Value = match serde_json::from_str(&file_content) {
        Ok(json) => json,
        Err(err) => return Err(format!("Failed to parse JSON: {}", err)),
    };

    // Ensure the JSON is an array
    let keypair_array = match json_data.as_array() {
        Some(array) => array,
        None => return Err("Invalid format: JSON is not an array.".to_string()),
    };

    // Check that the array length is between 1 and 20
    if keypair_array.len() < 1 || keypair_array.len() > 20 {
        return Err("Invalid number of keypairs: Must be between 1 and 20.".to_string());
    }

    // Validate each entry in the array is a valid Base58-encoded string
    let mut keypairs = Vec::new();
    let mut unique_wallets: HashSet<Pubkey> = HashSet::new(); // To track unique public keys

    for entry in keypair_array {
        if let Some(encoded_keypair) = entry.as_str() {
            // Decode the Base58 string
            match bs58::decode(encoded_keypair).into_vec() {
                Ok(decoded_bytes) => {
                    // Check if it forms a valid Solana keypair

                    let pair = Keypair::from_bytes(&decoded_bytes).map_err(|e| {
                        format!(
                            "Invalid keypair found: '{}' is not a valid Solana keypair.",
                            encoded_keypair
                        )
                    })?;

                    let pubkey = pair.pubkey();
                    if !unique_wallets.insert(pubkey) {
                        let truncated_keypair = format!(
                            "{}...{}",
                            &encoded_keypair[..4], // First 4 characters
                            &encoded_keypair[encoded_keypair.len() - 4..] // Last 4 characters
                        );
                    
                        return Err(format!(
                            "Duplicate keypair found: Keypair '{}' is repeated.",
                            truncated_keypair
                        ));
                    }

                    if pubkey.eq(dev) {
                        return Err(String::from(
                            "bundle wallets cannot have dev keypair included",
                        ));
                    }

                    if pubkey.eq(funder) {
                        return Err(String::from(
                            "bundle wallets cannot have funder keypair included",
                        ));
                    }

                    keypairs.push(encoded_keypair.to_string());
                }
                Err(_) => {
                    return Err(format!(
                        "Invalid keypair found: '{}' is not valid Base58.",
                        encoded_keypair
                    ));
                }
            }
        } else {
            return Err("Invalid entry: Wallet array contains invalid elements.".to_string());
        }
    }

    // Return the valid list of keypairs
    Ok(keypairs)
}
