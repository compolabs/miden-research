use miden_objects::{crypto::merkle::LeafIndex, Felt, Word};
use miden_processor::ProcessState;
use mock::{
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction, run_tx,
};

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, SlotItem, StorageSlot},
    accounts::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN, ACCOUNT_ID_SENDER,
    },
    assembly::{ModuleAst, ProgramAst},
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::rand::{FeltRng, RpoRandomCoin},
    notes::{
        Note, NoteAssets, NoteExecutionMode, NoteInputs, NoteMetadata, NoteRecipient, NoteScript,
        NoteTag, NoteType,
    },
    transaction::TransactionArgs,
    NoteError, ONE, ZERO,
};
use std::fs;
use utils::{get_new_key_pair_with_advice_map, MockDataStore};
mod utils;

#[test]
fn test_set_item() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    // copy the initial account slots (SMT)
    let mut account_smt = tx_inputs.account().storage().slots().clone();
    let init_root = account_smt.root();

    // insert a new leaf value
    let new_item_index = LeafIndex::new(12).unwrap();
    let new_item_value: Word = [Felt::new(91), Felt::new(92), Felt::new(93), Felt::new(94)];
    account_smt.insert(new_item_index, new_item_value);
    assert_ne!(account_smt.root(), init_root);

    let assembly_code: &str = include_str!("../src/masm/set_item.masm");

    let transaction = prepare_transaction(tx_inputs, tx_args, &assembly_code, None);
    let _process = run_tx(&transaction).unwrap();

    println!("{:?}", _process.get_stack_state());
}
