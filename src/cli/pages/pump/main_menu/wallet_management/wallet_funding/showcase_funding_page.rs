use solana_sdk::{signature::Keypair, signer::Signer};

use crate::{
    cli::{
        info::{display_info_page, InfoSegment},
        menu::MenuHandler,
        options::OptionCallback,
    },
    utils::misc::{FundingStrategy, WalletsFundingType},
};

pub fn display_funding_showcase_page(
    wallets_vec: Vec<String>,
    amounts: Vec<f64>,
    handler_ref: &mut MenuHandler,
    funding_type: String,
) {
    let typed_funding = match funding_type.to_lowercase().as_str() {
        "in-contract" => FundingStrategy::InContract,
        "pre-fund" => FundingStrategy::PreFund,
        _ => FundingStrategy::InContract,
    };

    let mut info_segment_entries: Vec<InfoSegment> = vec![
        InfoSegment::Emphasized(format!("Funding Preview ({}):", typed_funding)),
        match typed_funding {
            FundingStrategy::PreFund => {
                InfoSegment::Normal(String::from("Wallets will be funded after confirming"))
            }
            FundingStrategy::InContract => InfoSegment::Normal(String::from(
                "No transactions will be sent, manifest file will be created instead",
            )),
        },
    ];
    for (idx, bs58_keypair) in wallets_vec.iter().enumerate() {
        let decoded_bytes = bs58::decode(bs58_keypair.clone()).into_vec().unwrap();
        let keypair = Keypair::from_bytes(&decoded_bytes).unwrap();

        info_segment_entries.push(InfoSegment::Normal("".to_string()));
        info_segment_entries.push(InfoSegment::Normal(format!("Wallet {}", idx + 1)));
        info_segment_entries.push(InfoSegment::StringSplitInfo((
            String::from("-- Address: "),
            keypair.pubkey().to_string(),
        )));
        info_segment_entries.push(InfoSegment::NumericSplitInfo((
            String::from("-- Funding: "),
            amounts[idx].to_string(),
        )));
    }

    handler_ref.to_previous_page();
    display_info_page(
        info_segment_entries,
        String::from("Funding"),
        handler_ref,
        Some(OptionCallback::FundBundleWallets(
            WalletsFundingType::Initiate(amounts),
        )),
        None,
        Some(17),
    );
}
