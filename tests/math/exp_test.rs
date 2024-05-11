use miden_vm::{prove, verify, Assembler, DefaultHost, ProvingOptions, StackInputs};
// use rand::Rng;

// In progress...
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
    let input = 200000;
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

/* #[test]
fn test_random_exp_masm() {
    // Instantiate the assembler
    let assembler = Assembler::default().with_debug_mode(true);

    // Read the assembly program from a file
    let assembly_code: &str = include_str!("../../src/math/exp.masm");

    // Compile the program from the loaded assembly code
    let program = assembler
        .compile(assembly_code)
        .expect("Failed to compile the assembly code");

    // Test with multiple random inputs
    let mut rng = rand::thread_rng();
    for _ in 0..10 { // Test 10 random values
        // Generate a random input within the range 0.1 to 100
        let real_input: f64 = rng.gen_range(0.1..100.0);
        let input: u64 = (real_input * 1e6).round() as u64;

        // Calculate the exponential using Rust's standard library, specifying `f64`
        let rust_exp: f64 = real_input.exp();
        let expected_result: u64 = (rust_exp * 1e6).round() as u64;

        let stack_inputs = StackInputs::try_from_ints([input]).unwrap();
        let cloned_inputs = stack_inputs.clone();

        let host = DefaultHost::default();

        // Clone the program for this iteration to avoid move errors
        let program_clone = program.clone();

        // Execute the program and generate a STARK proof
        let (outputs, proof) = prove(&program_clone, stack_inputs, host, ProvingOptions::default())
            .expect("Failed to execute the program and generate a proof");

        println!("Input: {}", real_input);
        println!("Stack output:");
        println!("{:?}", outputs.stack());

        let result = outputs.stack().get(0).unwrap().as_int();

        println!("Result: {}", result);
        println!("Expected (Rust calculated): {}", expected_result);

        // Define a relative margin of error (for example, 0.05% of the expected result)
        let margin_of_error: u64 = (expected_result as f64 * 0.0005).round() as u64;
        let is_within_error: bool = (result as i64 - expected_result as i64).abs() <= margin_of_error as i64;

        assert!(is_within_error, "The result from MASM and Rust's calculation differ beyond the allowed margin: ±{}", margin_of_error);

        verify(program_clone.into(), cloned_inputs, outputs, proof).unwrap();
        println!("Program run successfully for input: {}", real_input);
    }
} */
