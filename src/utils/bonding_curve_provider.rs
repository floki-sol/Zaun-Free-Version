use borsh::BorshDeserialize;
use log::info;
use num_bigint::BigUint;
use num_traits::cast::ToPrimitive;
use num_traits::{one, zero};
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use std::cmp;
use std::ops::{Add, Div, Mul, Sub};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::watch;
use tokio::time::{interval, Duration};

#[derive(BorshDeserialize, Debug)]
pub struct BondingCurve {
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
}
impl Default for BondingCurve {
    fn default() -> Self {
        Self {
            virtual_token_reserves: 1073000000000000,
            virtual_sol_reserves: 30000000000,
            real_token_reserves: 793100000000000,
            real_sol_reserves: 0, // Defaulting real_sol_reserves to 0
            token_total_supply: 1000000000000000,
            complete: false, // Explicitly set to false
        }
    }
}

pub struct BondingCurveProvider {
    rpc_client: Arc<RpcClient>,
    curve_state: RwLock<BondingCurve>,
    pub curve_pubkey: Pubkey,
    stop_signal: watch::Sender<bool>, // Sender to signal when to stop
    should_destory: AtomicBool,
    is_initialized: Arc<AtomicBool>,
}

impl BondingCurveProvider {
    pub fn new(rpc_client: Arc<RpcClient>, curve_address: Pubkey) -> Self {
        // Create the stop signal channel
        let (stop_sender, _stop_receiver) = watch::channel(false);

        Self {
            rpc_client,
            curve_state: RwLock::new(BondingCurve::default()),
            curve_pubkey: curve_address,
            stop_signal: stop_sender,
            should_destory: AtomicBool::new(false),
            is_initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn destroy(&self) {
        self.should_destory.store(true, Ordering::Relaxed);
    }

    pub fn initialize(&self) {
        //info!("bonding curve initialized");
        self.is_initialized.store(true, Ordering::Relaxed);
    }

    pub fn is_complete(&self) -> bool {
        self.curve_state.read().unwrap().complete
    }
    pub fn is_initialized(&self) -> bool {
        self.is_initialized.load(Ordering::Relaxed)
    }

    pub fn get_real_sol_reserves(&self) -> u64 {
        self.curve_state.read().unwrap().real_sol_reserves
    }

    pub fn get_virtual_token_reserves(&self) -> u64 {
        self.curve_state.read().unwrap().virtual_token_reserves
    }

    pub fn get_virtual_sol_reserves(&self) -> u64 {
        self.curve_state.read().unwrap().virtual_sol_reserves
    }

    pub fn get_real_token_reserves(&self) -> u64 {
        self.curve_state.read().unwrap().real_token_reserves
    }

    pub fn get_token_total_supply(&self) -> u64 {
        self.curve_state.read().unwrap().token_total_supply
    }

    pub fn get_rpc_client(&self) -> Arc<RpcClient> {
        self.rpc_client.clone()
    }

    fn update_curve(&self, new_curve: BondingCurve) {
        *self.curve_state.write().unwrap() = new_curve;
    }

    pub fn start(&self) {
        let _ = self.stop_signal.send(false);
    }

    pub fn stop(&self) {
        let _ = self.stop_signal.send(true);
    }

    // Function to create a stop signal receiver
    pub fn get_signal_receiver(&self) -> watch::Receiver<bool> {
        self.stop_signal.subscribe()
    }

    pub fn get_bonding_curve_progress(&self) -> u64 {
        let curve = self.curve_state.read().unwrap();

        // Get the current and initial real token reserves
        let current_real_token_reserves = curve.real_token_reserves;
        let initial_real_token_reserves = BondingCurve::default().real_token_reserves;

        // Calculate the progress (X) as the percentage of remaining progress
        let progress = if initial_real_token_reserves > 0 {
            // X = 100 - (current_real_token_reserves / initial_real_token_reserves) * 100
            100 - ((current_real_token_reserves as f64 / initial_real_token_reserves as f64)
                * 100.0) as u64
        } else {
            0 // If initial reserves are zero, return 0 progress
        };

        progress
    }
    pub fn get_koth_progress(&self) -> u64 {
        let en = BigUint::from(793_100_000_000_000u64); // Initial real token reserves
                                                        //info!("Initial real token reserves (en): {}", &en);

        let ed = BigUint::from(1_073_000_000u64); // Initial virtual token reserves
                                                  //info!("Initial virtual token reserves (ed): {}", &ed);

        let eu = BigUint::from(30u64); // Virtual SOL reserves
                                       //info!("Virtual SOL reserves (eu): {}", &eu);

        let em = BigUint::from(200u64); // EM value
                                        //info!("EM value (em): {}", &em);

        let ef = BigUint::from(1_000_000_000u64); // Hardcoded total token supply
                                                  //info!("Total token supply (ef): {}", &ef);

        let curve = self.curve_state.read().unwrap();
        let ei = BigUint::from(curve.real_token_reserves); // Current virtual token reserves
                                                           //info!("Current virtual token reserves (ei): {}", &ei);

        // Calculate intermediate values
        let part_1 = ed.clone().mul(&eu).mul(&ef).div(&em); // ed * eu * ef / em
                                                            //info!("Intermediate value part_1 (ed * eu * ef / em): {}", &part_1);

        let sqrt_part = part_1.sqrt(); // Math.sqrt(ed * eu * ef / em)
                                       //info!("Square root of part_1 (sqrt_part): {}", &sqrt_part);

        // Ensure no underflow for ed - sqrt_part
        let adjusted_ed = if ed > sqrt_part {
            ed.clone().sub(&sqrt_part)
        } else {
            one() // Default to 1 if underflow occurs
        };
        //info!("Adjusted ed (adjusted_ed): {}", &adjusted_ed);

        // Calculate the denominator
        let denominator = BigUint::from(1_000_000u64).mul(cmp::max(adjusted_ed.clone(), one()));
        //info!(
        //    "Denominator (1_000_000 * max(adjusted_ed, 1)): {}",
        //    &denominator
        //);

        // Calculate the numerator
        let numerator = if en > ei {
            en.clone().sub(&ei).mul(100u32)
        } else {
            zero()
        };
        //info!("Numerator ((en - ei) * 100): {}", &numerator);

        // Prevent division by zero
        let ratio = if denominator.eq(&zero()) {
            zero()
        } else {
            numerator.div(&denominator)
        };
        //info!("Ratio (numerator / denominator): {}", &ratio);

        // Ensure the result is clamped to 100
        let result = cmp::min(ratio.to_u64().unwrap_or(0), 100);
        //info!("Final clamped result (eh): {}", result);

        result
    }
}

//launch a thread with this
pub async fn run_curve_provider(bonding_curve_provider: Arc<BondingCurveProvider>) {
    let mut interval = interval(Duration::from_millis(300));
    let mut signal_receiver = bonding_curve_provider.get_signal_receiver();

    loop {
        if bonding_curve_provider
            .should_destory
            .load(Ordering::Relaxed)
            || bonding_curve_provider.is_complete()
        {
            //info!("curve provider task finished, or bonding curve complete");
            break;
        }

        tokio::select! {
            _ = interval.tick() => {

                if *signal_receiver.borrow() == false {
                    let rpc_client = bonding_curve_provider.get_rpc_client();
                    match rpc_client.get_account_with_commitment(
                        &bonding_curve_provider.curve_pubkey,
                        CommitmentConfig::processed()
                    ) {
                        Ok(account_info) => {
                            if let Some(info) = account_info.value {
                                let account_data = info.data;
                                let bonding_curve:BondingCurve = BondingCurve::deserialize(&mut &account_data[8..]).unwrap();
                                //info!("updated curve: {:#?}", &bonding_curve);
                                bonding_curve_provider.initialize();
                                bonding_curve_provider.update_curve(bonding_curve);
                            };
                            //blockhash_manager.update_blockhash(new_blockhash);
                        }
                        Err(e) => {
                            //info!("Failed to fetch bonding curve {}", e.to_string());
                        }
                    }
                } else {
                    //eprintln!("Failed to fetch recent blockhash: {:?}", e);
                }
            }
            _ = signal_receiver.changed() => {}
        }
    }
}
