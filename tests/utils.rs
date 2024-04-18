use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, SlotItem, StorageSlot},
    assembly::{ModuleAst, ProgramAst},
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::{dsa::rpo_falcon512::SecretKey, utils::Serializable},
    notes::{
        Note, NoteAssets, NoteId, NoteInputs, NoteMetadata, NoteRecipient, NoteScript, NoteType,
    },
    transaction::{
        ChainMmr, ExecutedTransaction, InputNote, InputNotes, OutputNote, ProvenTransaction,
        TransactionArgs, TransactionInputs,
    },
    BlockHeader, Felt, Word, ZERO,
};
use miden_prover::ProvingOptions;
use miden_tx::{
    DataStore, DataStoreError, TransactionProver, TransactionVerifier, TransactionVerifierError,
};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

/*
use mock::{
  constants::MIN_PROOF_SECURITY_LEVEL,
  mock::{
      account::{MockAccountType, DEFAULT_ACCOUNT_CODE},
      notes::AssetPreservationStatus,
      transaction::{mock_inputs, mock_inputs_with_existing},
  },
};
*/

#[cfg(test)]
pub fn get_new_key_pair_with_advice_map() -> (Word, Vec<Felt>) {
    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key: Word = sec_key.public_key().into();
    let mut pk_sk_bytes = sec_key.to_bytes();
    pk_sk_bytes.append(&mut pub_key.to_bytes());
    let pk_sk_felts: Vec<Felt> = pk_sk_bytes
        .iter()
        .map(|a| Felt::new(*a as u64))
        .collect::<Vec<Felt>>();

    (pub_key, pk_sk_felts)
}
