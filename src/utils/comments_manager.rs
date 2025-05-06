use borsh::BorshDeserialize;
use log::info;
use rand::seq::SliceRandom;
use reqwest::Proxy;
use serde_json::Value;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::borrow::Borrow;
use std::collections::{HashSet, VecDeque};
use std::fmt::{self, format};
use std::fs;
use std::ops::DerefMut;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{watch, Mutex, RwLock};
use tokio::time::{interval, sleep, Duration};

use crate::constants::general::{OperationIntensity, PumpKeys};
use crate::jito::bundles::{simulate_bundle, BundleSenderBalancer};


use super::blockhash_manager::RecentBlockhashManager;
use super::bonding_curve_provider::BondingCurveProvider;
use super::misc::adjust_file_path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommentType {
    Bullish,
    Bearish,
    Custom,
}

impl fmt::Display for CommentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            &CommentType::Bullish => "Bullish",
            &CommentType::Bearish => "Bearish",
            &CommentType::Custom => "Custom",
        };
        write!(f, "{}", text)
    }
}

#[derive(Debug, Clone)]
pub enum CommentsStage {
    Initialized,
    Commenting,
    Paused,
    Finished,
}

#[derive(Clone)]
pub struct CommentsManager {
    pub capsolver_api_key: Arc<String>,
    pub mint: Arc<Pubkey>,
    pub message_queue: Arc<Mutex<VecDeque<String>>>,
    pub comments_type: Arc<Mutex<CommentType>>,
    pub comments_stage: Arc<RwLock<CommentsStage>>,
    pub bullish_comments: Arc<Vec<String>>,
    pub bullish_load_error: Option<String>,

    pub bearish_comments: Arc<Vec<String>>,
    pub bearish_load_error: Option<String>,
    pub intensity: OperationIntensity,
    pub processed_comments: Arc<Mutex<HashSet<String>>>,
}

impl CommentsManager {
    ////
}
