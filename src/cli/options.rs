use core::fmt;
use std::{
    collections::VecDeque,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{atomic::AtomicBool, Arc, Mutex as SyncMutex},
};

use crate::{
    cli::menu::MenuPage,
    constants::general::{
        LaunchMode, LutCallback, OperationIntensity, PumpKeys, SplitBundleConfig,
        SplitBundleFlavor, TokenAccountBalanceState,
    },
    utils::{
        bonding_curve_provider::BondingCurveProvider,
        bump_manager::BumpManager,
        comments_manager::{CommentType, CommentsManager},
        misc::{FundingStrategy, PercentileGroup, WalletType, WalletsFundingType},
    },
};
use solana_sdk::{
    address_lookup_table::AddressLookupTableAccount, pubkey::Pubkey, signature::Keypair,
};
use tokio::{process::Child, sync::Mutex};

use super::{
    menu::{MenuHandler, Page},
    pages::pump::main_menu::bundle::launch_setup::dev_buy,
};

#[derive(Clone)]
pub enum CallbackCategory {
    Settings,
    BumpBot,
    CommentBot,
    Launching,
    Tracking,
    WalletManagement,
    Misc,
}

#[derive(Clone)]
pub enum OptionCallback {
    BalanceCheckerCallback((WalletType, String)),
    GrindVanityCallBack,
    FetchVanityCallBack,
    StopGrindTask(Arc<Mutex<Child>>),
    BundlerWalletGenerationMenuCallBack,
    GenerateBundlerWalletsCallback(bool),
    ConfirmGenerationInput(String),
    RetrieveWalletsFromBackupsCallback,
    ConfirmRetrieveBackup(PathBuf),

    //bump bot
    ValidateAndConfirmOnDemandBumpInput,
    OnDemandBump(String),
    ResumeBumpTask(Arc<Mutex<BumpManager>>),
    PauseBumpTask(Arc<Mutex<BumpManager>>),
    StopBumpTask(Arc<Mutex<BumpManager>>),

    //comments
    ValidateAndConfirmOnDemandCommentInput(CommentType),
    OnDemandComment((CommentType, String)),
    ResumeCommentsTask(Arc<Mutex<CommentsManager>>),
    PauseCommentsTask(Arc<Mutex<CommentsManager>>),
    StopCommentsTask(Arc<Mutex<CommentsManager>>),

    // settings
    ChangePercentile(PercentileGroup),
    ChangeMaxTip(String),
    ChangeSplitBundleTipPercentages((f32, f32)),
    ViewJitoTip,
    ChangeFundingStrategy(FundingStrategy),
    ChangePumpCommentIntensity(OperationIntensity),
    ChangeBundleTimeout(String),
    ChangeUseVideo(bool),
    ChangeBumpDelay(String),
    ChangeBumpAmount(String),
    ChangeWalletsToBump(String),
    ChangeBumpFunder(String),
    RemoveOptionalProfile,
    ConfigureBumpOptionalProfile(String),
    ChangeFollowerProfile(String),
    ChangePumpFollowIntensity(OperationIntensity),
    ToggleDebugMode(bool),
    ChangeTrackerTransferRecepient(String),
    ChangeTrackerTradeFeedMinSolValue(String),
    ChangeTrackerCommentType(CommentType),
    ChangeTrackerTargetMarketCap(String),
    ChangeTrackerDelaySell(String),
    ChangeRpcHealthCheckPreference(bool),
    ViewSubscriptionDetails,

    //wallet funding
    FundSingleWallet((WalletType, String)),
    FundBundleWallets(WalletsFundingType),
    //wallet cleanup
    CleanUpSol(WalletType, WalletType),
    CleanUpTokens(String, WalletType, WalletType),
    ReceiverInputCallback(String),
    //burning tokens
    OnDemandBurn((WalletType, String)),

    //launching
    StartDustCoinsTask(Arc<AtomicBool>),
    StopDustCoinsTask(Arc<AtomicBool>),
    StartTradingActivityTask,
    StopTradingActivityTask(Arc<AtomicBool>),
    SimulateLaunch,
    GenerateCa,
    ValidateBase58Ca(String),
    ValidateCaFile,
    VerifyAndUploadMetadata(Arc<Keypair>),
    CloneTokenMetadata((Arc<Keypair>, String)),
    VerifyMetadataLinkInput((Arc<Keypair>, String)),
    SetupLookUpTable((Arc<Keypair>, String, (String, String))),
    CreateLookUpTable((Arc<Keypair>, String, (String, String))),
    ManageLut(LutCallback),
    ValidateCTOInput((Arc<Keypair>, String, (String, String), LaunchMode, String)),
    ValidateDevBuy((Arc<Keypair>, String, (String, String), LaunchMode, String)),
    ValidateSplitBundleDelay(
        (
            Arc<Keypair>,
            String,
            (String, String),
            LaunchMode,
            u64,
            Vec<u64>,
            String,
        ),
    ),
    LaunchToken(
        (
            Arc<Keypair>,
            String,
            (String, String),
            LaunchMode,
            u64,
            Vec<u64>,
            Option<Pubkey>,
            Option<SplitBundleConfig>,
        ),
    ),
    SignalManualBuy((Arc<AtomicBool>, Arc<BondingCurveProvider>)),

    //tracking
    TrackNormal,
    TrackBonded,
    SingleWalletTradePump(
        (
            (),
            Pubkey,
            Arc<Mutex<Vec<()>>>,
            usize,
            u64,
            Arc<Mutex<VecDeque<()>>>,
            Arc<PumpKeys>,
            Arc<Mutex<Vec<()>>>,
            Arc<Mutex<Vec<()>>>,
            Option<Arc<AddressLookupTableAccount>>,
            Arc<AddressLookupTableAccount>,
        ),
    ),
    MultiWalletTradePump(
        (
            (),
            Pubkey,
            Arc<Mutex<Vec<()>>>,
            Vec<(usize, u64)>,
            Arc<Mutex<VecDeque<()>>>,
            Arc<PumpKeys>,
            Arc<Mutex<Vec<()>>>,
            Arc<Mutex<Vec<()>>>,
            Option<Arc<AddressLookupTableAccount>>,
            Arc<AddressLookupTableAccount>,
        ),
    ),
    StopNormalTracking(
        (
            Arc<BondingCurveProvider>,
            Option<Arc<Mutex<BumpManager>>>,
            Arc<Mutex<CommentsManager>>,
        ),
    ),
    StopBondedTracking,
    QuickSellAllNormal,
    QuickSellAllBondedInsta,
    QuickSellAllBondedAwaited,
    StopQuickSellAllBondedTask(Arc<AtomicBool>),
    BurnDevAll,

    //simulations
    SimulateHolderDistributions(String),

    //menuing callbacks
    DoNothing,
    ReturnToMenu,

    //extras
    StartNewCoinsMonitor(String, String),
    StartKothMonitor(String, String),
    StartMigrationMonitor(String, String),
    StopMonitorTask(Arc<AtomicBool>),
}

impl OptionCallback {
    // Method to update the inner value if the variant is `ConfirmGenerationInput`
    pub fn update_input_callback(&mut self, new_value: String) {
        match self {
            OptionCallback::ConfirmGenerationInput(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::BalanceCheckerCallback((_wallet_type, ref mut input)) => {
                *input = new_value;
            }
            OptionCallback::ChangeMaxTip(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::ChangeBundleTimeout(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::FundSingleWallet((_wallet_type, ref mut input)) => {
                *input = new_value;
            }
            OptionCallback::ValidateBase58Ca(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::FundBundleWallets(ref mut inner_value) => {
                inner_value.update_inner_value(new_value);
            }
            OptionCallback::ReceiverInputCallback(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::CloneTokenMetadata((_pair, ref mut inner_value)) => {
                *inner_value = new_value;
            }
            OptionCallback::VerifyMetadataLinkInput((_pair, ref mut inner_value)) => {
                *inner_value = new_value;
            }
            OptionCallback::OnDemandBump(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::OnDemandComment((ref _comment_type, ref mut inner_value)) => {
                *inner_value = new_value;
            }
            OptionCallback::ValidateCTOInput((
                _token,
                _metadata,
                (_name, _symbol),
                _mode,
                ref mut inner_value,
            )) => {
                *inner_value = new_value;
            }
            OptionCallback::ValidateDevBuy((
                _token,
                _metadata,
                (_name, _symbol),
                _mode,
                ref mut inner_value,
            )) => {
                *inner_value = new_value;
            }

            OptionCallback::ValidateSplitBundleDelay((
                _token,
                _metadata,
                (_name_, _symbol),
                _mode,
                _dev_buy,
                _amounts,
                ref mut inner_value,
            )) => {
                *inner_value = new_value;
            }
            OptionCallback::ChangeBumpAmount(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::ChangeBumpDelay(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::ChangeWalletsToBump(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::ChangeBumpFunder(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::ConfigureBumpOptionalProfile(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::SimulateHolderDistributions(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::OnDemandBurn((_wallet_type, ref mut input)) => {
                *input = new_value;
            }
            OptionCallback::CleanUpTokens(input, _sender, ref mut _receiver) => {
                *input = new_value;
            }
            OptionCallback::ChangeFollowerProfile(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::ChangeTrackerTransferRecepient(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::ChangeTrackerTradeFeedMinSolValue(ref mut inner_value) => {
                *inner_value = new_value;
            }
            OptionCallback::StartNewCoinsMonitor(ref mut inner_value, ref _channel) => {
                *inner_value = new_value;
            }
            OptionCallback::StartKothMonitor(ref mut inner_value, ref _channel) => {
                *inner_value = new_value;
            }
            OptionCallback::StartMigrationMonitor(ref mut inner_value, ref _channel) => {
                *inner_value = new_value;
            }

            OptionCallback::ChangeTrackerTargetMarketCap(ref mut inner_value) => {
                *inner_value = new_value;
            }

            OptionCallback::ChangeTrackerDelaySell(ref mut inner_value) => {
                *inner_value = new_value;
            }

            _ => (),
        }
    }

    pub fn get_callback_category(&self) -> CallbackCategory {
        match self {
            //bump bot category
            OptionCallback::ValidateAndConfirmOnDemandBumpInput => CallbackCategory::BumpBot,
            OptionCallback::OnDemandBump(_) => CallbackCategory::BumpBot,
            OptionCallback::ResumeBumpTask(_) => CallbackCategory::BumpBot,
            OptionCallback::PauseBumpTask(_) => CallbackCategory::BumpBot,
            OptionCallback::StopBumpTask(_) => CallbackCategory::BumpBot,

            //comments category
            OptionCallback::ValidateAndConfirmOnDemandCommentInput(_) => {
                CallbackCategory::CommentBot
            }
            OptionCallback::OnDemandComment(_) => CallbackCategory::CommentBot,
            OptionCallback::ResumeCommentsTask(_) => CallbackCategory::CommentBot,
            OptionCallback::PauseCommentsTask(_) => CallbackCategory::CommentBot,
            OptionCallback::StopCommentsTask(_) => CallbackCategory::CommentBot,

            //settings category
            OptionCallback::ChangePercentile(_) => CallbackCategory::Settings,
            OptionCallback::ChangeMaxTip(_) => CallbackCategory::Settings,
            OptionCallback::ChangeFundingStrategy(_) => CallbackCategory::Settings,
            OptionCallback::ChangeSplitBundleTipPercentages(_) => CallbackCategory::Settings,
            OptionCallback::ChangePumpCommentIntensity(_) => {
                CallbackCategory::Settings
            }
            OptionCallback::ChangeBundleTimeout(_) => CallbackCategory::Settings,
            OptionCallback::ChangeUseVideo(_) => CallbackCategory::Settings,
            OptionCallback::ChangeBumpDelay(_) => CallbackCategory::Settings,
            OptionCallback::ChangeBumpAmount(_) => CallbackCategory::Settings,
            OptionCallback::ChangeWalletsToBump(_) => CallbackCategory::Settings,
            OptionCallback::ChangeBumpFunder(_) => CallbackCategory::Settings,
            OptionCallback::RemoveOptionalProfile => CallbackCategory::Settings,
            OptionCallback::ConfigureBumpOptionalProfile(_) => CallbackCategory::Settings,
            OptionCallback::ChangeFollowerProfile(_) => CallbackCategory::Settings,
            OptionCallback::ChangePumpFollowIntensity(_) => CallbackCategory::Settings,
            OptionCallback::ToggleDebugMode(_) => CallbackCategory::Settings,
            OptionCallback::ChangeTrackerTransferRecepient(_) => CallbackCategory::Settings,
            OptionCallback::ChangeTrackerCommentType(_) => CallbackCategory::Settings,
            OptionCallback::ChangeTrackerTradeFeedMinSolValue(_) => CallbackCategory::Settings,
            OptionCallback::ChangeTrackerTargetMarketCap(_) => CallbackCategory::Settings,
            OptionCallback::ChangeTrackerDelaySell(_) => CallbackCategory::Settings,
            OptionCallback::ViewJitoTip => CallbackCategory::Settings,
            OptionCallback::ChangeRpcHealthCheckPreference(_) => CallbackCategory::Settings,
            OptionCallback::ViewSubscriptionDetails => CallbackCategory::Settings,

            //wallet management category
            OptionCallback::BalanceCheckerCallback(_) => CallbackCategory::WalletManagement,
            OptionCallback::RetrieveWalletsFromBackupsCallback => {
                CallbackCategory::WalletManagement
            }
            OptionCallback::ConfirmRetrieveBackup(_) => CallbackCategory::WalletManagement,
            OptionCallback::BundlerWalletGenerationMenuCallBack => {
                CallbackCategory::WalletManagement
            }
            OptionCallback::GenerateBundlerWalletsCallback(_) => CallbackCategory::WalletManagement,
            OptionCallback::ConfirmGenerationInput(_) => CallbackCategory::WalletManagement,
            OptionCallback::FundSingleWallet(_) => CallbackCategory::WalletManagement,
            OptionCallback::FundBundleWallets(_) => CallbackCategory::WalletManagement,
            OptionCallback::ReceiverInputCallback(_) => CallbackCategory::WalletManagement,
            OptionCallback::CleanUpSol(_, _) => CallbackCategory::WalletManagement,
            OptionCallback::CleanUpTokens(_, _, _) => CallbackCategory::WalletManagement,
            OptionCallback::OnDemandBurn(_) => CallbackCategory::WalletManagement,

            //tracking
            OptionCallback::TrackNormal => CallbackCategory::Tracking,
            OptionCallback::TrackBonded => CallbackCategory::Tracking,
            OptionCallback::SingleWalletTradePump(_) => CallbackCategory::Tracking,
            OptionCallback::MultiWalletTradePump(_) => CallbackCategory::Tracking,
            OptionCallback::StopNormalTracking(_) => CallbackCategory::Tracking,
            OptionCallback::StopBondedTracking => CallbackCategory::Tracking,
            OptionCallback::QuickSellAllNormal => CallbackCategory::Tracking,
            OptionCallback::QuickSellAllBondedInsta => CallbackCategory::Tracking,
            OptionCallback::QuickSellAllBondedAwaited => CallbackCategory::Tracking,
            OptionCallback::StopQuickSellAllBondedTask(_) => CallbackCategory::Tracking,
            OptionCallback::BurnDevAll => CallbackCategory::Tracking,

            //launching
            OptionCallback::SimulateLaunch => CallbackCategory::Launching,
            OptionCallback::StartDustCoinsTask(_) => CallbackCategory::Launching,
            OptionCallback::StopDustCoinsTask(_) => CallbackCategory::Launching,
            OptionCallback::StartTradingActivityTask => CallbackCategory::Launching,
            OptionCallback::StopTradingActivityTask(_) => CallbackCategory::Launching,
            OptionCallback::GenerateCa => CallbackCategory::Launching,
            OptionCallback::ValidateBase58Ca(_) => CallbackCategory::Launching,
            OptionCallback::ValidateCaFile => CallbackCategory::Launching,
            OptionCallback::VerifyAndUploadMetadata(_) => CallbackCategory::Launching,
            OptionCallback::CloneTokenMetadata(_) => CallbackCategory::Launching,
            OptionCallback::VerifyMetadataLinkInput(_) => CallbackCategory::Launching,
            OptionCallback::SetupLookUpTable(_) => CallbackCategory::Launching,
            OptionCallback::CreateLookUpTable(_) => CallbackCategory::Launching,
            OptionCallback::ValidateCTOInput(_) => CallbackCategory::Launching,
            OptionCallback::ValidateDevBuy(_) => CallbackCategory::Launching,
            OptionCallback::ValidateSplitBundleDelay(_) => CallbackCategory::Launching,
            OptionCallback::LaunchToken(_) => CallbackCategory::Launching,
            OptionCallback::SignalManualBuy(_) => CallbackCategory::Launching,

            _ => CallbackCategory::Misc,
        }
    }
}

#[derive(Clone)]
pub struct PageOption {
    pub option_title: String,
    pub associated_page: Option<Page>, // Optional next page for navigation
    pub callback: Option<OptionCallback>, // Optional callback
}

impl PageOption {
    pub fn new(
        option_title: String,
        associated_page: Option<Page>,
        callback: Option<OptionCallback>,
    ) -> Self {
        PageOption {
            option_title,
            associated_page,
            callback,
        }
    }
}
