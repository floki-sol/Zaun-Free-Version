use std::sync::atomic::{AtomicUsize, Ordering};

use reqwest::Client;
use serde_json::Value;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, transaction::VersionedTransaction};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::{
    constants::general::{PumpKeys, BLOCK_ENGINE_URLS},
};

#[derive(Debug, Clone)]
pub struct Bundle {
    pub transactions: Vec<String>,
}

impl Bundle {
    pub fn new(transactions: Vec<String>) -> Result<Self, &'static str> {
        if transactions.len() < 1 || transactions.len() > 5 {
            return Err("Bundle must contain between 1 and 5 Txs.");
        }
        Ok(Self { transactions })
    }
}

pub fn create_bundle(txs: Vec<VersionedTransaction>) -> Bundle {
    //let serialized_txs =
    //    vec![bs58::encode(bincode::serialize(&tx.unwrap()).unwrap()).into_string()];
    let serialized_txs = txs
        .iter()
        .map(|tx| bs58::encode(bincode::serialize(&tx).unwrap()).into_string())
        .collect();

    Bundle::new(serialized_txs).unwrap()
}

pub struct BundleSenderBalancer {
    client: Client,
    current: AtomicUsize,
}

impl BundleSenderBalancer {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            current: AtomicUsize::new(0),
        }
    }
    fn get_next_blockengine(&self) -> &'static str {
        let index = self.current.fetch_add(1, Ordering::SeqCst);
        &BLOCK_ENGINE_URLS[index % BLOCK_ENGINE_URLS.len()]
    }

    pub async fn send_bundle(
        &self,
        bundle: Bundle,
        //sent_transactions: &Arc<Mutex<usize>>,
    ) -> Result<(), String> {
        let url = self.get_next_blockengine();
        let endpoint = format!("https://{}/api/v1/bundles", url);
        let request_data = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendBundle",
            "params": [bundle.transactions]
        });

        let client = self.client.clone();
        //let request_data = request_data.clone();
        tokio::spawn(async move {
            //println!("Sending bundle to block engine: {}", url);
            match 
                client
                .post(&endpoint)
                .json(&request_data)
                .send()
                .await
            {
                Ok(response) => match response.json::<Value>().await {
                    Ok(json) => {
                        if let Some(error) = json.get("error") {
                            //info!("Error sending to {}: {:?}", url, error);
                        } else if let Some(result) = json.get("result") {
                            let bundle_id = result.as_str().unwrap_or("Unknown");
                            //info!("Bundle ID: {}", bundle_id);
                        } else {
                            //info!("Unexpected response format from {}", url);
                        }
                    }
                    Err(e) => {
                        //info!("Failed to parse JSON response from {}: {:?}", url, e);
                    }
                },
                Err(e) => {
                    //info!("Failed to send request to {}: {:?}", url, e);
                }
            }
        });

        Ok(())
    }
}

// simulate bundle method 
pub async fn simulate_bundle(bundle: Bundle) {
    ////////////
}
