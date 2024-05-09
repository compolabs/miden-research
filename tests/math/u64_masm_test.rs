use miden_processor::ProcessState;
use mock::{
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction, run_tx,
};

#[test]
fn test_set_item() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    let assembly_code: &str = include_str!("../../src/math/u64.masm");

    let transaction = prepare_transaction(tx_inputs, tx_args, &assembly_code, None);
    let _process = run_tx(&transaction).unwrap();

    println!("{:?}", _process.get_stack_state());
}
