use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, SlotItem, StorageSlot},
    assembly::{AssemblyContext, ModuleAst, ProgramAst},
    assets::{Asset, AssetVault},
    crypto::rand::{FeltRng, RpoRandomCoin},
    notes::{
        Note, NoteAssets, NoteDetails, NoteExecutionHint, NoteHeader, NoteInputs, NoteMetadata,
        NoteRecipient, NoteScript, NoteTag, NoteType,
    },
    transaction::TransactionArgs,
    vm::CodeBlock,
    Felt, NoteError, Word, ZERO,
};
use miden_processor::AdviceMap;
use miden_tx::TransactionExecutor;
use miden_vm::Assembler;

use crate::utils::{
    get_new_key_pair_with_advice_map, MockDataStore,
    ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
    ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2, ACCOUNT_ID_SENDER,
};

const MASTS: [&str; 1] = [
    "0x69a58857dd6bf37d31106355beeeb169c3596a972c713f316b13f148d6858b63", // create note
];

const ACCOUNT_CODE: &str = include_str!("../../src/note_output/note_creator.masm");

pub fn account_code(assembler: &Assembler) -> AccountCode {
    let account_module_ast = ModuleAst::parse(ACCOUNT_CODE).unwrap();
    let code = AccountCode::new(account_module_ast, assembler).unwrap();

    let current = [code.procedures()[0].to_hex()];

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

fn create_custom_note<R: FeltRng>(
    sender_account_id: AccountId,
    target_account_id: AccountId,
    mut rng: R,
) -> Result<Note, NoteError> {
    let note_script = include_str!("../../src/note_output/message_note.masm");

    let note_assembler = TransactionKernel::assembler().with_debug_mode(true);

    let script_ast = ProgramAst::parse(&note_script).unwrap();
    let (note_script, _) = new_note_script(script_ast, &note_assembler).unwrap();

    // add the inputs to the note
    // let input_a = Felt::new(123);

    let inputs = NoteInputs::new(vec![])?;

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

pub fn create_output_note() -> Result<Note, NoteError> {
    let sender_account_id: AccountId =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();

    // Create target smart contract
    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();

    let note_script = include_str!("../../src/note_output/output_note.masm");

    let note_assembler = TransactionKernel::assembler().with_debug_mode(true);

    let script_ast = ProgramAst::parse(&note_script).unwrap();
    let (note_script, _) = new_note_script(script_ast, &note_assembler).unwrap();

    // add the inputs to the note
    // let input_a = Felt::new(123);

    let inputs: NoteInputs = NoteInputs::new(vec![]).unwrap();

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

    let current = [code.procedures()[0].to_hex()];
    assert!(current == MASTS, "UPDATE MAST ROOT: {:?};", current);
}

#[test]
pub fn get_dynamic_note_recipient() {
    let output_note = create_output_note().unwrap();

    let tag = output_note.clone().metadata().tag();
    let note_type = output_note.clone().metadata().note_type();
    let recipient = output_note.recipient();

    println!("{:?}", tag);
    println!("{:?}", note_type);
    println!("{:?}", recipient.digest());

    // println!("{:?}", output_note.recipient());
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

    // Create the note
    let note = create_custom_note(
        sender_account_id,
        target_account_id,
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
            vec![(target_pub_key, target_sk_pk_felt)],
            vec![],
        )
        .unwrap();

    let tx_args_target = TransactionArgs::new(Some(tx_script_target), None, AdviceMap::default());

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target)
        .unwrap();

    let tx_output_note_header: NoteHeader = executed_transaction.output_notes().get_note(0).into();

    let expected_output_note: Note = create_output_note().unwrap();
    let expected_output_note_header: NoteHeader = expected_output_note.header().clone();

    assert_eq!(expected_output_note_header, tx_output_note_header);
}
