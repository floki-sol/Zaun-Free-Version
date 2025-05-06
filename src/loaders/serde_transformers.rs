use std::{str::FromStr, sync::Arc};

use serde::{de, Deserialize, Deserializer, Serializer};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

use crate::{constants::general::LaunchMode, utils::comments_manager::CommentType};

pub fn pubkey_to_string<S>(pubkey: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let pubkey_str = pubkey.to_string();
    serializer.serialize_str(&pubkey_str)
}

pub fn pubkey_from_str<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Pubkey::from_str(&s).map_err(|_| {
        // Customize the error message here
        de::Error::custom(format!("Invalid Pubkey format: {}", s))
    })
}

pub fn keypair_to_string<S>(keypair: &Arc<Keypair>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let private_key = keypair.to_bytes();
    let private_key_str = bs58::encode(private_key).into_string();
    serializer.serialize_str(&private_key_str)
}

pub fn keypair_from_str<'de, D>(deserializer: D) -> Result<Arc<Keypair>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    let pair_byte_array = bs58::decode(&s)
        .into_vec()
        .map_err(|_| de::Error::custom(format!("Invalid base58 private key format {}", &s)))?;

    // Validate that the funding and dev wallet private keys are valid Solana private keys (base58 format)
    let pair = Keypair::from_bytes(&pair_byte_array.as_slice())
        .map_err(|_| de::Error::custom(format!("Invalid base58 private key format {}", &s)))?;

    Ok(Arc::new(pair))
}

pub fn launch_mode_to_string<S>(mode: &LaunchMode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let launch_mode_str = mode.to_string();
    serializer.serialize_str(&launch_mode_str)
}

pub fn launch_mode_from_str<'de, D>(deserializer: D) -> Result<LaunchMode, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    match s.to_lowercase().as_str() {
        "classic" => Ok(LaunchMode::Classic),
        "bundle snipe" => Ok(LaunchMode::BundleSnipe),
        "mass snipe" => Ok(LaunchMode::MassSnipe),
        "dev only" => Ok(LaunchMode::MassSnipe),
        _ => Err(de::Error::custom(format!("Invalid Launch mode: {}", s))),
    }
}

pub fn comment_type_to_string<S>(comment_type: &CommentType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let comment_type_str = comment_type.to_string();
    serializer.serialize_str(&comment_type_str)
}

pub fn comment_type_from_str<'de, D>(deserializer: D) -> Result<CommentType, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    match s.to_lowercase().as_str() {
        "bullish" => Ok(CommentType::Bullish),
        "bearish" => Ok(CommentType::Bearish),
        "custom" => Ok(CommentType::Custom),
        _ => Err(de::Error::custom(format!("Invalid comment type: {}", s))),
    }
}
