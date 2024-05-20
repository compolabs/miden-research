// use miden_vm::{prove, verify, Assembler, DefaultHost, ProvingOptions, StackInputs};

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountId},
    assembly::{AssemblyContext, ProgramAst},
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::rand::{FeltRng, RpoRandomCoin},
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteInputs, NoteMetadata, NoteRecipient, NoteScript,
        NoteTag, NoteType,
    },
    transaction::TransactionArgs,
    vm::CodeBlock,
    Felt, NoteError, ZERO,
};
use miden_tx::TransactionExecutor;
use miden_vm::Assembler;
use mock::mock::account::DEFAULT_AUTH_SCRIPT;

use crate::utils::{
    get_account_with_default_account_code, get_new_pk_and_authenticator, MockDataStore,
    ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
    ACCOUNT_ID_SENDER,
};

pub fn new_note_script(
    code: ProgramAst,
    assembler: &Assembler,
) -> Result<(NoteScript, CodeBlock), NoteError> {
    // Compile the code in the context with phantom calls enabled
    let code_block = assembler
        .compile_in_context(
            &code,
            &mut AssemblyContext::for_program(Some(&code)).with_phantom_calls(true),
        )
        .map_err(NoteError::ScriptCompilationError)?;

    // Use the from_parts method to create a NoteScript instance
    let note_script = NoteScript::from_parts(code, code_block.hash());

    Ok((note_script, code_block))
}

pub fn create_p2id_note<R: FeltRng>(
    sender: AccountId,
    target: AccountId,
    assets: Vec<Asset>,
    note_type: NoteType,
    mut rng: R,
) -> Result<Note, NoteError> {
    let note_script = include_str!("../../src/p2id/P2ID.masm");
    let note_assembler = TransactionKernel::assembler().with_debug_mode(true);

    let script_ast = ProgramAst::parse(&note_script).unwrap();
    let (note_script, _) = new_note_script(script_ast, &note_assembler).unwrap();

    let inputs = NoteInputs::new(vec![target.into()])?;
    let tag = NoteTag::from_account_id(target, NoteExecutionHint::Local)?;
    let serial_num = rng.draw_word();
    let aux = ZERO;

    let metadata = NoteMetadata::new(sender, note_type, tag, aux)?;
    let vault = NoteAssets::new(assets)?;
    let recipient = NoteRecipient::new(serial_num, note_script, inputs);
    Ok(Note::new(vault, metadata, recipient))
}

// P2ID TESTS
// ===============================================================================================
// We test the Pay to ID script. So we create a note that can only be consumed by the target
// account.
#[test]
fn prove_p2id_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN).unwrap();
    let (target_pub_key, falcon_auth) = get_new_pk_and_authenticator();

    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key, None);

    // Create the note
    let note = create_p2id_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset],
        NoteType::Public,
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone(), Some(falcon_auth.clone()));
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store
        .notes
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let tx_script = include_str!("../../src/p2id/tx_script.masm");
    let tx_script_code = ProgramAst::parse(tx_script).unwrap();

    let tx_script_target = executor
        .compile_tx_script(tx_script_code.clone(), vec![], vec![])
        .unwrap();
    let tx_args_target = TransactionArgs::with_tx_script(tx_script_target);

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target)
        .unwrap();

    // Prove, serialize/deserialize and verify the transaction
    // assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // vault delta
    let target_account_after: Account = Account::new(
        target_account.id(),
        AssetVault::new(&[fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );
    assert_eq!(
        executed_transaction.final_account().hash(),
        target_account_after.hash()
    );
}

// A script can be converted into a vector of Felt values and back
#[test]
fn test_note_script_to_from_felt() {
    let assembler = TransactionKernel::assembler();

    let note_program_ast = ProgramAst::parse("begin push.1 drop end").unwrap();
    let (note_script, _) = NoteScript::new(note_program_ast, &assembler).unwrap();

    let encoded: Vec<Felt> = (&note_script).into();

    println!("{:?}", encoded);

    let decoded: NoteScript = encoded.try_into().unwrap();

    assert_eq!(note_script, decoded);
}
