use std::{fmt, str::FromStr, sync::Arc};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{de, Deserialize, Deserializer, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

// address constants
pub const GLOBAL_STATE: &str = "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf";
pub const FEE_RECEPIENT: &str = "62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV";
pub const EVENT_AUTH: &str = "Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1";
pub const MINT_AUTH: &str = "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM";
//this won't be needed anymore as pump now has its own dex.
pub const PUMP_MIGRATION_AUTHORITY: &str = "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg";
pub const METAPLEX_METADATA: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
pub const PUMP_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const BOT_PROGRAM_ID: &str = "33AnLbRvZctaCqpcAFG71g4UecwHX9snF6x3ADPjDCio";
pub const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
pub const SYSVAR_RENT_ID: &str = "SysvarRent111111111111111111111111111111111";
pub const AMM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const OPENBOOK: &str = "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX";
pub const AMM_V4_AUTHORITY: &str = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";
pub const AMM_V4_FEES: &str = "7YttLkHDoNj9wyDur5pM1ejNaAvT9X4eqaYcHQqtj2G5";
pub const AMM_V4_CONFIG: &str = "9DCxsMizn3H1hprZ7xWe6LDzeUeZBksYFpBWBtSf1PQX";
pub const GLOBAL_LUT_ADDRESS: &str = "CFooz7YWnXSsyNbK6TWswa3WwXahSPXuz3uotEikYbQp";

//pumfun dex keys
pub const PUMP_AMM_ADDRESS: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const PUMP_AMM_GLOBAL_CONFIG_ADDRESS: &str = "ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw";
pub const PUMP_AMM_EVENT_AUTH_ADDRESS: &str = "GS4CU59F31iL7aR2Q8zVS8DRrcRnXX1yjQ66TqNVQnaR";
//can be used for both pump.fun and pump amm after creator fees update
pub const PUMP_AMM_PROTOCOL_FEES_ADDRESS: &str = "9rPYyANsfQZw3DnDmKE3YCQF5E8oD89UXoHn9JFEhJUz";

//seed constants
pub const BUNDLER_GUARD_SEED: &[u8; 13] = b"bundler_guard";
pub const PUMP_BONDING_CURVE_SEED: &[u8; 13] = b"bonding-curve";
pub const PUMP_CREATOR_VAULT_SEED: &[u8; 13] = b"creator-vault";
pub const PUMP_CREATOR_VAULT_AUTHORITY_SEED: &[u8; 13] = b"creator_vault";
pub const METADATA_SEED: &[u8; 8] = b"metadata";

pub static TIP_ACCOUNTS: [&str; 8] = [
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
    "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
    "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
    "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
    "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
    "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
];

pub const BLOCK_ENGINE_URLS: [&str; 4] = [
    "amsterdam.mainnet.block-engine.jito.wtf",
    "frankfurt.mainnet.block-engine.jito.wtf",
    "ny.mainnet.block-engine.jito.wtf",
    "tokyo.mainnet.block-engine.jito.wtf",
];

pub const USER_AGENTS: [&str; 3] = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.1.1 Safari/605.1.15",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:89.0) Gecko/20100101 Firefox/89.0",
];

pub const PUMP_TOKEN_DECIMALS: u8 = 6;
pub const SYSTEM_ACCOUNT_RENT: u64 = 890880;
pub const TX_MAX_SIZE: u16 = 1232;
pub const ATA_RENT: f64 = 0.00203928;

// API Endpoints
pub const REGISTER_ENDPOINT: &str = "https://frontend-api-v3.pump.fun/users/register";
pub const LOGIN_ENDPOINT: &str = "https://frontend-api-v3.pump.fun/auth/login";
pub const USER_ENDPOINT: &str = "https://frontend-api-v3.pump.fun/users";
pub const LATEST_ENDPOINT: &str = "https://frontend-api.pump.fun/trades/latest";
pub const LATEST_COINS_ENDPOINT: &str = "https://frontend-api-v3.pump.fun/coins/latest";
pub const KOTH_ENDPOINT: &str =
    "https://frontend-api-v3.pump.fun/coins/king-of-the-hill?includeNsfw=true";
pub const GENERAL_COINS_ENDPOINT: &str = "https://frontend-api-v3.pump.fun/coins";
pub const FOLLOWS_ENDPOINT: &str = "https://frontend-api-v3.pump.fun/following";
pub const LIKES_ENDPOINT: &str = "https://frontend-api.pump.fun/likes";
pub const VANITY_ENDPOINT: &str = "https://frontend-api.pump.fun/vanity/key";
pub const UPLOAD_METADATA_ENDPOINT: &str = "https://pump.fun/api/ipfs";
pub const SOL_PRICE_ENDPOINT: &str = "https://frontend-api-v3.pump.fun/sol-price";
pub const PUMP_TRADES_ENDPOINT: &str = "https://frontend-api-v3.pump.fun/trades/all";

//some other constants
pub const DEFAULT_WEBHOOK_IMAGE_PLACEHORLDER: &str =
    "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcSen-P2DHu6J3cS8rJf8CR2bg2ac1ECY3JZpQ&s";

//essential types
#[derive(Debug)]
pub struct GeneralCoinsFetchResponse {
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub description: Option<String>,
    pub image_uri: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
    pub website: Option<String>,
    pub creator: String,
    pub usdt_market_cap: Option<f64>,
    // /pub raydium_pool: Option<String>,
    //add a ray pool field
}

#[derive(Clone, Debug)]
pub enum SwapType {
    Buy,
    Sell,
}
#[derive(Clone, Debug)]
pub struct PumpTokenTrade {
    pub operation_type: SwapType,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub user: String,
    pub timestamp: u64,
    pub identifier: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PumpKeys {
    pub pump_program_id: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub associated_bonding_curve: Pubkey,
    pub fees: Pubkey,
    pub event_auth: Pubkey,
    pub global_state: Pubkey,
    pub creator_vault: Pubkey,
}
#[derive(BorshDeserialize, Debug, Clone)]
pub struct BundleGuard {
    pub nonce: u64,
    pub owner: Pubkey,
    pub lut_guard: Option<Pubkey>,
}



#[derive(BorshDeserialize, Debug)]
pub struct GlobalState {
    pub initialized: bool,
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub fee_basis_points: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LookupTableFileFormat {
    pub lookup_table: String,
    pub mint: String,
}

#[derive(Clone, Debug, Copy)]
pub enum LutCallback {
    Deactivate,
    Close,
}

#[derive(Clone, Debug)]
pub enum SplitBundleFlavor {
    WithDelay(u8),
    Manual,
}

impl fmt::Display for SplitBundleFlavor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            SplitBundleFlavor::WithDelay(delay) => {
                if *delay == 0 {
                    "No Delay"
                } else {
                    &format!("With Delay ({}s)", delay)
                }
            }
            SplitBundleFlavor::Manual => "Manual",
        };
        write!(f, "{}", text)
    }
}

#[derive(Clone, Debug)]
pub struct SplitBundleConfig {
    pub flavor: SplitBundleFlavor,
    pub dev_bundle_tip: u64,
    pub buy_bundle_tip: u64,
}

#[derive(Clone)]
pub enum LaunchMode {
    Classic,
    BundleSnipe,
    MassSnipe,
    Stagger,
    DevOnly,
    CTO,
    Decoy,
}

impl fmt::Display for LaunchMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            LaunchMode::Classic => "Classic",
            LaunchMode::BundleSnipe => "Bundle Snipe",
            LaunchMode::MassSnipe => "Mass Snipe",
            LaunchMode::DevOnly => "Dev Only",
            LaunchMode::CTO => "CTO",
            LaunchMode::Stagger => "Stagger",
            LaunchMode::Decoy => "Decoy",
        };
        write!(f, "{}", text)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenAccountBalanceState {
    DoesNotExist,
    ExistsWithNoBalance,
    ExistsWithBalance(u64),
}
impl TokenAccountBalanceState {
    pub fn sum_balances(
        states: &[TokenAccountBalanceState],
        include_dev: bool,
    ) -> TokenAccountBalanceState {
        let total: u64 = states
            .iter()
            .skip(if include_dev { 1 } else { 0 })
            .filter_map(|state| match state {
                TokenAccountBalanceState::ExistsWithBalance(balance) => Some(*balance),
                _ => None,
            })
            .sum();

        if total > 0 {
            TokenAccountBalanceState::ExistsWithBalance(total)
        } else {
            TokenAccountBalanceState::ExistsWithNoBalance
        }
    }
}

#[derive(Clone)]
pub enum OperationIntensity {
    Low,
    Medium,
    High,
    Spam,
}

impl OperationIntensity {
    pub fn from_label(label: String) -> Self {
        match label.as_str() {
            "low" => Self::Low,
            "medium" => Self::Medium,
            "high" => Self::High,
            "spam" => Self::Spam,
            _ => Self::Low,
        }
    }
}

pub struct CircularKeypairBuffer {
    buffer: Vec<Arc<Keypair>>,
    current: usize,
}

impl CircularKeypairBuffer {
    pub fn new(keypairs: Vec<Arc<Keypair>>) -> Self {
        if keypairs.is_empty() {
            panic!("Cannot create CircularKeypairBuffer with empty keypairs");
        }

        CircularKeypairBuffer {
            buffer: keypairs,
            current: 0,
        }
    }

    pub fn get_and_advance(&mut self) -> Arc<Keypair> {
        let current_keypair = Arc::clone(&self.buffer[self.current]);
        self.current = (self.current + 1) % self.buffer.len();
        current_keypair
    }

    //pub fn len(&self) -> usize {
    //    self.buffer.len()
    //}
    //
    //pub fn is_empty(&self) -> bool {
    //    self.buffer.is_empty()
    //}
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub struct CreateEvent {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,          // PublicKey is a 32-byte array
    pub bonding_curve: Pubkey, // PublicKey as well
    pub user: Pubkey,          // PublicKey as well
    pub creator: Pubkey,
}
