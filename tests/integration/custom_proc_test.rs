use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, SlotItem, StorageSlot},
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
use std::fs;

const MASTS: [&str; 3] = [
    "0x6b42a86658b1ecb729e86d47bd0fae6d57cecbc2ef52a81e0d87b3371fa75174",
    "0xe06a83054c72efc7e32698c4fc6037620cde834c9841afb038a5d39889e502b6",
    "0xf3bf6e2af9084abd1b24580d1378b61b7ce146831e65f5a6d9646c85332dd462",
];
pub fn mock_account_code(assembler: &Assembler) -> AccountCode {
    let account_code = "\
            use.miden::account
            use.miden::tx
            use.miden::contracts::wallets::basic->wallet
            use.miden::contracts::auth::basic->basic_eoa

            # acct proc 0
            export.wallet::receive_asset

            # acct proc 1
            export.basic_eoa::auth_tx_rpo_falcon512

            # acct proc 2
            export.account_procedure_1
                push.3.4
                add
                debug.stack
                drop
            end
            ";
    let account_module_ast = ModuleAst::parse(account_code).unwrap();
    let code = AccountCode::new(account_module_ast, assembler).unwrap();

    // Ensures the mast root constants match the latest version of the code.
    //
    // The constants will change if the library code changes, and need to be updated so that the
    // tests will work properly. If these asserts fail, copy the value of the code (the left
    // value), into the constants.
    //
    // Comparing all the values together, in case multiple of them change, a single test run will
    // detect it.
    let current = [
        code.procedures()[0].to_hex(),
        code.procedures()[1].to_hex(),
        code.procedures()[2].to_hex(),
    ];

    println!("{:?}", current[2]);
    println!("{:?}", code.procedures()[2]);
    // println!("const MASTS: [&str; 4] = {:?};", current);

    assert!(current == MASTS, "const MASTS: [&str; 8] = {:?};", current);

    code
}

pub fn get_account_with_custom_proc(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);

    let account_code = mock_account_code(&assembler);
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

fn create_note<R: FeltRng>(
    sender_account_id: AccountId,
    target_account_id: AccountId,
    assets: Vec<Asset>,
    mut rng: R,
) -> Result<Note, NoteError> {
    let filename = "./src/masm/custom_proc/note_script.masm";
    let note_script = fs::read_to_string(filename).expect("Failed to read the assembly file");

    let note_assembler = TransactionKernel::assembler().with_debug_mode(true);

    let script_ast = ProgramAst::parse(&note_script).unwrap();
    // let (note_script, _) = NoteScript::new(script_ast, &note_assembler)?;

    let (note_script, _) = new_note_script(script_ast, &note_assembler).unwrap();

    // add the inputs to the note
    let input_a = Felt::new(123);

    let inputs = NoteInputs::new(vec![input_a, input_a])?;

    let tag = NoteTag::from_account_id(target_account_id, NoteExecutionMode::Local)?;
    let serial_num = rng.draw_word();
    let aux = ZERO;
    let note_type = NoteType::OffChain;
    let metadata = NoteMetadata::new(sender_account_id, note_type, tag, aux)?;

    let vault = NoteAssets::new(assets)?;

    let recipient = NoteRecipient::new(serial_num, note_script, inputs);

    Ok(Note::new(vault, metadata, recipient))
}

#[test]
fn test_custom_proc() {
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

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

    let tx_script_code = ProgramAst::parse(
        "
        use.miden::contracts::auth::basic->auth_tx

        begin
            call.auth_tx::auth_tx_rpo_falcon512
            # dropw
            # call account_procedure_1
            call.0xf3bf6e2af9084abd1b24580d1378b61b7ce146831e65f5a6d9646c85332dd462
            
            # dropw
            debug.stack
            dup
            drop
        end",
    )
    .unwrap();

    let tx_script_target = executor
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(target_pub_key, target_sk_pk_felt)],
            vec![],
        )
        .unwrap();

    let tx_args_target = TransactionArgs::new(Some(tx_script_target), None, AdviceMap::default());

    // Execute the transaction and get the witness
    let _executed_transaction =
        executor.execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target);

    println!(
        "{:?}",
        _executed_transaction
            .unwrap()
            .account_delta()
            .vault()
            .added_assets
    );
}
