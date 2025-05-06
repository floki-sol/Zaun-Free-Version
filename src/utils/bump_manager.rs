use borsh::BorshDeserialize;
use log::info;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{watch, Mutex, RwLock};
use tokio::time::{interval, sleep, Duration};

use crate::constants::general::PumpKeys;
use crate::jito::bundles::{simulate_bundle, BundleSenderBalancer};

use super::blockhash_manager::RecentBlockhashManager;
use super::bonding_curve_provider::BondingCurveProvider;

#[derive(Debug, Clone)]
pub enum BumpingStatus {
    InsufficientBalance,
    Bumping,
    TokenBonded,
}

#[derive(Debug, Clone)]
pub enum BumpStage {
    Initialized(Result<(), String>),
    Bumping(BumpingStatus),
    Paused(Result<(), String>),
    Finished,
}

pub struct BumpManager {
    blockhash_manager: Arc<RecentBlockhashManager>,
    pub curve_provider: Arc<BondingCurveProvider>,
    connection: Arc<RpcClient>,
    pump_keys: Arc<PumpKeys>,
    bump_funder: Arc<Keypair>,
    bumper_wallets: Vec<Arc<Keypair>>,
    current_wallet_idx: Arc<AtomicUsize>,
    config: (),
    bump_stage: Arc<RwLock<BumpStage>>,
    message_queue: Arc<Mutex<VecDeque<String>>>,
    tip: u64,
    //stop_signal: watch::Sender<bool>, // Sender to signal when to stop
}

impl BumpManager {
    ////
}
