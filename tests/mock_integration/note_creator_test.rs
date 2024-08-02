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
use miden_tx::{testing::TransactionContextBuilder, TransactionExecutor};
use miden_vm::Assembler;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::common::*;

const MASTS: [&str; 3] = [
    "0x1e3c3fc738ec66e45db871d25d8d5b952b7a13cca4c9d5c803681316aa711cf6", // create note
    "0x25781a9e6af348eeddd49e99e42dd169de60c061954bcb7a8e182f2a7ce9c8fe", // note_a_receiver
    "0xea857a80b77ae53d702de6456eeb68de9eb9318a57e33e4747b199f48e58e285", // note_b_receiver
];

const ACCOUNT_CODE: &str = include_str!("../../src/note_output/note_creator.masm");

pub fn account_code(assembler: &Assembler) -> AccountCode {
    let account_module_ast = ModuleAst::parse(ACCOUNT_CODE).unwrap();
    let code = AccountCode::new(account_module_ast, assembler).unwrap();

    let current: [String; 3] = [
        code.procedures()[0].to_hex(),
        code.procedures()[1].to_hex(),
        code.procedures()[2].to_hex(),
    ];

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
    create_note_a: bool,
    mut rng: R,
) -> Result<Note, NoteError> {
    let note_script = include_str!("../../src/note_output/message_note.masm");

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
    let sender_account_id: AccountId = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // Create target smart contract
    let target_account_id = AccountId::try_from(ACCOUNT_ID_SENDER_1).unwrap();

    let note_assembler = TransactionKernel::assembler().with_debug_mode(true);

    let note_script_path = if use_note_a {
        "src/note_output/output_note_a.masm"
    } else {
        "src/note_output/output_note_b.masm"
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

    let current: [String; 3] = [
        code.procedures()[0].to_hex(),
        code.procedures()[1].to_hex(),
        code.procedures()[2].to_hex(),
    ];
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
    let target_account_id = AccountId::try_from(ACCOUNT_ID_SENDER_1).unwrap();
    let (target_pub_key, target_falcon_auth) = get_new_pk_and_authenticator();

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
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target)
        .unwrap();

    // Note outputted by the transaction
    let tx_output_note = executed_transaction.output_notes().get_note(0);

    // Note expected to be outputted by the transaction
    let expected_note = create_output_note(None, create_note_a).unwrap();

    // Check that the output note is the same as the expected note
    assert_eq!(
        NoteHeader::from(tx_output_note).metadata(),
        NoteHeader::from(expected_note.clone()).metadata()
    );
    assert_eq!(
        NoteHeader::from(tx_output_note),
        NoteHeader::from(expected_note.clone())
    );

    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());
}
