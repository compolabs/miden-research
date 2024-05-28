use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, SlotItem, StorageSlot},
    assembly::{AssemblyContext, ModuleAst, ProgramAst},
    assets::{Asset, AssetVault},
    crypto::rand::{FeltRng, RpoRandomCoin},
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteHeader, NoteInputs, NoteMetadata, NoteRecipient,
        NoteScript, NoteTag, NoteType,
    },
    transaction::TransactionArgs,
    vm::CodeBlock,
    Felt, NoteError, Word, ZERO,
};
use miden_processor::AdviceMap;
use miden_tx::TransactionExecutor;
use miden_vm::Assembler;

use std::fs;
use std::path::Path;

use crate::utils::{
    get_new_key_pair_with_advice_map, prove_and_verify_transaction, MockDataStore,
    ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN, ACCOUNT_ID_SENDER,
};

const MASTS: [&str; 2] = [
    "0x274fb08a1e8ec345beeca765ad44186a8b2bfcdfcdc2b08582ff6ef774789787", // do_calculation_output_note
    "0x4916cf24b0739eb6ff870a88c8d3903ace255598d8647e105f280226fa1dcc64", // consume_note
];

const ACCOUNT_CODE: &str =
    include_str!("../../src/verifiable_computation/note_creator_consumer.masm");

pub fn account_code(assembler: &Assembler) -> AccountCode {
    let account_module_ast = ModuleAst::parse(ACCOUNT_CODE).unwrap();
    let code = AccountCode::new(account_module_ast, assembler).unwrap();

    let current: [String; 2] = [code.procedures()[0].to_hex(), code.procedures()[1].to_hex()];

    assert!(current == MASTS, "UPDATE MAST ROOT: {:?};", current);

    code
}

pub fn get_account_with_custom_proc(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);

    let account_code = account_code(&assembler);
    let account_storage = AccountStorage::new(
        vec![SlotItem {
            index: 0,
            slot: StorageSlot::new_value(public_key),
        }],
        vec![],
    )
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

fn create_initial_message_note<R: FeltRng>(
    sender_account_id: AccountId,
    target_account_id: AccountId,
    create_note_a: bool,
    mut rng: R,
) -> Result<Note, NoteError> {
    let note_script = include_str!("../../src/verifiable_computation/message_note.masm");

    let note_assembler = TransactionKernel::assembler().with_debug_mode(true);

    let script_ast = ProgramAst::parse(&note_script).unwrap();
    let (note_script, _) = new_note_script(script_ast, &note_assembler).unwrap();

    // create note a (1) or note b (0)
    let create_note_a_u64: u64 = if create_note_a { 1 } else { 0 };

    println!("create_note_a_u64: {:?}", create_note_a_u64);

    let inputs = NoteInputs::new(vec![Felt::new(create_note_a_u64)])?;

    let tag = NoteTag::from_account_id(target_account_id, NoteExecutionHint::Local)?;
    let serial_num = rng.draw_word();
    let aux = ZERO;
    let note_type = NoteType::OffChain;
    let metadata = NoteMetadata::new(sender_account_id, note_type, tag, aux)?;

    // empty vault
    let vault = NoteAssets::new(vec![])?;

    let recipient = NoteRecipient::new(serial_num, note_script, inputs);

    Ok(Note::new(vault, metadata, recipient))
}

pub fn create_output_note(note_input: Option<Felt>, use_note_a: bool) -> Result<Note, NoteError> {
    let sender_account_id: AccountId =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();

    // Create target smart contract
    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();

    let note_assembler = TransactionKernel::assembler().with_debug_mode(true);

    let note_script_path = if use_note_a {
        "src/verifiable_computation/output_consumable_note.masm"
    } else {
        "src/verifiable_computation/output_consumable_note.masm"
    };

    let note_script = fs::read_to_string(Path::new(note_script_path))
        .expect("Failed to read the note script file");

    let script_ast = ProgramAst::parse(&note_script).unwrap();
    let (note_script, _) = new_note_script(script_ast, &note_assembler).unwrap();

    // add the inputs to the note
    let input_values = match note_input {
        Some(value) => vec![value],
        None => vec![],
    };

    let inputs: NoteInputs = NoteInputs::new(input_values).unwrap();

    let tag = NoteTag::from_account_id(target_account_id, NoteExecutionHint::Local).unwrap();
    let serial_num = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let aux = ZERO;
    let note_type = NoteType::OffChain;
    let metadata = NoteMetadata::new(sender_account_id, note_type, tag, aux).unwrap();

    // empty vault
    let vault: NoteAssets = NoteAssets::new(vec![]).unwrap();
    let recipient = NoteRecipient::new(serial_num, note_script, inputs);

    Ok(Note::new(vault, metadata, recipient))
}

// Run this first to check MASTs are correct
#[test]
pub fn check_account_masts() {
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);

    let account_module_ast = ModuleAst::parse(ACCOUNT_CODE).unwrap();
    let code = AccountCode::new(account_module_ast, &assembler).unwrap();

    let current: [String; 2] = [code.procedures()[0].to_hex(), code.procedures()[1].to_hex()];
    println!("{:?}", current);
    assert!(current == MASTS, "UPDATE MAST ROOT: {:?};", current);
}

#[test]
pub fn get_dynamic_note_recipient() {
    let output_note = create_output_note(None, false).unwrap();

    let tag = output_note.clone().metadata().tag();
    let note_type = output_note.clone().metadata().note_type();
    let recipient = output_note.recipient();

    println!("{:?}", tag);
    println!("{:?}", note_type);
    println!("{:?}", recipient.digest());
}

#[test]
fn test_note_output() {
    // Create note sender account
    let sender_account_id: AccountId = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // Create target smart contract
    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();
    let (target_pub_key, target_sk_pk_felt) = get_new_key_pair_with_advice_map();
    let target_account = get_account_with_custom_proc(target_account_id, target_pub_key, None);

    // Create note a (1) or note b (0)
    let create_note_a: bool = true;

    // Create the note
    let note = create_initial_message_note(
        sender_account_id,
        target_account_id,
        create_note_a,
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]));

    let mut executor: TransactionExecutor<_, ()> =
        TransactionExecutor::new(data_store.clone(), None).with_debug_mode(true);
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store
        .notes
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let tx_script_code = include_str!("../../src/note_output/tx_script.masm");
    let tx_script_ast = ProgramAst::parse(tx_script_code).unwrap();

    let tx_script_target = executor
        .compile_tx_script(
            tx_script_ast.clone(),
            vec![(target_pub_key, target_sk_pk_felt.clone())],
            vec![],
        )
        .unwrap();

    let tx_args_target = TransactionArgs::new(Some(tx_script_target), None, AdviceMap::default());

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target.clone())
        .unwrap();

    // Note outputted by the transaction
    let tx_output_note = executed_transaction.output_notes().get_note(0);

    println!("{:?}", tx_output_note.metadata());
    // let stack_output = executed_transaction.clone().stack_output();

    // Note expected to be outputted by the transaction
    // let expected_note = create_output_note(None, create_note_a).unwrap();
         
    // Check that the output note is the same as the expected note
/*     assert_eq!(
        NoteHeader::from(tx_output_note).metadata(),
        NoteHeader::from(expected_note.clone()).metadata()
    );
    assert_eq!(
        NoteHeader::from(tx_output_note),
        NoteHeader::from(expected_note.clone())
    );  */
    
    // assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // CONSTRUCT AND EXECUTE TX 2 (Success)
    // --------------------------------------------------------------------------------------------
    let note_ids_1 = data_store
    .notes
    .iter()
    .map(|tx_output_note| tx_output_note.id())
    .collect::<Vec<_>>();


    // Execute the transaction and get the witness
    let executed_transaction_1 = executor
        .execute_transaction(target_account_id, block_ref, &note_ids_1, tx_args_target)
        .unwrap();

}
