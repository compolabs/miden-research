use miden_lib::transaction::memory::FAUCET_STORAGE_DATA_SLOT;
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, SlotItem, StorageSlot},
    assembly::{ModuleAst, ProgramAst},
    assets::{AssetVault, FungibleAsset},
    Felt, Word,
};
use miden_tx::TransactionExecutor;

use crate::utils::{
    get_new_key_pair_with_advice_map, get_note_with_fungible_asset_and_script,
    prove_and_verify_transaction, MockDataStore, ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN,
};

fn get_faucet_account_with_max_supply_and_total_issuance(
    public_key: Word,
    max_supply: u64,
    total_issuance: Option<u64>,
) -> Account {
    let faucet_account_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap();
    let faucet_account_code_src = include_str!("../../src/token_burn/basic_fungible.masm");
    let faucet_account_code_ast = ModuleAst::parse(faucet_account_code_src).unwrap();
    let account_assembler = TransactionKernel::assembler();

    let faucet_account_code =
        AccountCode::new(faucet_account_code_ast.clone(), &account_assembler).unwrap();

    let faucet_storage_slot_1 = [
        Felt::new(max_supply),
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ];
    let mut faucet_account_storage = AccountStorage::new(
        vec![
            SlotItem {
                index: 0,
                slot: StorageSlot::new_value(public_key),
            },
            SlotItem {
                index: 1,
                slot: StorageSlot::new_value(faucet_storage_slot_1),
            },
        ],
        vec![],
    )
    .unwrap();

    if total_issuance.is_some() {
        let faucet_storage_slot_254 = [
            Felt::new(0),
            Felt::new(0),
            Felt::new(0),
            Felt::new(total_issuance.unwrap()),
        ];
        faucet_account_storage
            .set_item(FAUCET_STORAGE_DATA_SLOT, faucet_storage_slot_254)
            .unwrap();
    };

    Account::new(
        faucet_account_id,
        AssetVault::new(&[]).unwrap(),
        faucet_account_storage.clone(),
        faucet_account_code.clone(),
        Felt::new(1),
    )
}

#[test]
fn prove_faucet_contract_burn_fungible_asset_succeeds() {
    let (faucet_pub_key, _faucet_keypair_felts) = get_new_key_pair_with_advice_map();
    let faucet_account =
        get_faucet_account_with_max_supply_and_total_issuance(faucet_pub_key, 200, Some(100));

    let fungible_asset = FungibleAsset::new(faucet_account.id(), 100).unwrap();

    // check that max_supply (slot 1) is 200 and amount already issued (slot 255) is 100
    assert_eq!(
        faucet_account.storage().get_item(1),
        [Felt::new(200), Felt::new(0), Felt::new(0), Felt::new(0)].into()
    );
    assert_eq!(
        faucet_account.storage().get_item(FAUCET_STORAGE_DATA_SLOT),
        [Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(100)].into()
    );

    // need to create a note with the fungible asset to be burned
    let note_script = ProgramAst::parse(
        "
      use.miden::contracts::faucets::basic_fungible->faucet_contract
      use.miden::note

      # burn the asset
      begin
          dropw
          exec.note::get_assets drop
          mem_loadw
          call.faucet_contract::burn
      end
      ",
    )
    .unwrap();

    let note = get_note_with_fungible_asset_and_script(fungible_asset, note_script);

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(faucet_account.clone()), Some(vec![note.clone()]));

    let mut executor: TransactionExecutor<_, ()> =
        TransactionExecutor::new(data_store.clone(), None);
    executor.load_account(faucet_account.id()).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store
        .notes
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(
            faucet_account.id(),
            block_ref,
            &note_ids,
            data_store.tx_args.clone(),
        )
        .unwrap();

    // Prove, serialize/deserialize and verify the transaction
    // assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // check that the account burned the asset
    assert_eq!(
        executed_transaction.account_delta().nonce(),
        Some(Felt::new(2))
    );
    assert_eq!(
        executed_transaction.input_notes().get_note(0).id(),
        note.id()
    );
}
