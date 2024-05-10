use miden_vm::{prove, verify, Assembler, DefaultHost, ProvingOptions, StackInputs};

#[test]
fn test_exp_masm() {
    // Instantiate the assembler
    let assembler = Assembler::default().with_debug_mode(true);

    // Read the assembly program from a file
    let assembly_code: &str = include_str!("../../src/math/exp.masm");

    // Compile the program from the loaded assembly code
    let program = assembler
        .compile(assembly_code)
        .expect("Failed to compile the assembly code");

    // Define the input for the exponential calculation
    let input = 2000000;
    let real_input = (input as f64) / 1e6;

    // Calculate the exponential using Rust's standard library
    let rust_exp = real_input.exp();
    let expected_result = (rust_exp * 1e6).round() as u64;

    let stack_inputs = StackInputs::try_from_ints([input]).unwrap();
    let cloned_inputs = stack_inputs.clone();

    let host = DefaultHost::default();

    // Execute the program and generate a STARK proof
    let (outputs, proof) = prove(&program, stack_inputs, host, ProvingOptions::default())
        .expect("Failed to execute the program and generate a proof");

    println!("Stack output:");
    println!("{:?}", outputs.stack());

    let result = outputs.stack().get(0).unwrap().as_int();

    println!("Result: {}", result);
    println!("Expected (Rust calculated): {}", expected_result);

    // Define a relative margin of error (for example, 0.05% of the expected result)
    let margin_of_error = (expected_result as f64 * 0.0005).round() as u64;
    let is_within_error = (result as i64 - expected_result as i64).abs() <= margin_of_error as i64;

    assert!(
        is_within_error,
        "The result from MASM and Rust's calculation differ beyond the allowed margin: ±{}",
        margin_of_error
    );

    verify(program.into(), cloned_inputs, outputs, proof).unwrap();
    println!("Program run successfully");
}
