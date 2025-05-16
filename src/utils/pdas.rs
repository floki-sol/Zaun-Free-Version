use std::str::{self, FromStr};

use solana_sdk::pubkey::Pubkey;

use crate::constants::general::{
    BUNDLER_GUARD_SEED, METADATA_SEED, METAPLEX_METADATA, PUMP_BONDING_CURVE_SEED, PUMP_CREATOR_VAULT_AUTHORITY_SEED, PUMP_CREATOR_VAULT_SEED
};

fn encode_utf8(value: &str) -> &[u8] {
    value.as_bytes()
}

pub fn get_bonding_curve(mint: &Pubkey, program_id: &Pubkey) -> Pubkey {
    let seeds: &[&[u8]] = &[
        PUMP_BONDING_CURVE_SEED,
        mint.as_ref(), // Convert mint to bytes
    ];
    Pubkey::find_program_address(&seeds, program_id).0
}

pub fn get_pump_creator_vault(creator_address: &Pubkey, program_id: &Pubkey) -> Pubkey {
    let seeds: &[&[u8]] = &[
        PUMP_CREATOR_VAULT_SEED,
        creator_address.as_ref(), // Convert mint to bytes
    ];
    Pubkey::find_program_address(&seeds, program_id).0
}

pub fn get_pumpswap_creator_vault_authority(
    creator_address: &Pubkey,
    program_id: &Pubkey,
) -> Pubkey {
    let seeds: &[&[u8]] = &[
        PUMP_CREATOR_VAULT_AUTHORITY_SEED,
        creator_address.as_ref(), // Convert mint to bytes
    ];
    Pubkey::find_program_address(&seeds, program_id).0
}

pub fn get_metadata_pda(mint: &Pubkey) -> Pubkey {
    let metaplex_metadata = Pubkey::from_str(METAPLEX_METADATA).unwrap().to_bytes();

    let seeds: &[&[u8]] = &[
        METADATA_SEED,
        metaplex_metadata.as_ref(), // Convert to bytes
        mint.as_ref(),              // Convert mint to bytes
    ];
    Pubkey::find_program_address(&seeds, &Pubkey::from_str(METAPLEX_METADATA).unwrap()).0
}

pub fn get_bundle_guard(owner: &Pubkey, program_id: &Pubkey) -> Pubkey {
    let seeds: &[&[u8]] = &[
        BUNDLER_GUARD_SEED,
        owner.as_ref(), // Convert owner to bytes
    ];
    Pubkey::find_program_address(&seeds, program_id).0
}
