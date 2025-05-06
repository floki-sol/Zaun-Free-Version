use std::{fs, path::Path, str::FromStr, sync::Arc};

use crate::{constants::general::LaunchMode, utils::misc::adjust_file_path};
use chrono::Local;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

use super::serde_transformers::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LaunchManifestWalletEntry {
    #[serde(
        deserialize_with = "keypair_from_str",
        serialize_with = "keypair_to_string"
    )]
    pub wallet: Arc<Keypair>,
    pub initial_sol_investment: u64,
    pub return_on_investment: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SimpleMetadata {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LaunchManifest {
    pub date: String,
    #[serde(
        deserialize_with = "launch_mode_from_str",
        serialize_with = "launch_mode_to_string"
    )]
    pub launch_mode: LaunchMode,
    pub funding_type: String,
    #[serde(
        deserialize_with = "pubkey_from_str",
        serialize_with = "pubkey_to_string"
    )]
    pub lookup_table: Pubkey,
    #[serde(
        deserialize_with = "pubkey_from_str",
        serialize_with = "pubkey_to_string"
    )]
    pub mint: Pubkey,
    pub metadata: SimpleMetadata,
    pub owned_launch: bool,
    pub wallet_entries: Vec<LaunchManifestWalletEntry>,
    pub tip_used: u64,
}

impl LaunchManifest {
    pub fn new(
        mode: LaunchMode,
        funding_type: String,
        lookup_table: Pubkey,
        mint: Pubkey,
        owned_launch: bool,
        metadata: SimpleMetadata,
        wallet_entries: Vec<LaunchManifestWalletEntry>,
        tip_used: u64,
    ) -> Self {
        let date = Local::now().format("%Y-%m-%d").to_string();

        Self {
            date: date,
            launch_mode: mode,
            funding_type: funding_type,
            metadata: metadata,
            owned_launch: owned_launch,
            wallet_entries: wallet_entries,
            tip_used: tip_used,
            lookup_table: lookup_table,
            mint: mint,
        }
    }
}

pub fn validate_and_retrieve_launch_manifest() -> Result<LaunchManifest, String> {
    let file_path: &str = &adjust_file_path("configurations/pump/launch-manifest.json");

    // Check if the file exists
    if !Path::new(file_path).exists() {
        return Err("No launch manifest found".to_string());
    }

    // Read the file content
    let file_content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(err) => return Err(format!("Failed to read the file: {}", err)),
    };

    // Deserialize the JSON content into the GlobalConfig struct
    let manifest: LaunchManifest = serde_json::from_str(&file_content)
        .map_err(|e| format!("Invalid JSON format in launch-manifest.json: {}", e))?;

    Ok(manifest)
}

