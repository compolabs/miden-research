use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{
        account_id::testing::ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1, Account, AccountCode,
        AccountId, AccountStorage, SlotItem, StorageSlot,
    },
    assembly::{AssemblyContext, ModuleAst, ProgramAst},
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::rand::{FeltRng, RpoRandomCoin},
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteInputs, NoteMetadata, NoteRecipient, NoteScript,
        NoteTag, NoteType,
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

const MASTS: [&str; 3] = [
    "0x74de7e94e5afc71e608f590c139ac51f446fc694da83f93d968b019d1d2b7306", // receive_asset proc
    "0x30ab7cac0307a30747591be84f78a6d0c511b0f2154a8e22b6d7869207bc50c2", // get assets proc
    "0xbfc82a0785cba42b125147f5716ef7df0c7c0b0e60a49dae71121310c6cca0dc", // swap assets proc
];

pub fn account_code(assembler: &Assembler) -> AccountCode {
    let account_code = include_str!("../../src/amm/pool_account.masm");

    let account_module_ast = ModuleAst::parse(account_code).unwrap();
    let code = AccountCode::new(account_module_ast, assembler).unwrap();

    let current = [
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
    assets: Vec<Asset>,
) -> Account {
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);

    let account_code = account_code(&assembler);
    let account_storage = AccountStorage::new(
        vec![SlotItem {
            index: 0,
            slot: StorageSlot::new_value(public_key),
        }],
        BTreeMap::new(),
    )
    .unwrap();
    let account_vault = AssetVault::new(&assets).unwrap();

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

fn create_amm_swap_note<R: FeltRng>(
    sender_account_id: AccountId,
    target_account_id: AccountId,
    token_in: Asset,
    token_out: Asset,
    mut rng: R,
) -> Result<Note, NoteError> {
    let note_script = include_str!("../../src/amm/amm_note.masm");
    let note_assembler = TransactionKernel::assembler().with_debug_mode(true);

    let script_ast = ProgramAst::parse(&note_script).unwrap();
    let (note_script, _) = new_note_script(script_ast, &note_assembler).unwrap();

    let token_out_felt = Felt::new(token_out.faucet_id().into());

    let inputs = NoteInputs::new(vec![token_out_felt, sender_account_id.into()])?;

    let tag = NoteTag::from_account_id(target_account_id, NoteExecutionHint::Local)?;
    let serial_num = rng.draw_word();
    let aux = ZERO;
    let note_type = NoteType::OffChain;
    let metadata = NoteMetadata::new(sender_account_id, note_type, tag, aux)?;

    let vault = NoteAssets::new(vec![token_in])?;

    let recipient = NoteRecipient::new(serial_num, note_script, inputs);

    Ok(Note::new(vault, metadata, recipient))
}

// Run this first to check MASTs are correct
#[test]
pub fn check_account_masts() {
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);
    let account_code = include_str!("../../src/amm/pool_account.masm");

    let account_module_ast = ModuleAst::parse(account_code).unwrap();
    let code = AccountCode::new(account_module_ast, &assembler).unwrap();

    let current = [
        code.procedures()[0].to_hex(),
        code.procedures()[1].to_hex(),
        code.procedures()[2].to_hex(),
    ];
    assert!(current == MASTS, "UPDATE MAST ROOT: {:?};", current);
}

#[test]
fn test_swap_asset_amm() {
    // TOKEN A
    let faucet_id_a = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_amount_a: u64 = 10002;
    let fungible_asset_a: Asset = FungibleAsset::new(faucet_id_a, fungible_asset_amount_a)
        .unwrap()
        .into();

    // TOKEN B
    let faucet_id_b = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap();
    let fungible_asset_amount_b = 10005;
    let fungible_asset_b: Asset = FungibleAsset::new(faucet_id_b, fungible_asset_amount_b)
        .unwrap()
        .into();

    // Create user asset TOKEN A
    let fungible_asset_amount_user: u64 = 101;
    let fungible_asset_a_user: Asset = FungibleAsset::new(faucet_id_a, fungible_asset_amount_user)
        .unwrap()
        .into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // Create AMM SWAP contract account
    let target_account_id = AccountId::try_from(ACCOUNT_ID_SENDER_1).unwrap();
    let (target_pub_key, target_falcon_auth) = get_new_pk_and_authenticator();

    let target_account = get_account_with_custom_proc(
        target_account_id,
        target_pub_key,
        vec![fungible_asset_b, fungible_asset_a],
    );

    // Create the user AMM swap note (not SWAP note)
    let note = create_amm_swap_note(
        sender_account_id,
        target_account_id,
        fungible_asset_a_user,
        fungible_asset_b,
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

    let tx_script_code = include_str!("../../src/amm/tx_script.masm");
    let tx_script_ast = ProgramAst::parse(tx_script_code).unwrap();

    let tx_script_target = executor
        .compile_tx_script(tx_script_ast.clone(), vec![], vec![])
        .unwrap();

    let tx_args_target = TransactionArgs::new(Some(tx_script_target), None, AdviceMap::default());

    // Execute the transaction and get the witness
    let _executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target)
        .expect("Transaction consuming swap note failed");

    let created_note_0 = _executed_transaction.output_notes().get_note(0);
    println!("Note 1 {:?}", created_note_0);
}
