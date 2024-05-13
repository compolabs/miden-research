use miden_objects::accounts::ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN;
use miden_objects::{Felt, Word, ONE};
use miden_processor::ProcessState;
use mock::{
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction, run_tx,
};

use std::fs;

#[test]
fn test_tx_create_fungible_asset() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::FungibleFaucet {
            acct_id: ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );

    let assembly_code = include_str!("../../src/asset/create_asset.masm");

    let transaction = prepare_transaction(tx_inputs, tx_args, &assembly_code, None);
    let process = run_tx(&transaction).unwrap();

    assert_eq!(
        process.get_stack_word(0),
        Word::from([
            Felt::new(1000),
            Felt::new(0),
            Felt::new(0),
            Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN)
        ])
    );
    println!("{:?}", process.get_stack_word(0));
}

#[test]
fn test_tx_create_fungible_asset_and_send() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::FungibleFaucet {
            acct_id: ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );

    let assembly_code = include_str!("../../src/asset/create_asset.masm");

    let transaction = prepare_transaction(tx_inputs, tx_args, &assembly_code, None);
    let process = run_tx(&transaction).unwrap();

    assert_eq!(
        process.get_stack_word(0),
        Word::from([
            Felt::new(1000),
            Felt::new(0),
            Felt::new(0),
            Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN)
        ])
    );
    println!("{:?}", process.get_stack_word(0));
}
