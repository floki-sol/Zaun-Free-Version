use std::{
    f64::NAN,
    str::FromStr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::{constants::general::TIP_ACCOUNTS, loaders::global_config_loader::GlobalConfig};
use futures::StreamExt;
use log::info;
use rand::seq::SliceRandom;
use serde_json::Value;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use tokio::{
    sync::{Mutex, RwLock},
    time::timeout,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct TipPercentileInfo {
    pub _25th: f64,
    pub _50th: f64,
    pub _75th: f64,
    pub _95th: f64,
    pub _99th: f64,
}

impl TipPercentileInfo {

    // Method 2: Using an array
    pub fn get_percentile_array(&self) -> [(u8, f64); 5] {
        [
            (25, self._25th),
            (50, self._50th),
            (75, self._75th),
            (95, self._95th),
            (99, self._99th),
        ]
    }
}

pub fn get_random_tip_account() -> Pubkey {
    let mut rng = rand::thread_rng();
    Pubkey::from_str(TIP_ACCOUNTS.choose(&mut rng).unwrap()).unwrap()
}

pub async fn run_jito_fee_websocket(
    fee: Arc<AtomicU64>,
    global_config: Arc<RwLock<Option<GlobalConfig>>>,
) {
    let url = "wss://bundles.jito.wtf/api/v1/bundles/tip_stream";

    loop {
        //info!("Connecting to Jito fee WebSocket");
        match timeout(Duration::from_secs(3), connect_async(url)).await {
            Ok(Ok((ws_stream, _))) => {
                //info!("Connected to Jito fee WebSocket");
                let (_, mut read) = ws_stream.split();

                while let Some(message) = read.next().await {
                    match message {
                        Ok(Message::Text(text)) => {
                            if let Ok(value) = serde_json::from_str::<Value>(&text) {
                                //info!("{:#?}", value);

                                let floored_fee = extract_jito_fee(&global_config, &value).await;

                                //println!("tip stream res: {:?}", value);
                                //println!("Updated tip: {:?}", floored_fee);

                                fee.store(floored_fee, Ordering::Relaxed);
                                //println!("Updated Jito fee: {}", (floored_fee as f64 / LAMPORTS_PER_SOL as f64));
                                //}
                            }
                        }
                        Ok(Message::Close(_)) => {
                            //println!("WebSocket connection closed");
                            break;
                        }
                        Err(e) => {
                            //println!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Ok(Err(e)) => {
                //println!("Failed to connect to WebSocket: {}", e);
            }
            Err(_) => {
                //println!("WebSocket connection timed out");
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn extract_jito_fee(
    global_config: &Arc<RwLock<Option<GlobalConfig>>>,
    json_data: &Value,
) -> u64 {
    let default_fee: u64 = 100_000;

    if let Some(config) = global_config.read().await.as_ref() {
        let percentile = config.jito_tip_stream_percentile;
        let fee_field = match percentile {
            25 => "landed_tips_25th_percentile",
            50 => "landed_tips_50th_percentile",
            75 => "landed_tips_75th_percentile",
            95 => "landed_tips_95th_percentile",
            99 => "landed_tips_99th_percentile",
            _ => return default_fee,
        };

        //info!("{}", fee_field);

        let fee_value = json_data
            .get(0)
            .and_then(|v| v.get(fee_field))
            .and_then(|v| v.as_f64())
            .map(|fee| (fee.max(0.0) * 1e9) as u64)
            .unwrap_or(default_fee);

        std::cmp::min(fee_value, (config.jito_max_tip * 1e9) as u64)
    } else {
        default_fee
    }
}

pub async fn fetch_recent_tip_percentiles() -> Result<TipPercentileInfo, String> {
    // Construct the URL using the token pubkey
    let url = "https://bundles.jito.wtf/api/v1/bundles/tip_floor";

    // Make the HTTP request and handle errors
    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Request error: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Failed to fetch data: HTTP {}", response.status()));
    }

    let response_json = response
        .json::<Value>()
        .await
        .map_err(|e| format!("Deserialization error: {}", e))?;

    let reponse_json_array = response_json.get(0).ok_or((String::from("Failed to deserialize percentile info")))?;

    let _25th_percentile = reponse_json_array
        .get("landed_tips_25th_percentile")
        .unwrap()
        .as_f64()
        .unwrap_or(0.0);

    let _50th_percentile = reponse_json_array
        .get("landed_tips_50th_percentile")
        .unwrap()
        .as_f64()
        .unwrap_or(0.0);

    let _75th_percentile = reponse_json_array
        .get("landed_tips_75th_percentile")
        .unwrap()
        .as_f64()
        .unwrap_or(0.0);

    let _95th_percentile = reponse_json_array
        .get("landed_tips_95th_percentile")
        .unwrap()
        .as_f64()
        .unwrap_or(0.0);

    let _99th_percentile = reponse_json_array
        .get("landed_tips_99th_percentile")
        .unwrap()
        .as_f64()
        .unwrap_or(0.0);

    Ok(TipPercentileInfo {
        _25th: _25th_percentile,
        _50th: _50th_percentile,
        _75th: _75th_percentile,
        _95th: _95th_percentile,
        _99th: _99th_percentile,
    })
}
