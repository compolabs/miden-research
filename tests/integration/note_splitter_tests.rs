use miden_client::client::{
    accounts::{AccountStorageMode, AccountTemplate},
    transactions::transaction_request::TransactionRequest,
};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{
        Account, AccountCode, AccountData, AccountId, AccountStorage, AuthSecretKey, SlotItem,
        StorageSlot,
    },
    assembly::{ModuleAst, ProgramAst},
    assets::{AssetVault, FungibleAsset, TokenSymbol},
    crypto::dsa::rpo_falcon512::SecretKey,
    crypto::rand::{FeltRng, RpoRandomCoin},
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteInputs, NoteMetadata, NoteRecipient, NoteTag,
        NoteType,
    },
    Felt, Word,
};
use miden_tx::utils::Serializable;
use miden_vm::Assembler;
use std::collections::BTreeMap;

use super::common::*;

const MASTS: [&str; 3] = [
    "0x74de7e94e5afc71e608f590c139ac51f446fc694da83f93d968b019d1d2b7306", // receive_asset proc
    "0x30ab7cac0307a30747591be84f78a6d0c511b0f2154a8e22b6d7869207bc50c2", // get assets proc
    "0xbfc82a0785cba42b125147f5716ef7df0c7c0b0e60a49dae71121310c6cca0dc", // swap assets proc
];

pub fn account_code(assembler: &Assembler) -> AccountCode {
    let account_code = include_str!("../../src/splitter/splitter_account.masm");

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

pub fn get_account_with_custom_proc(account_id: AccountId, public_key: Word) -> Account {
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);

    let account_code = account_code(&assembler);
    let account_storage = AccountStorage::new(
        vec![SlotItem {
            index: 0,
            slot: StorageSlot::new_value(public_key),
        }],
        vec![],
    )
    .unwrap();

    let account_vault = AssetVault::new(&[]).unwrap();

    Account::new(
        account_id,
        account_vault,
        account_storage,
        account_code,
        Felt::new(1),
    )
}

// CUSTOM TRANSACTION REQUEST
// ================================================================================================
//
// The following functions are for testing custom transaction code. What the test does is:
//
// - Create a custom tx that mints a custom note which checks that the note args are as expected
//   (ie, a word of 4 felts that represent [9, 12, 18, 3])
//
// - Create another transaction that consumes this note with custom code. This custom code only
//   asserts that the {asserted_value} parameter is 0. To test this we first execute with
//   an incorrect value passed in, and after that we try again with the correct value.
//
// Because it's currently not possible to create/consume notes without assets, the P2ID code
// is used as the base for the note code.
#[tokio::test]
async fn test_custom_accounts_and_notes() {
    let mut client = create_test_client();
    wait_for_node(&mut client).await;

    let account_template = AccountTemplate::BasicWallet {
        mutable_code: false,
        storage_mode: AccountStorageMode::Local,
    };

    client.sync_state().await.unwrap();

    // @dev, importing custom account
    let splitter_account_id = AccountId::try_from(3458764513820540975).unwrap();
    let (target_pub_key, target_sk_pk_felt) = get_new_key_pair_with_advice_map();
    let splitter_account = get_account_with_custom_proc(splitter_account_id, target_pub_key);

    let splitter_account_data: AccountData = AccountData::new(
        splitter_account,
        None,
        AuthSecretKey::RpoFalcon512(SecretKey::new()),
    );

    client.import_account(splitter_account_data).unwrap();

    // Insert Account
    let (regular_account, _seed) = client.new_account(account_template).unwrap();

    let account_template = AccountTemplate::FungibleFaucet {
        token_symbol: TokenSymbol::new("TEST").unwrap(),
        decimals: 5u8,
        max_supply: 10_000u64,
        storage_mode: AccountStorageMode::Local,
    };
    let (fungible_faucet, _seed) = client.new_account(account_template).unwrap();
    println!("sda1");

    // Execute mint transaction in order to create custom note
    let note = mint_custom_note(&mut client, fungible_faucet.id(), regular_account.id()).await;
    client.sync_state().await.unwrap();

    // Prepare transaction

    // If these args were to be modified, the transaction would fail because the note code expects
    // these exact arguments
    let note_args = [[Felt::new(9), Felt::new(12), Felt::new(18), Felt::new(3)]];

    let note_args_map = BTreeMap::from([(note.id(), Some(note_args[0]))]);

    /*     let code = "
           use.miden::contracts::auth::basic->auth_tx
           use.miden::kernels::tx::prologue
           use.miden::kernels::tx::memory

           begin
               push.0 push.{asserted_value}
               # => [0, {asserted_value}]
               assert_eq

               call.auth_tx::auth_tx_rpo_falcon512
           end
           ";
    */
    // SUCCESS EXECUTION

    let code = include_str!("../../src/splitter/asset_note.masm");
    let program = ProgramAst::parse(&code).unwrap();

    let tx_script = {
        let account_auth = client.get_account_auth(regular_account.id()).unwrap();
        let (pubkey_input, advice_map): (Word, Vec<Felt>) = match account_auth {
            AuthSecretKey::RpoFalcon512(key) => (
                key.public_key().into(),
                key.to_bytes()
                    .iter()
                    .map(|a| Felt::new(*a as u64))
                    .collect::<Vec<Felt>>(),
            ),
        };

        let script_inputs = vec![(pubkey_input, advice_map)];
        client
            .compile_tx_script(program, script_inputs, vec![])
            .unwrap()
    };

    let transaction_request = TransactionRequest::new(
        regular_account.id(),
        note_args_map,
        vec![],
        vec![],
        Some(tx_script),
    );

    execute_tx_and_sync(&mut client, transaction_request).await;

    client.sync_state().await.unwrap();
}

async fn mint_custom_note(
    client: &mut TestClient,
    faucet_account_id: AccountId,
    target_account_id: AccountId,
) -> Note {
    // Prepare transaction
    let mut random_coin = RpoRandomCoin::new(Default::default());
    let note = create_custom_note(
        client,
        faucet_account_id,
        target_account_id,
        &mut random_coin,
    );

    let recipient = note
        .recipient()
        .digest()
        .iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".");

    let note_tag = note.metadata().tag().inner();

    let code = "
    use.miden::contracts::faucets::basic_fungible->faucet
    use.miden::contracts::auth::basic->auth_tx
    
    begin
        push.{recipient}
        push.{note_type}
        push.{tag}
        push.{amount}
        call.faucet::distribute
    
        call.auth_tx::auth_tx_rpo_falcon512
        dropw dropw
    end
    "
    .replace("{recipient}", &recipient)
    .replace(
        "{note_type}",
        &Felt::new(NoteType::OffChain as u64).to_string(),
    )
    .replace("{tag}", &Felt::new(note_tag.into()).to_string())
    .replace("{amount}", &Felt::new(10).to_string());

    let program = ProgramAst::parse(&code).unwrap();

    let tx_script = client.compile_tx_script(program, vec![], vec![]).unwrap();

    let transaction_request = TransactionRequest::new(
        faucet_account_id,
        BTreeMap::new(),
        vec![note.clone()],
        vec![],
        Some(tx_script),
    );

    let _ = execute_tx_and_sync(client, transaction_request).await;
    note
}

fn create_custom_note(
    client: &TestClient,
    faucet_account_id: AccountId,
    target_account_id: AccountId,
    rng: &mut RpoRandomCoin,
) -> Note {
    let expected_note_arg = [Felt::new(9), Felt::new(12), Felt::new(18), Felt::new(3)]
        .iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".");

    let note_script =
        include_str!("asm/custom_p2id.masm").replace("{expected_note_arg}", &expected_note_arg);
    let note_script = ProgramAst::parse(&note_script).unwrap();
    let note_script = client.compile_note_script(note_script, vec![]).unwrap();

    let inputs = NoteInputs::new(vec![target_account_id.into()]).unwrap();
    let serial_num = rng.draw_word();
    let note_metadata = NoteMetadata::new(
        faucet_account_id,
        NoteType::OffChain,
        NoteTag::from_account_id(target_account_id, NoteExecutionHint::Local).unwrap(),
        Default::default(),
    )
    .unwrap();
    let note_assets = NoteAssets::new(vec![FungibleAsset::new(faucet_account_id, 10)
        .unwrap()
        .into()])
    .unwrap();
    let note_recipient = NoteRecipient::new(serial_num, note_script, inputs);
    Note::new(note_assets, note_metadata, note_recipient)
}
