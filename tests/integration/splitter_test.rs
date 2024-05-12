use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, AccountStorageType, AccountType, SlotItem,
        StorageSlot,
    },
    accounts::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_SENDER,
    },
    assembly::{AssemblyContext, ModuleAst, ProgramAst},
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::rand::{FeltRng, RpoRandomCoin},
    notes::{
        Note, NoteAssets, NoteExecutionMode, NoteInputs, NoteMetadata, NoteRecipient, NoteScript,
        NoteTag, NoteType,
    },
    transaction::TransactionArgs,
    vm::CodeBlock,
    Felt, NoteError, Word, ZERO,
};
use miden_processor::AdviceMap;
use miden_tx::TransactionExecutor;
use miden_vm::Assembler;

use crate::utils::{get_new_key_pair_with_advice_map, MockDataStore};

const MASTS: [&str; 2] = [
    "0xe06a83054c72efc7e32698c4fc6037620cde834c9841afb038a5d39889e502b6", // receive_asset proc
    "0x5b6c888475118067bc48ae7bf5262a34ddac1a75d5bbaf4b871dc8790ef022ff", // split_note custom proc
];

pub fn account_code(assembler: &Assembler) -> AccountCode {
    let account_code = include_str!("../../src/splitter/splitter_account.masm");

    let account_module_ast = ModuleAst::parse(account_code).unwrap();
    let code = AccountCode::new(account_module_ast, assembler).unwrap();

    let current = [code.procedures()[0].to_hex(), code.procedures()[1].to_hex()];

    assert!(current == MASTS, "UPDATE MAST ROOT: {:?};", current);

    code
}

pub fn get_account_with_custom_proc(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);

    let account_code = account_code(&assembler);
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

const fn account_id(account_type: AccountType, storage: AccountStorageType, rest: u64) -> u64 {
    let mut id = 0;

    id ^= (storage as u64) << 62;
    id ^= (account_type as u64) << 60;
    id ^= rest;

    id
}

fn create_note<R: FeltRng>(
    sender_account_id: AccountId,
    target_account_id: AccountId,
    assets: Vec<Asset>,
    mut rng: R,
) -> Result<Note, NoteError> {
    let note_script = include_str!("../../src/splitter/asset_note.masm");

    let note_assembler = TransactionKernel::assembler().with_debug_mode(true);

    let script_ast = ProgramAst::parse(&note_script).unwrap();
    let (note_script, _) = new_note_script(script_ast, &note_assembler).unwrap();

    // @dev TODO add user addresses as input to the note
    let user_0 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageType::OffChain,
        45,
    );

    let user_1 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageType::OffChain,
        46,
    );

    let user_0_felt = Felt::new(user_0);
    let user_1_felt = Felt::new(user_1);

    let inputs = NoteInputs::new(vec![user_0_felt, user_1_felt])?;

    let tag = NoteTag::from_account_id(target_account_id, NoteExecutionMode::Local)?;
    let serial_num = rng.draw_word();
    let aux = ZERO;
    let note_type = NoteType::OffChain;
    let metadata = NoteMetadata::new(sender_account_id, note_type, tag, aux)?;

    let vault = NoteAssets::new(assets)?;

    let recipient = NoteRecipient::new(serial_num, note_script, inputs);

    Ok(Note::new(vault, metadata, recipient))
}

// Run this first to check MASTs are correct
#[test]
pub fn check_account_masts() {
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);
    let account_code = include_str!("../../src/splitter/splitter_account.masm");

    let account_module_ast = ModuleAst::parse(account_code).unwrap();
    let code = AccountCode::new(account_module_ast, &assembler).unwrap();

    let current = [code.procedures()[0].to_hex(), code.procedures()[1].to_hex()];
    assert!(current == MASTS, "UPDATE MAST ROOT: {:?};", current);
}

#[test]
fn test_call_split_asset() {
    // Create fungible asset (right now notes must have at least one asset, so we create a fungible asset with 0 amount)
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 1010000).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // Create target account
    let target_account_id = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let (target_pub_key, target_sk_pk_felt) = get_new_key_pair_with_advice_map();
    let target_account = get_account_with_custom_proc(target_account_id, target_pub_key, None);

    // Create the note
    let note = create_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset],
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

    let tx_script_code = include_str!("../../src/splitter/tx_script.masm");
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
    let _executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target)
        .expect("Transaction consuming swap note failed");

    println!("{:?}", _executed_transaction.account_delta());

    let created_note = _executed_transaction.output_notes().get_note(0);
    println!("{:?}", created_note);
}
