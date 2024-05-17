# Miden Research

This repository is dedicated to exploring, experimenting, and learning more about how to use Miden and Miden Assembly.

### Running Tests:

Simple AMM test:
```
cargo test --package miden-research --test mock_integration -- amm_swap_test::test_swap_asset_amm --exact --show-output
```

Square root test:
```
cargo test --package miden-research --test math -- sqrt_test::test_sqrt_masm --exact --show-output
```
