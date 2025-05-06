use dotenv::dotenv;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use std::env;
use std::sync::Arc;
use url::Url;

#[derive(Debug)]
pub struct EnvConfig {
    pub rpc_https_endpoint_url: String,
    pub rpc_wss_endpoint_url: String,
    pub subscription_wallet_private_key: String,
    pub funding_wallet_private_key: String,
    pub dev_wallet_private_key: String,
    pub cap_solver_api_key: String,
}

impl EnvConfig {
    pub fn load_env() -> Result<Self, String> {
        // Load the .env file if present
        dotenv().ok();

        // Load and validate environment variables with context for better error messages
        let rpc_https_endpoint_url = env::var("RPC_HTTPS_ENDPOINT_URL")
            .map_err(|e| "Missing or invalid RPC_HTTPS_ENDPOINT_URL")?;

        let rpc_wss_endpoint_url = env::var("RPC_WSS_ENDPOINT_URL")
            .map_err(|e| "Missing or invalid RPC_WSS_ENDPOINT_URL")?;

        let funding_wallet_private_key = env::var("FUNDING_WALLET_PRIVATE_KEY")
            .map_err(|e| "Missing or invalid FUNDING_WALLET_PRIVATE_KEY")?;

        let subscription_wallet_private_key = env::var("SUBSCRIPTION_WALLET_PRIVATE_KEY")
            .map_err(|e| "Missing or invalid SUBSCRIPTION_WALLET_PRIVATE_KEY")?;

        let dev_wallet_private_key = env::var("DEV_WALLET_PRIVATE_KEY")
            .map_err(|e| "Missing or invalid DEV_WALLET_PRIVATE_KEY")?;
        // Validate non-empty strings
        if rpc_https_endpoint_url.is_empty() {
            return Err(String::from("RPC_HTTPS_ENDPOINT_URL cannot be empty"));
        }

        if rpc_wss_endpoint_url.is_empty() {
            return Err(String::from("RPC_WSS_ENDPOINT_URL cannot be empty"));
        }

        if subscription_wallet_private_key.is_empty() {
            return Err(String::from(
                "SUBSCRIPTION_WALLET_PRIVATE_KEY cannot be empty",
            ));
        }

        if funding_wallet_private_key.is_empty() {
            return Err(String::from("FUNDING_WALLET_PRIVATE_KEY cannot be empty"));
        }
        if dev_wallet_private_key.is_empty() {
            return Err(String::from("DEV_WALLET_PRIVATE_KEY cannot be empty"));
        }

        // Validate that the rpc https and wss endpoint is a valid URL
        Url::parse(&rpc_https_endpoint_url)
            .map_err(|_| String::from("RPC_HTTPS_ENDPOINT_URL is not a valid URL"))?;

        Url::parse(&rpc_wss_endpoint_url)
            .map_err(|_| String::from("RPC_WSS_ENDPOINT_URL is not a valid URL"))?;

        //validate base58 strings for wallets
        let funding_wallet_byte_array = bs58::decode(&funding_wallet_private_key)
            .into_vec()
            .map_err(|_| String::from("Invalid base58 private key for funding wallet"))?;

        let subscription_wallet_byte_array = bs58::decode(&subscription_wallet_private_key)
            .into_vec()
            .map_err(|_| String::from("Invalid base58 private key for subscription wallet"))?;

        let dev_wallet_byte_array = bs58::decode(&dev_wallet_private_key)
            .into_vec()
            .map_err(|_| String::from("Invalid base58 private key for dev wallet"))?;

        // Validate that the funding and dev wallet private keys are valid Solana private keys (base58 format)
        let funding_wallet_keypair = Keypair::from_bytes(&funding_wallet_byte_array.as_slice())
            .map_err(|_| String::from("Invalid base58 private key for funding wallet"))?;

        let subscription_wallet_keypair =
            Keypair::from_bytes(&&subscription_wallet_byte_array.as_slice())
                .map_err(|_| String::from("Invalid base58 private key for subscription wallet"))?;

        let dev_wallet_keypair = Keypair::from_bytes(&dev_wallet_byte_array.as_slice())
            .map_err(|_| String::from("Invalid base58 private key for dev wallet"))?;

        //wallets are not unique
        if funding_wallet_private_key == dev_wallet_private_key {
            return Err(String::from("Funder and Dev wallet must be unique"));
        }

        Ok(EnvConfig {
            rpc_https_endpoint_url,
            rpc_wss_endpoint_url,
            subscription_wallet_private_key,
            funding_wallet_private_key,
            dev_wallet_private_key,
            cap_solver_api_key:String::new(),
        })
    }

    
    pub fn get_subscription_keypair(&self) -> Arc<Keypair> {
        Arc::new(Keypair::from_base58_string(
            &self.subscription_wallet_private_key,
        ))
    }
    
    pub fn get_funding_keypair(&self) -> Arc<Keypair> {
        Arc::new(Keypair::from_base58_string(
            &self.funding_wallet_private_key,
        ))
    }

    pub fn get_dev_keypair(&self) -> Arc<Keypair> {
        Arc::new(Keypair::from_base58_string(&self.dev_wallet_private_key))
    }

    pub fn get_rpc_client(&self) -> Arc<RpcClient> {
        Arc::new(RpcClient::new(self.rpc_https_endpoint_url.clone()))
    }

    pub fn get_rpc_wss_url(&self) -> Arc<String> {
        Arc::new(self.rpc_wss_endpoint_url.clone())
    }

    //pub fn get_capsolver_api_key(&self) -> Arc<String> {
    //    Arc::new(self.cap_solver_api_key.clone())
    //}
}
