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
use log::info;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use tokio::{
    sync::{Mutex, RwLock},
    time::sleep,
};

use crate::{
    cli::{
        error::{display_error_page, spawn_error_input_listener},
        info::InfoSegment,
        input::{display_input_page, InputType},
        live_pausable::LivePausableInfoPage,
        loading_indicator::display_loading_page,
        menu::{MenuHandler, Page},
        options::OptionCallback,
    }, constants::general::OperationIntensity, loaders::global_config_loader::GlobalConfig, utils::{
        blockhash_manager::RecentBlockhashManager,
        bonding_curve_provider::{run_curve_provider, BondingCurveProvider},
        comments_manager::{CommentsManager, CommentsStage},
    }
};

pub async fn invoke_comments_callback(
    menu_handler: Arc<Mutex<MenuHandler>>,
    connection: Arc<RpcClient>,
    wss_url: Arc<String>,
    _blockhash_manager: Arc<RecentBlockhashManager>,
    callback: OptionCallback,
    _dev_wallet: Arc<Keypair>,
    _funding_wallet: Arc<Keypair>,
    global_config: Arc<RwLock<Option<GlobalConfig>>>,
    _current_tip: Arc<AtomicU64>,
    capsolver_api_key: Arc<String>,
) {
    let mut loading_handler_ref = menu_handler.lock().await;

    let menu_handler_clone1 = Arc::clone(&menu_handler);
    let menu_handler_clone2 = Arc::clone(&menu_handler);
    let _menu_handler_clone3 = Arc::clone(&menu_handler);

    match callback {
        //comment bot
        OptionCallback::ValidateAndConfirmOnDemandCommentInput(comment_type) => {
            ////
        }
        OptionCallback::OnDemandComment((comment_type, data)) => {
            ////
        }
        OptionCallback::ResumeCommentsTask(task) => {
            ////
        }
        OptionCallback::PauseCommentsTask(task) => {
            ////
        }
        OptionCallback::StopCommentsTask(task) => {
            ////
        }

        _ => {}
    }
}
