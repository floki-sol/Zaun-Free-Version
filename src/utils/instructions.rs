use std::{str::FromStr, sync::Arc};

use solana_sdk::{
    address_lookup_table::{
        instruction::{
            close_lookup_table, create_lookup_table, deactivate_lookup_table, extend_lookup_table,
        },
        state::AddressLookupTable,
    },
    clock::Slot,
    feature_set::rent_for_sysvars,
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account_idempotent,
};

use borsh::BorshSerialize;

use crate::{
    constants::general::{
        PumpKeys, SwapType, AMM_V4, BOT_PROGRAM_ID, MEMO_PROGRAM_ID, METAPLEX_METADATA, MINT_AUTH,
        PUMP_AMM_ADDRESS, SYSVAR_RENT_ID,
    },
    utils::pdas::get_metadata_pda,
};

use super::{
    callbacks::pump, misc::get_associated_accounts, pump_helpers::PumpDexKeys,
};

const PUMP_CREATE_IX_DISCRIMINATOR: [u8; 8] = [24, 30, 200, 40, 5, 28, 7, 119];
const PUMP_BUY_IX_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
const PUMP_SELL_IX_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
const IN_CONTRACT_BUY_IX_DISCRIMINATOR: [u8; 8] = [192, 150, 68, 220, 2, 7, 59, 222];
const PUMP_DEX_SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];

pub fn get_create_pump_token_instruction(
    pump_keys: &PumpKeys,
    creator: &Pubkey,
    name: &str,
    symbol: &str,
    metadata_uri: &str,
) -> Instruction {
    let metadata_pda = get_metadata_pda(&pump_keys.mint);
    let account_metas = vec![
        AccountMeta::new(pump_keys.mint, true),
        AccountMeta::new_readonly(Pubkey::from_str(MINT_AUTH).unwrap(), false),
        AccountMeta::new(pump_keys.bonding_curve, false),
        AccountMeta::new(pump_keys.associated_bonding_curve, false),
        AccountMeta::new_readonly(pump_keys.global_state, false),
        AccountMeta::new_readonly(Pubkey::from_str(METAPLEX_METADATA).unwrap(), false),
        AccountMeta::new(metadata_pda, false),
        AccountMeta::new(*creator, true),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        AccountMeta::new_readonly(Pubkey::from_str(SYSVAR_RENT_ID).unwrap(), false),
        AccountMeta::new_readonly(pump_keys.event_auth, false),
        AccountMeta::new_readonly(pump_keys.pump_program_id, false),
    ];

    let mut data: Vec<u8> = vec![];
    data.extend_from_slice(&PUMP_CREATE_IX_DISCRIMINATOR); // instruction discriminator

    // Serialize strings with their lengths
    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.extend_from_slice(&(symbol.len() as u32).to_le_bytes());
    data.extend_from_slice(symbol.as_bytes());

    data.extend_from_slice(&(metadata_uri.len() as u32).to_le_bytes());
    data.extend_from_slice(metadata_uri.as_bytes());

    data.extend_from_slice(&creator.to_bytes());

    let create_token_ix = Instruction {
        program_id: pump_keys.pump_program_id,
        accounts: account_metas,
        data,
    };

    create_token_ix
}

pub fn get_buy_pump_token_instructions(
    pump_keys: &PumpKeys,
    buyer: &Pubkey,
    raw_token_amount: u64,
    max_sol_amount: u64,
    //should_create_ata: bool,
) -> Instruction {
    let buyer_ata = get_associated_token_address(buyer, &pump_keys.mint);

    let account_metas = vec![
        AccountMeta::new_readonly(pump_keys.global_state, false),
        AccountMeta::new(pump_keys.fees, false),
        AccountMeta::new_readonly(pump_keys.mint, false),
        AccountMeta::new(pump_keys.bonding_curve, false),
        AccountMeta::new(pump_keys.associated_bonding_curve, false),
        AccountMeta::new(buyer_ata, false),
        AccountMeta::new(*buyer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
        AccountMeta::new_readonly(pump_keys.event_auth, false),
        AccountMeta::new_readonly(pump_keys.pump_program_id, false),
    ];

    let mut data: Vec<u8> = vec![];
    data.extend_from_slice(&PUMP_BUY_IX_DISCRIMINATOR); // instruction discriminator
    data.extend_from_slice(&raw_token_amount.to_le_bytes());
    data.extend_from_slice(&max_sol_amount.to_le_bytes());

    let buy_token_ix = Instruction {
        program_id: pump_keys.pump_program_id,
        accounts: account_metas,
        data,
    };

    buy_token_ix
}

pub fn get_sell_pump_token_instructions(
    pump_keys: &PumpKeys,
    seller: &Pubkey,
    raw_token_amount: u64,
    min_sol_amount: u64,
) -> Instruction {
    let seller_ata = get_associated_token_address(seller, &pump_keys.mint);

    let account_metas = vec![
        AccountMeta::new_readonly(pump_keys.global_state, false),
        AccountMeta::new(pump_keys.fees, false),
        AccountMeta::new_readonly(pump_keys.mint, false),
        AccountMeta::new(pump_keys.bonding_curve, false),
        AccountMeta::new(pump_keys.associated_bonding_curve, false),
        AccountMeta::new(seller_ata, false),
        AccountMeta::new(*seller, true),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(pump_keys.event_auth, false),
        AccountMeta::new_readonly(pump_keys.pump_program_id, false),
    ];

    let mut data: Vec<u8> = vec![];
    data.extend_from_slice(&PUMP_SELL_IX_DISCRIMINATOR); // instruction discriminator
    data.extend_from_slice(&raw_token_amount.to_le_bytes());
    data.extend_from_slice(&min_sol_amount.to_le_bytes());

    let sell_token_ix = Instruction {
        program_id: pump_keys.pump_program_id,
        accounts: account_metas,
        data: data,
    };

    sell_token_ix
}

pub fn get_in_contract_pump_buy_instruction(
    pump_keys: &PumpKeys,
    funder: &Pubkey,
    buyer: &Pubkey,
    sol_amount: u64,
    in_contract_funding: bool,
) -> Instruction {
    let buyer_ata = get_associated_token_address(buyer, &pump_keys.mint);

    let account_metas = vec![
        AccountMeta::new_readonly(pump_keys.global_state, false),
        AccountMeta::new(pump_keys.fees, false),
        AccountMeta::new_readonly(pump_keys.mint, false),
        AccountMeta::new(pump_keys.bonding_curve, false),
        AccountMeta::new(pump_keys.associated_bonding_curve, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
        AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        AccountMeta::new_readonly(pump_keys.event_auth, false),
        AccountMeta::new_readonly(pump_keys.pump_program_id, false),
        AccountMeta::new(*funder, true),
        AccountMeta::new(*buyer, true),
        AccountMeta::new(buyer_ata, false),
    ];

    let mut data: Vec<u8> = vec![];
    data.extend_from_slice(&IN_CONTRACT_BUY_IX_DISCRIMINATOR); // instruction discriminator
    data.extend_from_slice(&[in_contract_funding as u8]);
    data.extend_from_slice(&sol_amount.to_le_bytes());

    Instruction {
        program_id: Pubkey::from_str(BOT_PROGRAM_ID).unwrap(),
        accounts: account_metas,
        data: data,
    }
}

pub fn get_unique_memo_ix() -> Instruction {
    let unique_identifier = format!("Nonce: {}", rand::random::<u8>());
    Instruction {
        program_id: Pubkey::from_str(MEMO_PROGRAM_ID).unwrap(),
        accounts: vec![],
        data: unique_identifier.as_bytes().to_vec(),
    }
}

pub fn get_create_bundle_guard_ix(owner: Pubkey, bundle_blocker: Pubkey) -> Instruction {
    let mut data: Vec<u8> = vec![];
    data.extend_from_slice(&[29, 44, 126, 97, 170, 21, 50, 100]); // instruction discriminator

    Instruction {
        program_id: Pubkey::from_str(BOT_PROGRAM_ID).unwrap(),
        accounts: vec![
            AccountMeta {
                pubkey: owner,
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: bundle_blocker,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: system_program::id(),
                is_signer: false,
                is_writable: false,
            },
        ],
        data: data,
    }
}

pub fn get_increment_bundle_guard_ix(
    owner: Pubkey,
    bundle_blocker: &Pubkey,
    original_nonce: u64,
    curr_lut: Option<Pubkey>,
) -> Instruction {
    fn option_pubkey_to_bytes(pubkey_option: Option<Pubkey>) -> Vec<u8> {
        match pubkey_option {
            Some(pubkey) => {
                // Create a vec with 1 byte to indicate Some, then pubkey bytes
                let mut bytes = vec![1];
                bytes.extend_from_slice(&pubkey.to_bytes());
                bytes
            }
            None => {
                // Create a vec with just a 0 byte to indicate None
                vec![0]
            }
        }
    }

    let mut data: Vec<u8> = vec![];
    data.extend_from_slice(&[223, 147, 147, 228, 97, 180, 96, 207]);
    data.extend_from_slice(&original_nonce.to_le_bytes());
    data.extend_from_slice(&option_pubkey_to_bytes(curr_lut));

    Instruction {
        program_id: Pubkey::from_str(BOT_PROGRAM_ID).unwrap(),
        accounts: vec![
            AccountMeta {
                pubkey: owner,
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: *bundle_blocker,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: system_program::id(),
                is_signer: false,
                is_writable: false,
            },
        ],
        data: data,
    }
}


pub fn get_create_lookup_table_ix(funder: Arc<Keypair>, slot: u64) -> (Instruction, Pubkey) {
    create_lookup_table(funder.pubkey(), funder.pubkey(), slot)
}

pub fn get_extend_lookup_table_ix(
    funder: Arc<Keypair>,
    wallets: &Vec<Pubkey>,
    lookup_table: Pubkey,
) -> Instruction {
    extend_lookup_table(
        lookup_table,
        funder.pubkey(),
        Some(funder.pubkey()),
        wallets.clone(),
    )
}

pub fn get_deactivate_lut_ix(funder: Arc<Keypair>, lookup_table: Pubkey) -> Instruction {
    deactivate_lookup_table(lookup_table, funder.pubkey())
}

pub fn get_close_lut_ix(funder: Arc<Keypair>, lookup_table: Pubkey) -> Instruction {
    close_lookup_table(lookup_table, funder.pubkey(), funder.pubkey())
}



pub fn create_pf_amm_sell_instruction(
    trader: Arc<Keypair>,
    pool_keys: Arc<PumpDexKeys>,
    base_amount_to_sell: u64,
    quote_min_amount_out: u64,
) -> Instruction {
    let user_base_ata = get_associated_token_address(&trader.pubkey(), &pool_keys.base_mint);
    let user_quote_ata = get_associated_token_address(&trader.pubkey(), &pool_keys.quote_mint);

    let account_metas = vec![
        AccountMeta::new_readonly(pool_keys.pool_id, false),
        AccountMeta::new(trader.pubkey(), true),
        AccountMeta::new_readonly(pool_keys.pump_amm_global_config, false),
        AccountMeta::new_readonly(pool_keys.base_mint, false),
        AccountMeta::new_readonly(pool_keys.quote_mint, false),
        AccountMeta::new(user_base_ata, false),
        AccountMeta::new(user_quote_ata, false),
        AccountMeta::new(pool_keys.pool_base_ata, false),
        AccountMeta::new(pool_keys.pool_quote_ata, false),
        AccountMeta::new(pool_keys.pump_amm_protocol_fees, false),
        AccountMeta::new(pool_keys.pump_amm_protocol_fees_quote_ata, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        AccountMeta::new_readonly(pool_keys.pump_amm_event_auth, false),
        AccountMeta::new_readonly(pool_keys.program_id, false),
    ];

    let mut data = vec![51, 230, 133, 164, 1, 127, 131, 173];
    data.extend_from_slice(&base_amount_to_sell.to_le_bytes());
    data.extend_from_slice(&quote_min_amount_out.to_le_bytes());

    Instruction {
        program_id: pool_keys.program_id,
        accounts: account_metas,
        data,
    }
}
