use miden_objects::accounts::{
    ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
};
use miden_objects::{
    Felt, NoteError, Word, ONE, ZERO,
};
use miden_processor::ProcessState;
use mock::{
    constants::{non_fungible_asset, FUNGIBLE_ASSET_AMOUNT, NON_FUNGIBLE_ASSET_DATA},
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction,
    procedures::prepare_word,
    run_tx,
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

    let filename = "./src/masm/create_asset.masm";
    let assembly_code = fs::read_to_string(filename).expect("Failed to read the assembly file");

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

    let filename = "./src/masm/create_asset.masm";
    let assembly_code = fs::read_to_string(filename).expect("Failed to read the assembly file");

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
