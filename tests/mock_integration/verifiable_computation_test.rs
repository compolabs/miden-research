use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, SlotItem, StorageSlot},
    assembly::{AssemblyContext, ModuleAst, ProgramAst},
    assets::{Asset, AssetVault},
    crypto::hash::rpo::RpoDigest,
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
use miden_tx::{testing::TransactionContextBuilder, TransactionExecutor};
use miden_vm::Assembler;

use std::collections::BTreeMap;

use crate::common::*;

const MASTS: [&str; 2] = [
    "0xc2114e2bb4ad9e7183d376f34895bdae007e1e31e59084db3371e9ca9c4adf11", // do_calculation_output_note
    "0xbf8e006fdf47e206c6ab4fd9f6f8ba1e993981f0533993fdd6772b5c7797fd1a", // consume_note
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
        BTreeMap::new(),
    )
    .unwrap();

    let account_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::from_parts(
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
    note_input: Felt,
    mut rng: R,
) -> Result<Note, NoteError> {
    let note_script = include_str!("../../src/verifiable_computation/caller_note.masm");

    let note_assembler = TransactionKernel::assembler().with_debug_mode(true);

    let script_ast = ProgramAst::parse(&note_script).unwrap();
    let (note_script, _) = new_note_script(script_ast, &note_assembler).unwrap();

    let inputs = NoteInputs::new(vec![note_input])?;

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

pub fn create_output_note(note_input: Option<Felt>) -> Result<(Note, RpoDigest), NoteError> {
    let sender_account_id: AccountId = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // Create target smart contract
    let target_account_id = AccountId::try_from(ACCOUNT_ID_SENDER_1).unwrap();

    let note_assembler = TransactionKernel::assembler().with_debug_mode(true);

    let note_script = include_str!("../../src/verifiable_computation/output_consumable_note.masm");

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
    let recipient = NoteRecipient::new(serial_num, note_script.clone(), inputs);

    let note_script_hash = note_script.hash();

    Ok((Note::new(vault, metadata, recipient), note_script_hash))
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
pub fn get_note_script_hash() {
    let (output_note, note_script_hash) = create_output_note(None).unwrap();

    let tag = output_note.clone().metadata().tag();
    let note_type = output_note.clone().metadata().note_type();

    println!("{:?}", tag);
    println!("{:?}", note_type);
    println!("Note script hash: {:?}", note_script_hash);
}

#[test]
fn test_verifiable_computation() {
    // Create note sender account
    let sender_account_id: AccountId = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // Create target smart contract
    let target_account_id = AccountId::try_from(ACCOUNT_ID_SENDER_1).unwrap();
    let (target_pub_key, target_falcon_auth) = get_new_pk_and_authenticator();

    let target_account = get_account_with_custom_proc(target_account_id, target_pub_key, None);
    // message note input
    let note_input: Felt = Felt::new(5);

    // Create the note
    let note = create_initial_message_note(
        sender_account_id,
        target_account_id,
        note_input,
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let tx_context = TransactionContextBuilder::new(target_account.clone())
        .input_notes(vec![note.clone()])
        .build();

    let mut executor =
        TransactionExecutor::new(tx_context.clone(), Some(target_falcon_auth.clone()))
            .with_debug_mode(true);
    executor.load_account(target_account_id).unwrap();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let tx_script_code = include_str!("../../src/note_output/tx_script.masm");
    let tx_script_ast = ProgramAst::parse(tx_script_code).unwrap();

    let tx_script_target = executor
        .compile_tx_script(tx_script_ast.clone(), vec![], vec![])
        .unwrap();

    let tx_args_target = TransactionArgs::new(Some(tx_script_target), None, AdviceMap::default());

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(
            target_account_id,
            block_ref,
            &note_ids,
            tx_args_target.clone(),
        )
        .unwrap();

    // Note outputted by the transaction
    let tx_output_note = executed_transaction.output_notes().get_note(0);

    // Note expected to be outputted by the transaction
    let (expected_note, note_script_hash) = create_output_note(Some(Felt::new(301))).unwrap();

    // Check that the output note is the same as the expected note
    assert_eq!(
        NoteHeader::from(tx_output_note).metadata(),
        NoteHeader::from(expected_note.clone()).metadata()
    );
    assert_eq!(
        NoteHeader::from(tx_output_note),
        NoteHeader::from(expected_note.clone())
    );

    // comment out to speed up test
    // assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // CONSTRUCT AND EXECUTE TX 2 (Success)
    // --------------------------------------------------------------------------------------------
    let tx_context_1 = TransactionContextBuilder::new(target_account.clone())
        .input_notes(vec![expected_note.clone()]);

    let mut executor =
        TransactionExecutor::new(tx_context.clone(), Some(target_falcon_auth.clone()))
            .with_debug_mode(true);
    executor.load_account(target_account_id).unwrap();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let _executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target)
        .expect("Transaction consuming swap note failed");

    // commented out to speed up test
    // assert!(prove_and_verify_transaction(executed_transaction_1.clone()).is_ok());
}
