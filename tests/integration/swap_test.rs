use crate::utils::{
    get_account_with_default_account_code, get_new_key_pair_with_advice_map,
    prove_and_verify_transaction, MockDataStore,
};
use miden_lib::{
    notes::{create_swap_note, utils::build_p2id_recipient},
    transaction::TransactionKernel,
};
use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, SlotItem, StorageSlot,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN, ACCOUNT_ID_SENDER,
    },
    assembly::{AssemblyContext, ModuleAst, ProgramAst},
    assets::{Asset, AssetVault, FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails},
    crypto::rand::{FeltRng, RpoRandomCoin},
    notes::{
        Note, NoteAssets, NoteEnvelope, NoteExecutionMode, NoteId, NoteInputs, NoteMetadata,
        NoteRecipient, NoteScript, NoteTag, NoteType,
    },
    transaction::TransactionArgs,
    Felt, NoteError, Word, ZERO,
};
use miden_tx::TransactionExecutor;
use miden_vm::Assembler;
use mock::mock::account::DEFAULT_AUTH_SCRIPT;

/* pub fn build_note_script(bytes: &[u8]) -> Result<NoteScript, NoteError> {
  let note_assembler = TransactionKernel::assembler().with_debug_mode(true);

  let script_ast = ProgramAst::from_bytes(bytes).map_err(NoteError::NoteDeserializationError)?;
  let (note_script, _) = NoteScript::new(script_ast, &note_assembler)?;

  Ok(note_script)
} */

pub fn get_custom_account_code(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    let account_code_src = include_str!("../../src/swap/swap_account.masm");
    let account_code_ast = ModuleAst::parse(account_code_src).unwrap();
    let account_assembler = TransactionKernel::assembler().with_debug_mode(true);

    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler).unwrap();
    let account_storage = AccountStorage::new(vec![SlotItem {
        index: 0,
        slot: StorageSlot::new_value(public_key),
    }])
    .unwrap();

    let account_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::new(
        account_id,
        account_vault,
        account_storage,
        account_code,
        Felt::new(1),
    )
}

pub fn new_note_script(code: ProgramAst, assembler: &Assembler) -> Result<NoteScript, NoteError> {
    // Compile the code in the context with phantom calls enabled
    let code_block = assembler
        .compile_in_context(
            &code,
            &mut AssemblyContext::for_program(Some(&code)).with_phantom_calls(true),
        )
        .map_err(NoteError::ScriptCompilationError)?;

    // Use the from_parts method to create a NoteScript instance
    let note_script = NoteScript::from_parts(code, code_block.hash());

    Ok(note_script)
}
/// Generates a SWAP note - swap of assets between two accounts.
///
/// This script enables a swap of 2 assets between the `sender` account and any other account that
/// is willing to consume the note. The consumer will receive the `offered_asset` and will create a
/// new P2ID note with `sender` as target, containing the `requested_asset`.
///
/// # Errors
/// Returns an error if deserialization or compilation of the `SWAP` script fails.
pub fn create_custom_swap_note<R: FeltRng>(
    sender: AccountId,
    offered_asset: Asset,
    requested_asset: Asset,
    note_type: NoteType,
    mut rng: R,
) -> Result<(Note, Word), NoteError> {
    // let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/SWAP.masb"));
    // let note_script = build_note_script(bytes)?;

    let note_code = include_str!("../../src/swap/SWAP.masm");
    let note_script = new_note_script(
        ProgramAst::parse(note_code).unwrap(),
        &TransactionKernel::assembler().with_debug_mode(true),
    )
    .unwrap();

    let payback_serial_num = rng.draw_word();
    let payback_recipient = build_p2id_recipient(sender, payback_serial_num)?;
    let asset_word: Word = requested_asset.into();
    let payback_tag = NoteTag::from_account_id(sender, NoteExecutionMode::Local)?;

    let inputs = NoteInputs::new(vec![
        payback_recipient[0],
        payback_recipient[1],
        payback_recipient[2],
        payback_recipient[3],
        asset_word[0],
        asset_word[1],
        asset_word[2],
        asset_word[3],
        payback_tag.inner().into(),
    ])?;

    // TODO: build the tag for the SWAP use case
    let tag = 0.into();
    let serial_num = rng.draw_word();
    let aux = ZERO;

    let metadata = NoteMetadata::new(sender, note_type, tag, aux)?;
    let vault = NoteAssets::new(vec![offered_asset])?;
    let recipient = NoteRecipient::new(serial_num, note_script, inputs);
    let note = Note::new(vault, metadata, recipient);

    Ok((note, payback_serial_num))
}

#[test]
fn prove_swap_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let non_fungible_asset: Asset = NonFungibleAsset::new(
        &NonFungibleAssetDetails::new(faucet_id_2, vec![1, 2, 3, 4]).unwrap(),
    )
    .unwrap()
    .into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN).unwrap();
    let (target_pub_key, target_sk_felt) = get_new_key_pair_with_advice_map();
    let target_account =
        get_custom_account_code(target_account_id, target_pub_key, Some(non_fungible_asset));

    // Create the note containing the SWAP script
    let (note, repay_serial_num) = create_custom_swap_note(
        sender_account_id,
        fungible_asset,
        non_fungible_asset,
        NoteType::Public,
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone()).with_debug_mode(true);
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store
        .notes
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(DEFAULT_AUTH_SCRIPT).unwrap();
    let tx_script_target = executor
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(target_pub_key, target_sk_felt)],
            vec![],
        )
        .unwrap();
    let tx_args_target = TransactionArgs::with_tx_script(tx_script_target);

    let executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target)
        .expect("Transaction consuming swap note failed");

    // Prove, serialize/deserialize and verify the transaction
    //  assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // target account vault delta
    let target_account_after: Account = Account::new(
        target_account.id(),
        AssetVault::new(&[fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );

    // Check that the target account has received the asset from the note
    assert_eq!(
        executed_transaction.final_account().hash(),
        target_account_after.hash()
    );

    // Check if only one `Note` has been created
    assert_eq!(executed_transaction.output_notes().num_notes(), 1);

    // Check if the created `Note` is what we expect
    let recipient = build_p2id_recipient(sender_account_id, repay_serial_num).unwrap();
    let tag = NoteTag::from_account_id(sender_account_id, NoteExecutionMode::Local).unwrap();
    let note_metadata =
        NoteMetadata::new(target_account_id, NoteType::OffChain, tag, ZERO).unwrap();
    let assets = NoteAssets::new(vec![non_fungible_asset]).unwrap();
    let note_id = NoteId::new(recipient, assets.commitment());

    let created_note = executed_transaction.output_notes().get_note(0);
    assert_eq!(
        NoteEnvelope::from(created_note),
        NoteEnvelope::new(note_id, note_metadata).unwrap()
    );
}
