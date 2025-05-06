use std::{cmp, str::FromStr, sync::Arc};

use bincode::serialized_size;
use log::info;
use serde::Serialize;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcSimulateTransactionAccountsConfig, RpcSimulateTransactionConfig},
};
use solana_sdk::{
    address_lookup_table::{state::AddressLookupTable, AddressLookupTableAccount},
    commitment_config::CommitmentConfig,
    compute_budget,
    instruction::Instruction,
    message::{v0::Message, VersionedMessage},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
    transaction::{Transaction, VersionedTransaction},
};
use spl_associated_token_account::{
    get_associated_token_address,
    instruction::{create_associated_token_account, create_associated_token_account_idempotent},
};
use spl_token::{
    instruction::{burn, close_account, sync_native, transfer},
    native_mint,
};

use crate::{
    constants::general::{
        GlobalState, LutCallback, PumpKeys, SwapType, TokenAccountBalanceState, BOT_PROGRAM_ID,
        MEMO_PROGRAM_ID,
    },
    jito::{
        bundles::{create_bundle, Bundle},
        tip::get_random_tip_account,
    },
    utils::misc::get_associated_accounts,
};

use super::{
    blockhash_manager::RecentBlockhashManager,
    bonding_curve_provider::{BondingCurve, BondingCurveProvider},
    instructions::{
        create_pf_amm_sell_instruction,
        get_buy_pump_token_instructions, get_close_lut_ix, get_create_bundle_guard_ix,
        get_create_lookup_table_ix, get_create_pump_token_instruction, get_deactivate_lut_ix,
        get_extend_lookup_table_ix, get_in_contract_pump_buy_instruction,
        get_increment_bundle_guard_ix,
        get_sell_pump_token_instructions, get_unique_memo_ix,
    },
    misc::calculate_pump_tokens_to_buy,
    pump_helpers::PumpDexKeys,
};

pub fn get_fund_wallet_bundle(
    blockhash_provider: Arc<RecentBlockhashManager>,
    sender: Arc<Keypair>,
    receiver: Pubkey,
    bundle_guard: &Pubkey,
    original_nonce: u64,
    amount_to_send: u64,
    tip: u64,
) -> Bundle {
    //let balance_validation_ix = get_validate_balance_ix(sender.pubkey(), expected_sol_balance);
    let bundle_guard_ix =
        get_increment_bundle_guard_ix(sender.pubkey(), bundle_guard, original_nonce, None);

    let compute_price_ix =
        compute_budget::ComputeBudgetInstruction::set_compute_unit_price(1500000);

    let compute_limit_ix = compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(13000);

    // Memo instruction to add uniqueness
    let memo_ix = get_unique_memo_ix();

    let transfer_ix = system_instruction::transfer(&sender.pubkey(), &receiver, amount_to_send);
    let tip_ix = system_instruction::transfer(&sender.pubkey(), &get_random_tip_account(), tip);

    let tx_ixs = vec![
        compute_price_ix,
        compute_limit_ix,
        memo_ix,
        bundle_guard_ix,
        transfer_ix,
        tip_ix,
    ];

    let tx = VersionedTransaction::try_new(
        VersionedMessage::V0(
            Message::try_compile(
                &sender.pubkey(),
                &tx_ixs,
                &[],
                blockhash_provider.get_recent_blockhash(),
            )
            .unwrap(),
        ),
        &[Arc::clone(&sender)],
    );

    let tx = tx.unwrap();
    //println!("{:?}", bs58::encode(tx.signatures[0]).into_string());

    create_bundle(vec![tx])

    //create_bundle(vec![tx.unwrap()])
}

pub fn get_fund_wallets_bundle(
    blockhash_provider: Arc<RecentBlockhashManager>,
    sender: Arc<Keypair>,
    wallets: &Vec<Pubkey>,
    amounts: &Vec<f64>,
    bundle_guard: &Pubkey,
    original_nonce: u64,
    tip: u64,
) -> Bundle {
    let group_size = 4;
    let num_groups = (wallets.len() + group_size - 1) / group_size;

    let mut all_txs: Vec<VersionedTransaction> = vec![];

    for group_idx in 0..num_groups {
        //here I create a versioned transaction

        let start_idx = group_idx * group_size;
        let end_idx = std::cmp::min((group_idx + 1) * group_size, wallets.len());
        let is_first_group = group_idx == 0;
        let is_last_group = group_idx == num_groups - 1;

        let mut curr_ixs: Vec<Instruction> = vec![];

        let compute_price_ix =
            compute_budget::ComputeBudgetInstruction::set_compute_unit_price(1500000);

        let compute_limit_ix = compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            12000 + (600 * (end_idx - start_idx)) as u32,
        );

        let memo_ix = get_unique_memo_ix();

        curr_ixs.extend_from_slice(&[compute_price_ix, compute_limit_ix, memo_ix]);

        if is_last_group {
            let tip_ix =
                system_instruction::transfer(&sender.pubkey(), &get_random_tip_account(), tip);
            curr_ixs.push(tip_ix);
        }

        if is_first_group {
            //let balance_validation_ix = get_validate_balance_ix(sender.pubkey(), expected_sol_balance);
            let bundle_guard_ix =
                get_increment_bundle_guard_ix(sender.pubkey(), bundle_guard, original_nonce, None);
            curr_ixs.push(bundle_guard_ix);
        }

        // Loop over wallets within the current group
        for idx in start_idx..end_idx {
            let wallet = &wallets[idx];
            let amount = &amounts[idx];

            let transfer_ix = system_instruction::transfer(
                &sender.pubkey(),
                wallet,
                (amount * LAMPORTS_PER_SOL as f64) as u64,
            );
            curr_ixs.push(transfer_ix);
        }

        let tx = VersionedTransaction::try_new(
            VersionedMessage::V0(
                Message::try_compile(
                    &sender.pubkey(),
                    &curr_ixs,
                    &[],
                    blockhash_provider.get_recent_blockhash(),
                )
                .unwrap(),
            ),
            &[Arc::clone(&sender)],
        );
        all_txs.push(tx.unwrap());
    }
    //all_txs
    create_bundle(all_txs)
}

pub fn get_retrieve_sol_bundle(
    blockhash_provider: Arc<RecentBlockhashManager>,
    funder_keypair: Arc<Keypair>,
    senders: &Vec<Keypair>,
    receiver: Pubkey,
    bundle_guard: &Pubkey,
    original_nonce: u64,
    amounts_to_send: &Vec<u64>,
    tip: u64,
) -> Bundle {
    //info!("before group calculation");
    let group_size = 4;
    let num_groups = (senders.len() + group_size - 1) / group_size;
    //info!("after group calculation");
    //info!("groups: {}", num_groups);

    let mut all_txs: Vec<VersionedTransaction> = vec![];

    for group_idx in 0..num_groups {
        let start_idx = group_idx * group_size;
        let end_idx = std::cmp::min((group_idx + 1) * group_size, senders.len());
        let is_first_group = group_idx == 0;
        let is_last_group = group_idx == num_groups - 1;

        let mut curr_ixs: Vec<Instruction> = vec![];
        let mut curr_signers: Vec<Arc<Keypair>> = vec![Arc::clone(&funder_keypair)];

        let compute_price_ix =
            compute_budget::ComputeBudgetInstruction::set_compute_unit_price(1500000);

        let compute_limit_ix = compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            12000 + (600 * (end_idx - start_idx)) as u32,
        );

        let memo_ix = get_unique_memo_ix();

        curr_ixs.extend_from_slice(&[compute_price_ix, compute_limit_ix, memo_ix]);

        if is_last_group {
            let tip_ix = system_instruction::transfer(
                &funder_keypair.pubkey(),
                &get_random_tip_account(),
                tip,
            );
            curr_ixs.push(tip_ix);
        }

        if is_first_group {
            //let balance_validation_ix = get_validate_balance_ix(sender.pubkey(), expected_sol_balance);
            let bundle_guard_ix = get_increment_bundle_guard_ix(
                funder_keypair.pubkey(),
                bundle_guard,
                original_nonce,
                None,
            );
            curr_ixs.push(bundle_guard_ix);
        }

        // Loop over wallets within the current group
        for idx in start_idx..end_idx {
            let wallet = &senders[idx];
            let amount = amounts_to_send[idx];
            //info!("{}, {}", wallet.pubkey().to_string(), amount);

            let transfer_ix = system_instruction::transfer(&wallet.pubkey(), &receiver, amount);
            curr_signers.push(Arc::new(wallet.insecure_clone()));
            curr_ixs.push(transfer_ix);
        }

        let message = Message::try_compile(
            &funder_keypair.pubkey(),
            &curr_ixs,
            &[],
            blockhash_provider.get_recent_blockhash(),
        )
        .unwrap();
        //info!("after message creation");
        let versioned_message = VersionedMessage::V0(message);
        //info!("after versioned message creation");
        // /info!("{:?}", message);
        let tx = VersionedTransaction::try_new(versioned_message, &curr_signers).unwrap();
        all_txs.push(tx);
    }

    create_bundle(all_txs)
}

pub fn get_burn_or_retrieve_tokens_bundle(
    blockhash_provider: Arc<RecentBlockhashManager>,
    mint: &Pubkey,
    funder_keypair: Arc<Keypair>,
    burners_or_senders: &Vec<Arc<Keypair>>,
    receiver: Pubkey,
    bundle_guard: &Pubkey,
    original_nonce: u64,
    balance_states: &Vec<TokenAccountBalanceState>,
    tip: u64,
    is_burn: bool,
) -> Bundle {
    //info!("before group calculation");
    let group_size = 4;
    let num_groups = (burners_or_senders.len() + group_size - 1) / group_size;
    //info!("after group calculation");
    //info!("groups: {}", num_groups);

    let mut all_txs: Vec<VersionedTransaction> = vec![];
    let receiver_ata = get_associated_token_address(&receiver, mint);

    for group_idx in 0..num_groups {
        let start_idx = group_idx * group_size;
        let end_idx = std::cmp::min((group_idx + 1) * group_size, burners_or_senders.len());
        let is_first_group = group_idx == 0;
        let is_last_group = group_idx == num_groups - 1;

        let mut curr_ixs: Vec<Instruction> = vec![];
        let mut curr_signers: Vec<Arc<Keypair>> = vec![Arc::clone(&funder_keypair)];

        let compute_price_ix =
            compute_budget::ComputeBudgetInstruction::set_compute_unit_price(20000);

        let compute_limit_ix = compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            12000 + (20000 * (end_idx - start_idx)) as u32,
        );

        let memo_ix = get_unique_memo_ix();

        curr_ixs.extend_from_slice(&[compute_price_ix, compute_limit_ix, memo_ix]);

        if is_last_group {
            let tip_ix = system_instruction::transfer(
                &funder_keypair.pubkey(),
                &get_random_tip_account(),
                tip,
            );
            curr_ixs.push(tip_ix);
        }

        if is_first_group {
            //let balance_validation_ix = get_validate_balance_ix(sender.pubkey(), expected_sol_balance);
            let bundle_guard_ix = get_increment_bundle_guard_ix(
                funder_keypair.pubkey(),
                bundle_guard,
                original_nonce,
                None,
            );

            //create the receivers associated accounts
            let create_ata_ix = create_associated_token_account_idempotent(
                &funder_keypair.pubkey(),
                &receiver,
                &mint,
                &spl_token::ID,
            );
            curr_ixs.push(bundle_guard_ix);
            curr_ixs.push(create_ata_ix);
        }

        // Loop over wallets within the current group
        for idx in start_idx..end_idx {
            let wallet = &burners_or_senders[idx];
            let ata = get_associated_token_address(&wallet.pubkey(), mint);
            let balance_state = balance_states[idx].clone();
            //info!("{}, {}", wallet.pubkey().to_string(), amount);

            match balance_state {
                TokenAccountBalanceState::ExistsWithNoBalance => {
                    //do nothing, close only the token account
                }
                TokenAccountBalanceState::ExistsWithBalance(bal) => {
                    if is_burn {
                        let burn_ix =
                            burn(&spl_token::ID, &ata, mint, &wallet.pubkey(), &[], bal).unwrap();
                        curr_ixs.push(burn_ix);
                    } else {
                        let transfer_ix = transfer(
                            &spl_token::ID,
                            &ata,
                            &receiver_ata,
                            &wallet.pubkey(),
                            &[],
                            bal,
                        )
                        .unwrap();
                        curr_ixs.push(transfer_ix);
                    }
                }
                _ => continue,
            }

            let close_ata_ix = close_account(
                &spl_token::ID,
                &ata,
                &funder_keypair.pubkey(),
                &wallet.pubkey(),
                &[],
            )
            .unwrap();
            curr_ixs.push(close_ata_ix);

            //let transfer_ix = system_instruction::transfer(&wallet.pubkey(), &receiver, amount);
            curr_signers.push(Arc::new(wallet.insecure_clone()));
        }

        let message = Message::try_compile(
            &funder_keypair.pubkey(),
            &curr_ixs,
            &[],
            blockhash_provider.get_recent_blockhash(),
        )
        .unwrap();
        //info!("after message creation");
        let versioned_message = VersionedMessage::V0(message);
        //info!("after versioned message creation");
        // /info!("{:?}", message);
        let tx = VersionedTransaction::try_new(versioned_message, &curr_signers).unwrap();
        all_txs.push(tx);
    }

    create_bundle(all_txs)
}

pub fn get_create_bundle_guard_bundle(
    owner: Arc<Keypair>,
    blockhash_provider: Arc<RecentBlockhashManager>,
    bundle_guard: Pubkey,
    tip: u64,
) -> Bundle {
    let bundle_guard_ix = get_create_bundle_guard_ix(owner.pubkey(), bundle_guard);

    let memo_ix = get_unique_memo_ix();

    let tip_ix = system_instruction::transfer(&owner.pubkey(), &get_random_tip_account(), tip);

    let tx_ixs = vec![bundle_guard_ix, memo_ix, tip_ix];

    let tx = VersionedTransaction::try_new(
        VersionedMessage::V0(
            Message::try_compile(
                &owner.pubkey(),
                &tx_ixs,
                &[],
                blockhash_provider.get_recent_blockhash(),
            )
            .unwrap(),
        ),
        &[Arc::clone(&owner)],
    );

    let tx = tx.unwrap();
    //println!("{:?}", bs58::encode(tx.signatures[0]).into_string());

    create_bundle(vec![tx])
}

pub fn get_create_lookup_table_bundle(
    blockhash_provider: Arc<RecentBlockhashManager>,
    bundle_guard: &Pubkey,
    slot: u64,
    original_nonce: u64,
    funder: Arc<Keypair>,
    wallets: &Vec<Pubkey>,
    tip: u64,
) -> Bundle {
    let compute_price_ix =
        compute_budget::ComputeBudgetInstruction::set_compute_unit_price(200_000);

    let compute_limit_ix = compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(60_000);

    let memo_ix = get_unique_memo_ix();

    //let transfer_ix = system_instruction::transfer(&sender.pubkey(), &receiver, amount_to_send);
    let tip_ix = system_instruction::transfer(&funder.pubkey(), &get_random_tip_account(), tip);

    let (create_lut_ix, lut) = get_create_lookup_table_ix(Arc::clone(&funder), slot);
    let extend_lut_ix = get_extend_lookup_table_ix(Arc::clone(&funder), &wallets, lut);

    //let balance_validation_ix = get_validate_balance_ix(sender.pubkey(), expected_sol_balance);
    let bundle_guard_ix =
        get_increment_bundle_guard_ix(funder.pubkey(), bundle_guard, original_nonce, Some(lut));
    //curr_ixs.push(bundle_guard_ix);

    //let create_alu_ix = get_create_lookup_table_ix(Arc::clone(&funder), slot);

    let tx_ixs = vec![
        compute_price_ix,
        compute_limit_ix,
        memo_ix,
        bundle_guard_ix,
        create_lut_ix,
        extend_lut_ix,
        tip_ix,
    ];

    let tx = VersionedTransaction::try_new(
        VersionedMessage::V0(
            Message::try_compile(
                &funder.pubkey(),
                &tx_ixs,
                &[],
                blockhash_provider.get_recent_blockhash(),
            )
            .unwrap(),
        ),
        &[Arc::clone(&funder)],
    );

    let tx = tx.unwrap();
    create_bundle(vec![tx])
    //Bundle::new(vec![]).unwrap()
}

pub fn get_redeem_lookup_table_bundle(
    blockhash_provider: Arc<RecentBlockhashManager>,
    bundle_guard: &Pubkey,
    original_nonce: u64,
    funder: Arc<Keypair>,
    lut: Pubkey,
    tip: u64,
    operation_type: LutCallback,
) -> Bundle {
    let compute_price_ix =
        compute_budget::ComputeBudgetInstruction::set_compute_unit_price(1000000);

    let compute_limit_ix = compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(15000);

    let memo_ix = get_unique_memo_ix();

    //let transfer_ix = system_instruction::transfer(&sender.pubkey(), &receiver, amount_to_send);
    let tip_ix = system_instruction::transfer(&funder.pubkey(), &get_random_tip_account(), tip);

    let bundle_guard_ix =
        get_increment_bundle_guard_ix(funder.pubkey(), bundle_guard, original_nonce, None);

    let redeem_lut_ix = match operation_type {
        LutCallback::Deactivate => get_deactivate_lut_ix(Arc::clone(&funder), lut),
        LutCallback::Close => get_close_lut_ix(Arc::clone(&funder), lut),
    };

    let tx_ixs = vec![
        compute_price_ix,
        compute_limit_ix,
        memo_ix,
        bundle_guard_ix,
        redeem_lut_ix,
        tip_ix,
    ];

    let tx = VersionedTransaction::try_new(
        VersionedMessage::V0(
            Message::try_compile(
                &funder.pubkey(),
                &tx_ixs,
                &[],
                blockhash_provider.get_recent_blockhash(),
            )
            .unwrap(),
        ),
        &[Arc::clone(&funder)],
    );

    let tx = tx.unwrap();
    create_bundle(vec![tx])
}

pub fn get_classic_launch_bundle(
    mint_keypair: Arc<Keypair>,
    blockhash_provider: Arc<RecentBlockhashManager>,
    bonding_curve: &BondingCurve,
    wallets: &Vec<Arc<Keypair>>,
    amounts: &Vec<u64>,
    in_contact: bool,
    metadata_uri: &str,
    name: &str,
    symbol: &str,
    pump_keys: &PumpKeys,
    global_lut: Arc<AddressLookupTableAccount>,
    session_lut: Arc<AddressLookupTableAccount>,
    dev_wallet: Arc<Keypair>,
    dev_buy: Option<u64>,
    funder_wallet: Arc<Keypair>,
    tip: u64,
) -> Bundle {
    let mut all_txs: Vec<VersionedTransaction> = vec![];

    let mut creation_stage_ixs: Vec<Instruction> = vec![];
    //ok first of all here im gonna do the dev part, token creation and optional buy

    let compute_price_ix = compute_budget::ComputeBudgetInstruction::set_compute_unit_price(1_000);
    let compute_limit_ix =
        compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(250_000);
    let memo_ix = get_unique_memo_ix();

    let create_ix = get_create_pump_token_instruction(
        pump_keys,
        &dev_wallet.pubkey(),
        name,
        symbol,
        metadata_uri,
    );
    creation_stage_ixs.push(compute_price_ix);
    creation_stage_ixs.push(compute_limit_ix);
    creation_stage_ixs.push(memo_ix);
    creation_stage_ixs.push(create_ix);

    //info!("after base ixs");

    //if dev intends to buy
    if let Some(buy_amount) = dev_buy {
        creation_stage_ixs.push(create_associated_token_account_idempotent(
            &dev_wallet.pubkey(),
            &dev_wallet.pubkey(),
            &pump_keys.mint,
            &spl_token::ID,
        ));
        let dev_tokens_to_buy = calculate_pump_tokens_to_buy(
            buy_amount,
            bonding_curve.virtual_sol_reserves,
            bonding_curve.virtual_token_reserves,
            bonding_curve.real_token_reserves,
        );
        let dev_max_cost = buy_amount + (buy_amount as f64 * 0.5) as u64;

        creation_stage_ixs.push(get_buy_pump_token_instructions(
            pump_keys,
            &dev_wallet.pubkey(),
            dev_tokens_to_buy,
            dev_max_cost,
        ));
    }

    if wallets.is_empty() {
        creation_stage_ixs.push(system_instruction::transfer(
            &dev_wallet.pubkey(),
            &get_random_tip_account(),
            tip,
        ));
    }

    all_txs.push(
        VersionedTransaction::try_new(
            VersionedMessage::V0(
                Message::try_compile(
                    &dev_wallet.pubkey(),
                    &creation_stage_ixs,
                    &[],
                    blockhash_provider.get_recent_blockhash(),
                )
                .unwrap(),
            ),
            &[Arc::clone(&dev_wallet), Arc::clone(&mint_keypair)],
        )
        .unwrap(),
    );

    // First, let's calculate how many groups we need based on wallet count
    let wallets_per_tx = 5;
    let total_wallets = wallets.len();
    let needed_txs = (total_wallets + wallets_per_tx - 1) / wallets_per_tx;

    //info!("{total_wallets}, {wallets_per_group}, {group_count}");

    for group_idx in 0..needed_txs {
        let mut group_ixs: Vec<Instruction> = vec![];

        // Calculate start and end indices for this group
        let start_idx = group_idx * wallets_per_tx;
        let end_idx = std::cmp::min((group_idx + 1) * wallets_per_tx, total_wallets);

        // Add compute budget instructions for this group's transaction
        let compute_price_ix =
            compute_budget::ComputeBudgetInstruction::set_compute_unit_price(2000);
        let compute_limit_ix = compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            120_000 * (end_idx - start_idx) as u32,
        );
        group_ixs.push(compute_price_ix);
        group_ixs.push(compute_limit_ix);
        group_ixs.push(get_unique_memo_ix());

        //let is_first_group = group_idx == 0;
        let is_last_group = group_idx == needed_txs - 1;

        for wallet_idx in start_idx..end_idx {
            let in_contract_buy_ix = get_in_contract_pump_buy_instruction(
                &pump_keys,
                &funder_wallet.pubkey(),
                &wallets[wallet_idx].pubkey(),
                amounts[wallet_idx],
                in_contact,
            );
            group_ixs.push(in_contract_buy_ix);
        }

        // Add tip instruction for last group
        if is_last_group {
            //info!("locked tip account");
            let tip_ix = system_instruction::transfer(
                &funder_wallet.pubkey(),
                &get_random_tip_account(),
                tip,
            );
            group_ixs.push(tip_ix);
        }

        //create signers array here

        // Create and add the versioned transaction for this group
        let mut signers: Vec<Arc<Keypair>> = wallets[start_idx..end_idx]
            .iter()
            .map(|k| Arc::clone(k))
            .collect();
        signers.push(Arc::clone(&funder_wallet));

        //info!("{:#?}", &signers);

        all_txs.push(
            VersionedTransaction::try_new(
                VersionedMessage::V0(
                    Message::try_compile(
                        &funder_wallet.pubkey(),
                        &group_ixs,
                        &[session_lut.as_ref().clone(), global_lut.as_ref().clone()],
                        blockhash_provider.get_recent_blockhash(),
                    )
                    .unwrap(),
                ),
                &signers,
            )
            .unwrap(),
        );
    }

    //info!("{:#?}", all_txs.len());

    create_bundle(all_txs)
}

pub fn get_normal_multi_wallet_sell_bundle(
    pump_keys: Arc<PumpKeys>,
    blockhash_provider: Arc<RecentBlockhashManager>,
    chosen_wallets: &Vec<Arc<Keypair>>,
    amounts: &Vec<u64>,
    funder_keypair: Arc<Keypair>,
    bundle_guard: &Pubkey,
    original_nonce: u64,
    tip: u64,
    global_alu: Arc<AddressLookupTableAccount>,
) -> Bundle {
    let group_size = 5;
    let num_groups = cmp::min((chosen_wallets.len() + group_size - 1) / group_size, 4);
    let mut all_txs: Vec<VersionedTransaction> = vec![];

    for group_idx in 0..num_groups {
        let start_idx = group_idx * group_size;
        let end_idx = std::cmp::min((group_idx + 1) * group_size, chosen_wallets.len());
        let is_first_group = group_idx == 0;
        //let is_last_group = group_idx == num_groups - 1;

        let mut curr_ixs: Vec<Instruction> = vec![];
        let mut signers = vec![Arc::clone(&funder_keypair)];

        // Add compute budget instructions
        let compute_price_ix =
            compute_budget::ComputeBudgetInstruction::set_compute_unit_price(50000);
        let compute_limit_ix =
            compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(100_000);
        let memo_ix = get_unique_memo_ix();

        curr_ixs.extend_from_slice(&[compute_price_ix, compute_limit_ix, memo_ix]);

        // Add tip instruction for last group
        // Add bundle guard instruction and create receiver_ata for first group
        if is_first_group {
            let bundle_guard_ix = get_increment_bundle_guard_ix(
                funder_keypair.pubkey(),
                bundle_guard,
                original_nonce,
                None,
            );
            curr_ixs.push(bundle_guard_ix);

            //first I have to idempotently create the account for the funder so he can sell from in the last ix
            let create_ata_ix = create_associated_token_account_idempotent(
                &funder_keypair.pubkey(),
                &funder_keypair.pubkey(),
                &pump_keys.mint,
                &spl_token::ID,
            );
            curr_ixs.push(create_ata_ix);
        }

        // Process each wallet in the current group
        for idx in start_idx..end_idx {
            let chosen_wallet = &chosen_wallets[idx];
            let amount = amounts[idx];

            //now for each wallet im gonna transfer the tokens to the funder and close its ata
            let source_ata = get_associated_token_address(&chosen_wallet.pubkey(), &pump_keys.mint);
            let receiver_ata =
                get_associated_token_address(&funder_keypair.pubkey(), &pump_keys.mint);

            let transfer_ix = transfer(
                &spl_token::ID,
                &source_ata,
                &receiver_ata,
                &chosen_wallet.pubkey(),
                &[],
                amount,
            )
            .unwrap();

            //let close_ix = close_account(
            //    &spl_token::ID,
            //    &source_ata,
            //    &funder_keypair.pubkey(),
            //    &chosen_wallet.pubkey(),
            //    &[],
            //)
            //.unwrap();

            curr_ixs.extend_from_slice(&[transfer_ix]);
            signers.push(Arc::clone(&chosen_wallets[idx]));
        }

        //now create the transaction
        let tx = VersionedTransaction::try_new(
            VersionedMessage::V0(
                Message::try_compile(
                    &funder_keypair.pubkey(),
                    &curr_ixs,
                    &[(&*global_alu).clone()],
                    blockhash_provider.get_recent_blockhash(),
                )
                .unwrap(),
            ),
            &signers,
        );
        all_txs.push(tx.unwrap());
    }

    //this is the swap transaction
    let mut curr_ixs: Vec<Instruction> = vec![];
    let mut signers = vec![Arc::clone(&funder_keypair)];

    // Add compute budget instructions
    let compute_price_ix = compute_budget::ComputeBudgetInstruction::set_compute_unit_price(50000);
    let compute_limit_ix =
        compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(200_000);
    let memo_ix = get_unique_memo_ix();

    curr_ixs.extend_from_slice(&[compute_price_ix, compute_limit_ix, memo_ix]);
    let tip_ix =
        system_instruction::transfer(&funder_keypair.pubkey(), &get_random_tip_account(), tip);
    curr_ixs.push(tip_ix);

    //now before executing the swap, i need to see if the wallet count is 21,
    if chosen_wallets.len() == 21 {
        //if true then I will just simply add another transfer ix for it.
        let chosen_wallet = &chosen_wallets[20];
        let amount = amounts[20];
        signers.push(Arc::clone(chosen_wallet));
        let source_ata = get_associated_token_address(&chosen_wallet.pubkey(), &pump_keys.mint);
        let receiver_ata = get_associated_token_address(&funder_keypair.pubkey(), &pump_keys.mint);
        let transfer_ix = transfer(
            &spl_token::ID,
            &source_ata,
            &receiver_ata,
            &chosen_wallet.pubkey(),
            &[],
            amount,
        )
        .unwrap();
        curr_ixs.push(transfer_ix);
    }

    let funder_coin_ata = get_associated_token_address(&funder_keypair.pubkey(), &pump_keys.mint);
    let total_amount = amounts.iter().sum::<u64>();

    //here i just do the sell ix instead

    let final_sell_ix =
        get_sell_pump_token_instructions(&pump_keys, &funder_keypair.pubkey(), total_amount, 0);
    curr_ixs.push(final_sell_ix);

    let close_coin_ata_ix = close_account(
        &spl_token::ID,
        &funder_coin_ata,
        &funder_keypair.pubkey(),
        &funder_keypair.pubkey(),
        &[],
    )
    .unwrap();
    curr_ixs.push(close_coin_ata_ix);

    //now create the transaction
    let tx = VersionedTransaction::try_new(
        VersionedMessage::V0(
            Message::try_compile(
                &funder_keypair.pubkey(),
                &curr_ixs,
                &[(&*global_alu).clone()],
                blockhash_provider.get_recent_blockhash(),
            )
            .unwrap(),
        ),
        &signers,
    );
    all_txs.push(tx.unwrap());

    create_bundle(all_txs)
}

pub fn get_bonded_multi_wallet_sell_bundle(
    pump_amm_keys: Arc<PumpDexKeys>,
    blockhash_provider: Arc<RecentBlockhashManager>,
    chosen_wallets: &Vec<Arc<Keypair>>,
    amounts: &Vec<u64>,
    funder_keypair: Arc<Keypair>,
    bundle_guard: &Pubkey,
    original_nonce: u64,
    tip: u64,
    global_alu: Arc<AddressLookupTableAccount>,
) -> Bundle {
    let group_size = 5;
    let num_groups = cmp::min((chosen_wallets.len() + group_size - 1) / group_size, 4);
    let mut all_txs: Vec<VersionedTransaction> = vec![];

    for group_idx in 0..num_groups {
        let start_idx = group_idx * group_size;
        let end_idx = std::cmp::min((group_idx + 1) * group_size, chosen_wallets.len());
        let is_first_group = group_idx == 0;
        //let is_last_group = group_idx == num_groups - 1;

        let mut curr_ixs: Vec<Instruction> = vec![];
        let mut signers = vec![Arc::clone(&funder_keypair)];

        // Add compute budget instructions
        let compute_price_ix =
            compute_budget::ComputeBudgetInstruction::set_compute_unit_price(50000);
        let compute_limit_ix =
            compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(100_000);
        let memo_ix = get_unique_memo_ix();

        curr_ixs.extend_from_slice(&[compute_price_ix, compute_limit_ix, memo_ix]);

        // Add tip instruction for last group
        // Add bundle guard instruction and create receiver_ata for first group
        if is_first_group {
            let bundle_guard_ix = get_increment_bundle_guard_ix(
                funder_keypair.pubkey(),
                bundle_guard,
                original_nonce,
                None,
            );
            curr_ixs.push(bundle_guard_ix);

            //first I have to idempotently create the account for the funder so he can sell from in the last ix
            let create_ata_ix = create_associated_token_account_idempotent(
                &funder_keypair.pubkey(),
                &funder_keypair.pubkey(),
                &pump_amm_keys.base_mint,
                &spl_token::ID,
            );
            curr_ixs.push(create_ata_ix);
        }

        // Process each wallet in the current group
        for idx in start_idx..end_idx {
            let chosen_wallet = &chosen_wallets[idx];
            let amount = amounts[idx];

            //now for each wallet im gonna transfer the tokens to the funder and close its ata
            let source_ata =
                get_associated_token_address(&chosen_wallet.pubkey(), &pump_amm_keys.base_mint);
            let receiver_ata =
                get_associated_token_address(&funder_keypair.pubkey(), &pump_amm_keys.base_mint);

            let transfer_ix = transfer(
                &spl_token::ID,
                &source_ata,
                &receiver_ata,
                &chosen_wallet.pubkey(),
                &[],
                amount,
            )
            .unwrap();

            //let close_ix = close_account(
            //    &spl_token::ID,
            //    &source_ata,
            //    &funder_keypair.pubkey(),
            //    &chosen_wallet.pubkey(),
            //    &[],
            //)
            //.unwrap();

            curr_ixs.extend_from_slice(&[transfer_ix]);
            signers.push(Arc::clone(&chosen_wallets[idx]));
        }

        //now create the transaction
        let tx = VersionedTransaction::try_new(
            VersionedMessage::V0(
                Message::try_compile(
                    &funder_keypair.pubkey(),
                    &curr_ixs,
                    &[(&*global_alu).clone()],
                    blockhash_provider.get_recent_blockhash(),
                )
                .unwrap(),
            ),
            &signers,
        );
        all_txs.push(tx.unwrap());
    }

    //this is the swap transaction
    let mut curr_ixs: Vec<Instruction> = vec![];
    let mut signers = vec![Arc::clone(&funder_keypair)];

    // Add compute budget instructions
    let compute_price_ix = compute_budget::ComputeBudgetInstruction::set_compute_unit_price(50000);
    let compute_limit_ix =
        compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(200_000);
    let memo_ix = get_unique_memo_ix();

    curr_ixs.extend_from_slice(&[compute_price_ix, compute_limit_ix, memo_ix]);
    let tip_ix =
        system_instruction::transfer(&funder_keypair.pubkey(), &get_random_tip_account(), tip);
    curr_ixs.push(tip_ix);

    //now before executing the swap, i need to see if the wallet count is 21,
    if chosen_wallets.len() == 21 {
        //if true then I will just simply add another transfer ix for it.
        let chosen_wallet = &chosen_wallets[20];
        let amount = amounts[20];
        signers.push(Arc::clone(chosen_wallet));
        let source_ata =
            get_associated_token_address(&chosen_wallet.pubkey(), &pump_amm_keys.base_mint);
        let receiver_ata =
            get_associated_token_address(&funder_keypair.pubkey(), &pump_amm_keys.base_mint);
        let transfer_ix = transfer(
            &spl_token::ID,
            &source_ata,
            &receiver_ata,
            &chosen_wallet.pubkey(),
            &[],
            amount,
        )
        .unwrap();
        curr_ixs.push(transfer_ix);
    }

    let funder_coin_ata =
        get_associated_token_address(&funder_keypair.pubkey(), &pump_amm_keys.base_mint);
    let funder_wsol_ata =
        get_associated_token_address(&funder_keypair.pubkey(), &pump_amm_keys.quote_mint);

    let create_wsol_ata_ix = create_associated_token_account_idempotent(
        &funder_keypair.pubkey(),
        &funder_keypair.pubkey(),
        &native_mint::ID,
        &spl_token::ID,
    );
    curr_ixs.push(create_wsol_ata_ix);
    let total_amount = amounts.iter().sum::<u64>();

    let swap_ix = create_pf_amm_sell_instruction(
        Arc::clone(&funder_keypair),
        Arc::clone(&pump_amm_keys),
        total_amount,
        0,
    );
    curr_ixs.push(swap_ix);

    let close_wsol_ata_ix = close_account(
        &spl_token::ID,
        &funder_wsol_ata,
        &funder_keypair.pubkey(),
        &funder_keypair.pubkey(),
        &[],
    )
    .unwrap();
    curr_ixs.push(close_wsol_ata_ix);

    let close_coin_ata_ix = close_account(
        &spl_token::ID,
        &funder_coin_ata,
        &funder_keypair.pubkey(),
        &funder_keypair.pubkey(),
        &[],
    )
    .unwrap();
    curr_ixs.push(close_coin_ata_ix);

    //now create the transaction
    let tx = VersionedTransaction::try_new(
        VersionedMessage::V0(
            Message::try_compile(
                &funder_keypair.pubkey(),
                &curr_ixs,
                &[(&*global_alu).clone()],
                blockhash_provider.get_recent_blockhash(),
            )
            .unwrap(),
        ),
        &signers,
    );
    all_txs.push(tx.unwrap());

    create_bundle(all_txs)
}