use miden_vm::{prove, verify, Assembler, DefaultHost, ProvingOptions, StackInputs};

#[test]
fn test_sqrt_masm() {
    // Instantiate the assembler
    let assembler = Assembler::default();

    // Read the assembly program from a file
    let assembly_code: &str = include_str!("../../src/masm/math/sqrt.masm");

    // Compile the program from the loaded assembly code
    let program = assembler
        .compile(assembly_code)
        .expect("Failed to compile the assembly code");

    let input = 2000000; // This is 2.0 scaled by 1e6 for the input
    let real_input = (input as f64) / 1e6;

    // Calculate expected square root in Rust
    let real_sqrt = real_input.sqrt();
    let expected_result = (real_sqrt * 1e6).round() as u64;

    let stack_inputs = StackInputs::try_from_ints([input]).unwrap();
    let cloned_inputs = stack_inputs.clone();

    let host = DefaultHost::default();

    // Execute the program and generate a STARK proof
    let (outputs, proof) = prove(&program, stack_inputs, host, ProvingOptions::default())
        .expect("Failed to execute the program and generate a proof");

    println!("Stack output:");
    println!("{:?}", outputs.stack());

    let result = outputs.stack().get(0).unwrap().as_int();

    // Define an acceptable margin of error
    let margin_of_error = 1000;

    // Assert that the result is within the expected range
    assert!(
        (result >= expected_result.saturating_sub(margin_of_error))
            && (result <= expected_result + margin_of_error),
        "Result out of acceptable range: {} not within {} +/- {}",
        result,
        expected_result,
        margin_of_error
    );

    verify(program.into(), cloned_inputs, outputs, proof).unwrap();
    println!("Program run successfully");
}
