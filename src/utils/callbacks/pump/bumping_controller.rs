use std::{
    io,
    str::FromStr,
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};
//use strip_ansi_escapes::strip;

use crossterm::{
    style::SetBackgroundColor,
    terminal::{self, LeaveAlternateScreen},
    ExecutableCommand,
};
use serde_json::Value;
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer,
};
use tokio::{
    sync::{Mutex, RwLock},
    time::sleep,
};

use crate::{
    cli::{
        error::{display_error_page, spawn_error_input_listener},
        info::{display_info_page, InfoSegment},
        input::{display_input_page, InputType},
        live_pausable::LivePausableInfoPage,
        loading_indicator::display_loading_page,
        menu::{MenuHandler, Page},
        options::OptionCallback,
    },
    constants::general::OperationIntensity,
    loaders::{
        global_config_loader::{load_global_config, save_global_config, GlobalConfig},
    },
    utils::{
        blockhash_manager::RecentBlockhashManager,
        bonding_curve_provider::{run_curve_provider, BondingCurveProvider},
        misc::{
            is_pump_token, is_valid_decimal_string,
            is_valid_small_integer_string, parse_token_address, FundingStrategy, PercentileGroup,
        },
        pump_helpers::{derive_all_pump_keys},
    },
};

pub async fn invoke_bumping_callback(
    menu_handler: Arc<Mutex<MenuHandler>>,
    connection: Arc<RpcClient>,
    wss_url: Arc<String>,
    blockhash_manager: Arc<RecentBlockhashManager>,
    callback: OptionCallback,
    dev_wallet: Arc<Keypair>,
    funding_wallet: Arc<Keypair>,
    global_config: Arc<RwLock<Option<GlobalConfig>>>,
    current_tip: Arc<AtomicU64>,
    capsolver_api_key: Arc<String>,
) {
    let mut loading_handler_ref = menu_handler.lock().await;

    //let menu_handler_clone1 = Arc::clone(&menu_handler);
    //let menu_handler_clone2 = Arc::clone(&menu_handler);
    //let menu_handler_clone3 = Arc::clone(&menu_handler);

    match callback {
        //bump bot
        OptionCallback::ValidateAndConfirmOnDemandBumpInput => {
            ////
        }
        OptionCallback::OnDemandBump(data) => {
            ////
        }
        OptionCallback::ResumeBumpTask(task) => {
            ////
        }
        OptionCallback::PauseBumpTask(task) => {
            ////
        }
        OptionCallback::StopBumpTask(task) => {
            ////
        }
        _ => {}
    }
}
