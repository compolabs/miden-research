# Miden Research

This repository is dedicated to exploring, experimenting, and learning more about how to use Miden and Miden Assembly.

Disclaimer: Currently this repository is in progress of being updated from Miden v0.3 to v0.4. Not all tests are currently working.

### Running Tests:

Simple AMM test:
```
cargo test --package miden-research --test mock_integration -- amm_swap_test::test_swap_asset_amm --exact --show-output
```

Square root test:
```
cargo test --package miden-research --test math -- sqrt_test::test_sqrt_masm --exact --show-output
```

Testnet client integration tests:
```
# This will ensure we start from a clean node and client
cargo make reset
# This command will clone the node's repo and generate the accounts and genesis files and lastly start the node 
cargo make node
# This command will run the node on background
cargo make start-node 
# This will run the integration test 
cargo make integration-test
```

Before pushing run:
```
cargo test --test mock_integration
```