use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::utils::misc::adjust_file_path;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct JitoSplitBundlePercentages {
    pub dev_bundle_tip_percentage: f32,
    pub buy_bundle_tip_percentage: f32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct GlobalConfig {
    pub jito_tip_stream_percentile: u32,
    pub jito_max_tip: f64,
    pub jito_tip_split_bundle_percentages: JitoSplitBundlePercentages,
    pub funding_strategy: String,
    pub bundle_timeout: u8,
    pub use_video: bool,
    pub pump_comments_intensity: String,
    pub debug: bool,
    pub skip_rpc_health_check: bool,
}

impl GlobalConfig {}

pub fn load_global_config() -> Result<GlobalConfig, String> {
    let file_path = &adjust_file_path("configurations/global-config.json");

    // Ensure the file path exists and is a valid JSON file
    if !Path::new(file_path).exists() {
        return Err(format!("Configuration file '{}' does not exist", file_path));
    }

    // Read the JSON file
    let data = fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Deserialize the JSON content into the GlobalConfig struct
    let config: GlobalConfig = serde_json::from_str(&data)
        .map_err(|e| format!("Invalid JSON format in global-config.json: {}", e))?;

    // Validate `jito_tip_stream_percentile`
    match config.jito_tip_stream_percentile {
        25 | 50 | 75 | 95 | 99 => (),
        _ => {
            return Err(
                "Invalid value for JITO_TIP_STREAM_PERCENTILE. Must be one of: 25, 50, 75, 95, 99."
                    .to_string(),
            )
        }
    }

    // Validate `jito_max_tip`
    if config.jito_max_tip < 0.00001 {
        return Err("JITO_MAX_TIP must be at least 0.00001.".to_string());
    }

    let dev = config
        .jito_tip_split_bundle_percentages
        .dev_bundle_tip_percentage;
    let buy = config
        .jito_tip_split_bundle_percentages
        .buy_bundle_tip_percentage;

    // Check sum equals 1.0
    if dev + buy != 1.0 {
        return Err(format!(
            "dev bundle percentage ({}) and buy bundle percentage ({}) must sum to exactly 1.0 (100%)",
            dev, buy
        ));
    }

    if !has_one_decimal_place(dev) {
        return Err(format!(
            "dev bundle percentage ({}) must have exactly one decimal place",
            dev
        ));
    }
    if !has_one_decimal_place(buy) {
        return Err(format!(
            "buy bundle percentage ({}) must have exactly one decimal place",
            buy
        ));
    }

    match config.funding_strategy.as_str() {
        "pre-fund" => (),
        "in-contract" => (),
        _ => {
            return Err(
                "Invalid value for FUNDING_STRATEGY. Must be one of: 'pre-fund' | 'in-contract'."
                    .to_string(),
            )
        }
    }

    match config.pump_comments_intensity.as_str() {
        "low" => (),
        "medium" => (),
        "high" => (),
        "spam" => (),
        _ => {
            return Err(
                "Invalid value for PUMP_COMMENTS_INTENSITY. Must be one of: 'low' | 'medium' | 'high' |  | 'spam'."
                    .to_string(),
            )
        }
    }

    if config.bundle_timeout <= 10 || config.bundle_timeout >= 180 {
        return Err(
            "Invalid value for BUNDLE_TIMEOUT. Must be between 10 and 180 seconds".to_string(),
        );
    }

    // If all checks pass, return Ok
    Ok(config)
}

fn has_one_decimal_place(value: f32) -> bool {
    let scaled = (value * 10.0).to_string();
    scaled.parse::<u8>().is_ok()
}

pub fn save_global_config(config: &GlobalConfig) -> Result<(), String> {
    let file_path = &adjust_file_path("configurations/global-config.json");

    // Serialize the configuration to a JSON string
    let json_data = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    // Write the JSON data to the file
    fs::write(file_path, json_data).map_err(|e| format!("Failed to write to file: {}", e))?;

    Ok(())
}
