use miden_objects::{
    accounts::{Account, AccountId},
    assembly::ModuleAst,
    // assets::{Asset, AssetVault, FungibleAsset},
    crypto::{dsa::rpo_falcon512::SecretKey, utils::Serializable},
    notes::{Note, NoteId},
    transaction::{
        ChainMmr, InputNote, InputNotes, OutputNote, TransactionArgs, TransactionInputs,
    },
    BlockHeader,
    Felt,
    Word,
};
use miden_tx::{DataStore, DataStoreError};
use mock::mock::{
    account::MockAccountType,
    notes::AssetPreservationStatus,
    transaction::{mock_inputs, mock_inputs_with_existing},
};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

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

// ________________________________MOCK DATA STORE________________________________

#[derive(Clone)]
pub struct MockDataStore {
    pub account: Account,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub notes: Vec<InputNote>,
    pub tx_args: TransactionArgs,
}

impl MockDataStore {
    pub fn new() -> Self {
        let (tx_inputs, tx_args) = mock_inputs(
            MockAccountType::StandardExisting,
            AssetPreservationStatus::Preserved,
        );
        let (account, _, block_header, block_chain, notes) = tx_inputs.into_parts();
        Self {
            account,
            block_header,
            block_chain,
            notes: notes.into_vec(),
            tx_args,
        }
    }

    pub fn with_existing(account: Option<Account>, input_notes: Option<Vec<Note>>) -> Self {
        let (
            account,
            block_header,
            block_chain,
            consumed_notes,
            _auxiliary_data_inputs,
            created_notes,
        ) = mock_inputs_with_existing(
            MockAccountType::StandardExisting,
            AssetPreservationStatus::Preserved,
            account,
            input_notes,
        );
        let output_notes = created_notes.into_iter().filter_map(|note| match note {
            OutputNote::Public(note) => Some(note),
            OutputNote::Private(_) => None,
        });
        let mut tx_args = TransactionArgs::default();
        tx_args.extend_expected_output_notes(output_notes);

        Self {
            account,
            block_header,
            block_chain,
            notes: consumed_notes,
            tx_args,
        }
    }
}

impl Default for MockDataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DataStore for MockDataStore {
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        assert_eq!(block_num, self.block_header.block_num());
        assert_eq!(notes.len(), self.notes.len());

        let notes = self
            .notes
            .iter()
            .filter(|note| notes.contains(&note.id()))
            .cloned()
            .collect::<Vec<_>>();

        Ok(TransactionInputs::new(
            self.account.clone(),
            None,
            self.block_header,
            self.block_chain.clone(),
            InputNotes::new(notes).unwrap(),
        )
        .unwrap())
    }

    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        Ok(self.account.code().module().clone())
    }
}
