use log::info;
use solana_client::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use std::sync::{Arc, RwLock};
use tokio::sync::watch;
use tokio::time::{interval, Duration};


pub struct RecentBlockhashManager {
    rpc_client: Arc<RpcClient>,
    recent_blockhash: RwLock<Hash>,
    stop_signal: watch::Sender<bool>, // Sender to signal when to stop
}

impl RecentBlockhashManager {
    pub fn new(rpc_client: Arc<RpcClient>, initial_blockhash: Hash) -> Self {
        // Create the stop signal channel
        let (stop_sender, _stop_receiver) = watch::channel(false);

        Self {
            rpc_client,
            recent_blockhash: RwLock::new(initial_blockhash),
            stop_signal: stop_sender,
        }
    }

    pub fn get_recent_blockhash(&self) -> Hash {
        *self.recent_blockhash.read().unwrap()
    }

    pub fn get_rpc_client(&self) -> Arc<RpcClient> {
        self.rpc_client.clone()
    }

    fn update_blockhash(&self, new_blockhash: Hash) {
        *self.recent_blockhash.write().unwrap() = new_blockhash;
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
}

pub async fn run_blockhash_updater(blockhash_manager: Arc<RecentBlockhashManager>) {
    let mut interval = interval(Duration::from_millis(300));
    let mut signal_receiver = blockhash_manager.get_signal_receiver();

    loop {
        tokio::select! {
            _ = interval.tick() => {

                if *signal_receiver.borrow() == false {
                    let rpc_client = blockhash_manager.get_rpc_client();
                    match rpc_client.get_latest_blockhash() {
                        Ok(new_blockhash) => {
                            blockhash_manager.update_blockhash(new_blockhash);
                            //info!("updated blockhash to: {:?}",new_blockhash);
                            //println!("Updated recent blockhash: {:?}", new_blockhash);
                        }
                        Err(e) => {
                            //info!("failed to fetch blockhash, current block hash: {:?}", blockhash_manager.get_recent_blockhash());
                            //eprintln!("Failed to fetch recent blockhash: {:?}", e);
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
