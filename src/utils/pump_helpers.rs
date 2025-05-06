use futures::TryFutureExt;
use image::{DynamicImage, ImageFormat};
use log::info;
use rand::seq::SliceRandom;
use reqwest::{
    header,
    multipart::{Form, Part},
    Client, Proxy,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::native_mint;
use std::{
    fs,
    io::Cursor,
    str::FromStr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{fs::File, io::BufReader};

use crate::{
    constants::general::{
        GeneralCoinsFetchResponse, PumpKeys, PumpTokenTrade, SwapType, EVENT_AUTH, FEE_RECEPIENT,
        FOLLOWS_ENDPOINT, GENERAL_COINS_ENDPOINT, GLOBAL_STATE, KOTH_ENDPOINT,
        LATEST_COINS_ENDPOINT, LOGIN_ENDPOINT, PUMP_AMM_ADDRESS, PUMP_AMM_EVENT_AUTH_ADDRESS,
        PUMP_AMM_GLOBAL_CONFIG_ADDRESS, PUMP_AMM_PROTOCOL_FEES_ADDRESS, PUMP_PROGRAM_ID,
        PUMP_TRADES_ENDPOINT, REGISTER_ENDPOINT, SOL_PRICE_ENDPOINT, UPLOAD_METADATA_ENDPOINT,
        USER_AGENTS, USER_ENDPOINT,
    },
    loaders::metadata_loader::Metadata,
};

use super::{
    misc::{adjust_file_path, fix_ipfs_url},
    pdas::get_bonding_curve,
};

pub fn derive_all_pump_keys(buyer: &Pubkey, mint: Pubkey) -> PumpKeys {
    let program_id = Pubkey::from_str(PUMP_PROGRAM_ID).unwrap();
    let bonding_curve = get_bonding_curve(&mint, &program_id);

    let associated_bonding_curve = get_associated_token_address(&bonding_curve, &mint);

    PumpKeys {
        pump_program_id: program_id,
        mint,
        bonding_curve,
        associated_bonding_curve,
        fees: Pubkey::from_str(FEE_RECEPIENT).unwrap(),
        event_auth: Pubkey::from_str(EVENT_AUTH).unwrap(),
        global_state: Pubkey::from_str(GLOBAL_STATE).unwrap(),
    }
}

#[derive(Debug)]
pub struct PumpDexKeys {
    pub program_id: Pubkey,
    pub pool_authority: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub pool_id: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_base_ata: Pubkey,
    pub pool_quote_ata: Pubkey,
    pub pump_amm_global_config: Pubkey,
    pub pump_amm_event_auth: Pubkey,
    pub pump_amm_protocol_fees: Pubkey,
    pub pump_amm_protocol_fees_quote_ata: Pubkey,
}

pub fn derive_all_pump_dex_keys(mint: &Pubkey) -> PumpDexKeys {
    let pump_program_id = Pubkey::from_str(PUMP_PROGRAM_ID).unwrap();
    let pump_amm_program_id = Pubkey::from_str(PUMP_AMM_ADDRESS).unwrap();
    let bonding_curve = get_bonding_curve(&mint, &pump_program_id);

    //assume cannonical index
    let index_seed = 0u16.to_le_bytes(); // [0, 0]

    //this is the pool authority in the case of migration through pump automated systems.
    let (pool_authority, _) = Pubkey::find_program_address(
        &[b"pool-authority", mint.to_bytes().as_ref()],
        &pump_program_id,
    );

    let (pool_id, _) = Pubkey::find_program_address(
        &[
            b"pool",
            &index_seed,
            pool_authority.to_bytes().as_ref(),
            mint.to_bytes().as_ref(),
            native_mint::id().to_bytes().as_ref(),
        ],
        &pump_amm_program_id,
    );

    let (lp_mint, _) = Pubkey::find_program_address(
        &[b"pool_lp_mint", pool_id.to_bytes().as_ref()],
        &pump_amm_program_id,
    );

    let (pool_base_ata, _) = Pubkey::find_program_address(
        &[
            pool_id.to_bytes().as_ref(),
            spl_token::id().to_bytes().as_ref(),
            mint.to_bytes().as_ref(),
        ],
        &spl_associated_token_account::id(),
    );

    let (pool_quote_ata, _) = Pubkey::find_program_address(
        &[
            pool_id.to_bytes().as_ref(),
            spl_token::id().to_bytes().as_ref(),
            native_mint::id().to_bytes().as_ref(),
        ],
        &spl_associated_token_account::id(),
    );

    //constant keys
    let pump_amm_global_config = Pubkey::from_str(PUMP_AMM_GLOBAL_CONFIG_ADDRESS).unwrap();
    let pump_amm_event_auth = Pubkey::from_str(PUMP_AMM_EVENT_AUTH_ADDRESS).unwrap();
    let pump_amm_protocol_fees = Pubkey::from_str(PUMP_AMM_PROTOCOL_FEES_ADDRESS).unwrap();

    let (protocol_fees_quote_ata, _) = Pubkey::find_program_address(
        &[
            pump_amm_protocol_fees.to_bytes().as_ref(),
            spl_token::id().to_bytes().as_ref(),
            native_mint::id().to_bytes().as_ref(),
        ],
        &spl_associated_token_account::id(),
    );

    PumpDexKeys {
        program_id: pump_amm_program_id,
        pool_authority: pool_authority,
        base_mint: *mint,
        quote_mint: native_mint::id(),
        bonding_curve: bonding_curve,
        pool_id: pool_id,
        lp_mint: lp_mint,
        pool_base_ata: pool_base_ata,
        pool_quote_ata: pool_quote_ata,
        pump_amm_global_config: pump_amm_global_config,
        pump_amm_event_auth: pump_amm_event_auth,
        pump_amm_protocol_fees: pump_amm_protocol_fees,
        pump_amm_protocol_fees_quote_ata: protocol_fees_quote_ata,
    }
}

#[derive(Serialize)]
pub struct LoginRequest {
    address: String,
    signature: String,
    timestamp: u64,
}

fn extract_auth_token(cookie_string: &str) -> Option<String> {
    const AUTH_TOKEN_KEY: &str = "auth_token=";
    let start_index = cookie_string.find(AUTH_TOKEN_KEY)?;
    let token_start = start_index + AUTH_TOKEN_KEY.len();
    let end_index = cookie_string[token_start..]
        .find(';')
        .map(|i| i + token_start);

    Some(match end_index {
        Some(end) => cookie_string[token_start..end].to_string(),
        None => cookie_string[token_start..].to_string(),
    })
}

pub async fn login(keypair: Arc<Keypair>) -> Result<String, String> {
    // Get current timestamp
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;

    // Create signing message
    let signing_message = format!("Sign in to pump.fun: {}", time);
    //info!("message: {signing_message}");
    let raw_signature = keypair.sign_message(signing_message.as_str().as_bytes());
    let base58_encoded = bs58::encode(raw_signature.as_ref()).into_string();
    //info!("{}", base58_encoded);

    //info!("base58 signature: {base58_signature}");

    // Create HTTP client with custom headers
    let client = Client::new();
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        header::ORIGIN,
        header::HeaderValue::from_static("https://pump.fun"),
    );
    headers.insert(
        header::REFERER,
        header::HeaderValue::from_static("https://pump.fun/"),
    );
    headers.insert(header::ACCEPT, header::HeaderValue::from_static("*/*"));

    //info!("address: {}", keypair.pubkey().to_string());
    // Prepare request body
    let request_body = LoginRequest {
        address: keypair.pubkey().to_string(),
        signature: base58_encoded.to_string(),
        timestamp: time,
    };
    //info!("before sending request");
    // Send request
    let response = client
        .post(LOGIN_ENDPOINT)
        .headers(headers)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    //info!("status: {:?}", response.status());
    //info!("{:#?}", response);

    // Get cookies from response
    let cookies: Vec<String> = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(String::from)
        .collect();

    // Find auth cookie
    let auth_cookie = cookies
        .iter()
        .find(|cookie| cookie.contains("auth_token"))
        .ok_or_else(|| "No auth token found in response".to_string())?;

    // Extract auth token
    let auth_token = extract_auth_token(auth_cookie)
        .ok_or_else(|| "Failed to extract auth token from cookie".to_string())?;

    Ok(auth_token)
}

#[derive(Serialize)]
struct RegisterRequest {
    address: String,
}

pub async fn register(auth_token: &str, address: &str) -> Result<bool, String> {
    // Create HTTP client with custom headers
    let client = Client::new();
    let mut headers = header::HeaderMap::new();

    // Add required headers
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        header::ORIGIN,
        header::HeaderValue::from_static("https://pump.fun"),
    );
    headers.insert(
        header::REFERER,
        header::HeaderValue::from_static("https://pump.fun/"),
    );
    headers.insert(header::ACCEPT, header::HeaderValue::from_static("*/*"));

    // Add cookie header
    let cookie_value = format!("auth_token={}", auth_token);
    headers.insert(
        header::COOKIE,
        header::HeaderValue::from_str(&cookie_value).map_err(|e| e.to_string())?,
    );

    // Prepare request body
    let request_body = RegisterRequest {
        address: address.to_string(),
    };

    // Send request and get status code
    let response = client
        .post(REGISTER_ENDPOINT)
        .headers(headers)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status().as_u16();

    //info!("register status: {}", response.status());

    Ok(status < 299)
}

#[derive(Deserialize, Debug)]
struct VideoUploadResponse {
    url: VideoUploadData,
}

#[derive(Deserialize, Debug)]
struct VideoUploadData {
    fields: VideoUploadFields,
}

#[derive(Deserialize, Debug)]
struct VideoUploadFields {
    bucket: String,
    key: String,
    #[serde(rename = "X-Amz-Algorithm")]
    x_amz_algorithm: String,
    #[serde(rename = "X-Amz-Credential")]
    x_amz_credential: String,
    #[serde(rename = "X-Amz-Date")]
    x_amz_date: String,
    policy: String,
    #[serde(rename = "X-Amz-Signature")]
    x_amz_signature: String,
}

pub async fn upload_metadata(
    auth_token: String,
    image_file_path: String,
    metadata: Metadata,
    use_video: bool,
) -> Result<(String, String, String), String> {
    let mut uploaded_vid_path = String::from("---");
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10)) // Allow up to 10 redirects
        .build()
        .map_err(|e| e.to_string())?;
    if use_video {
        let video_upload_url = "https://frontend-api.pump.fun/videos/get-signed-url?extension=mp4";

        let response = client
            .get(video_upload_url)
            .header("Authorization", format!("Bearer {}", auth_token))
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        //info!("vidoe url status: {:?}", response.status());

        let video_data: Option<VideoUploadResponse> = response.json().await.ok();

        let video_file = fs::read(&adjust_file_path("configurations/pump/media/video.mp4"))
            .map_err(|e| "Failed to open video file, Either not a video or malformed.")?;
        let video_part = Part::bytes(video_file)
            .file_name("video.mp4")
            .mime_str("video/mp4")
            .map_err(|e| "Could not set mime type to mp4")?;

        if let Some(data) = video_data {
            let form = Form::new()
                .part("file", video_part)
                .text("bucket", data.url.fields.bucket)
                .text("X-Amz-Algorithm", data.url.fields.x_amz_algorithm.clone())
                .text("X-Amz-Credential", data.url.fields.x_amz_credential.clone())
                .text("X-Amz-Date", data.url.fields.x_amz_date.clone())
                .text("key", data.url.fields.key.clone())
                .text("Policy", data.url.fields.policy)
                .text("X-Amz-Signature", data.url.fields.x_amz_signature.clone());

            let upload_response = client
                .post(data.url.fields.key.clone())
                .multipart(form)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if upload_response.status().is_success() {
                uploaded_vid_path = data.url.fields.key;
            } else {
                return Err(String::from("Failed to upload video"));
            }

            //info!("video upload status: {:?}", upload_response.status());
            //info!("video upload url: {}", uploaded_vid_path);
        } else {
            return Err(String::from("Failed to upload video"));
        }
    }

    let image_file = fs::read(&adjust_file_path(&format!(
        "configurations/pump/media/{}",
        &image_file_path
    )))
    .map_err(|e| "Failed to open image file.")?;

    // Method 2: String manipulation
    let extension = image_file_path.split('.').last().unwrap_or("png"); // Default to png if no extension found
    let mime_type = match extension {
        "jpg" | "jpeg" => "jpg",
        "png" => "png",
        "gif" => "gif",
        "webp" => "webp",
        _ => "png", // Default fallback
    };

    // info!("creating multipart form:");
    // info!("mime type: {}", mime_type);
    // info!("{}", &image_file_path);

    let image_part = Part::bytes(image_file)
        .file_name(image_file_path)
        .mime_str(&format!("image/{}", mime_type))
        .map_err(|e| "Could not set mime type for image")?;

    let form = Form::new()
        .part("file", image_part)
        .text("name", metadata.name)
        .text("symbol", metadata.symbol)
        .text("description", metadata.description)
        .text("twitter", metadata.twitter)
        .text("telegram", metadata.telegram)
        .text("website", metadata.website)
        .text("showName", "true");
    //.text(
    //    "video",
    //    if use_video {
    //        format!("https://media.pump.fun/{}", uploaded_vid_path)
    //    } else {
    //        "".to_string()
    //    },
    //);

    // info!("successfully created multipart form");

    let boundary = form.boundary().to_string();
    // info!("form boundary: {}", boundary);

    let response = client
        .post(UPLOAD_METADATA_ENDPOINT)
        .multipart(form)
        .header("Accept", "application/json, text/plain, */*")
        .header("Cookie", format!("auth_token={}", auth_token))
        .header("sec-ch-ua", r#""Google Chrome";v="125", "Chromium";v="125", "Not.A/Brand";v="24""#)
        .header("sec-ch-ua-platform", r#""Windows""#)
        .header("Referer", "https://pump.fun/create")
        .header("sec-ch-ua-mobile", "?0")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))  // Add this
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // info!("{:#?}", response);
    // info!("image and metadata upload status: {:?}", response.status());

    let data = response
        .json::<Value>()
        .await
        .map_err(|e| String::from("Could not parse metadata response data"))?;

    // info!("{data}");

    Ok((
        data.get("metadata")
            .ok_or_else(|| "Failed to upload metadata".to_string())?
            .get("image")
            .ok_or_else(|| "Failed to upload metadata".to_string())?
            .to_string()
            .trim_matches('"')
            .to_string(),
        data.get("metadataUri")
            .ok_or_else(|| "Failed to upload metadata".to_string())?
            .to_string()
            .trim_matches('"')
            .to_string(),
        uploaded_vid_path,
    ))
}

pub async fn fetch_pump_token_general_data(
    token_pubkey: &Pubkey,
) -> Result<GeneralCoinsFetchResponse, String> {
    // Construct the URL using the token pubkey
    let url = format!("{}/{}", GENERAL_COINS_ENDPOINT, token_pubkey);

    // Make the HTTP request and handle errors
    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Request error: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Failed to fetch data: HTTP {}", response.status()));
    }

    let response_json = response
        .json::<Value>()
        .await
        .map_err(|e| format!("Deserialization error: {}", e))?;

    let mint = response_json
        .get("mint")
        .unwrap()
        .as_str()
        .map(|s| s.to_string())
        .unwrap();
    let name = response_json
        .get("name")
        .unwrap()
        .as_str()
        .map(|s| s.to_string())
        .unwrap();
    let symbol = response_json
        .get("symbol")
        .unwrap()
        .as_str()
        .map(|s| s.to_string())
        .unwrap();
    let description = response_json
        .get("description")
        .unwrap()
        .as_str()
        .map(|s| s.to_string());
    let image_uri = response_json
        .get("image_uri")
        .unwrap()
        .as_str()
        .map(|s| s.to_string());
    let twitter = response_json
        .get("twitter")
        .unwrap()
        .as_str()
        .map(|s| s.to_string());
    let telegram = response_json
        .get("telegram")
        .unwrap()
        .as_str()
        .map(|s| s.to_string());
    let website = response_json
        .get("website")
        .unwrap()
        .as_str()
        .map(|s| s.to_string());
    let creator = response_json
        .get("creator")
        .unwrap()
        .as_str()
        .map(|s| s.to_string())
        .unwrap();
    let usdt_market_cap = response_json.get("usd_market_cap").unwrap().as_f64();

    //now I create the general reponse

    Ok(GeneralCoinsFetchResponse {
        mint,
        name,
        symbol,
        description,
        image_uri,
        twitter,
        telegram,
        website,
        creator,
        usdt_market_cap,
    })

    // Deserialize the response text and handle errors
    //response
    //    .json()
    //    .await
    //    .map_err(|e| format!("Deserialization error: {}", e))
}


pub async fn fetch_latest_koth() -> Result<Value, String> {
    // Make the HTTP request and handle errors
    let response = reqwest::get(KOTH_ENDPOINT)
        .await
        .map_err(|e| format!("Request error: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Failed to fetch data: HTTP {}", response.status()));
    }

    // Deserialize the response text and handle errors
    let deserialized_json: Value = response
        .json()
        .await
        .map_err(|e| format!("Deserialization error: {}", e))?;

    Ok(deserialized_json)
}
