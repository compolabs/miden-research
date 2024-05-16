use miden_vm::{prove, verify, Assembler, DefaultHost, ProvingOptions, StackInputs};

#[test]
fn test_amm() {
    // Instantiate the assembler
    let assembler = Assembler::default().with_debug_mode(true);

    // Read the assembly program from a file
    let assembly_code: &str = include_str!("../../src/math/amm.masm");

    // Compile the program from the loaded assembly code
    let program = assembler
        .compile(assembly_code)
        .expect("Failed to compile the assembly code");

    let amount_in_x = 10000;

    let stack_inputs = StackInputs::try_from_ints([amount_in_x]).unwrap();
    let cloned_inputs = stack_inputs.clone();

    let host = DefaultHost::default();

    // Execute the program and generate a STARK proof
    let (outputs, proof) = prove(&program, stack_inputs, host, ProvingOptions::default())
        .expect("Failed to execute the program and generate a proof");

    println!("Stack output:");
    println!("{:?}", outputs.stack());

    // Verify the proof
    verify(program.into(), cloned_inputs, outputs, proof).unwrap();

    println!("Program run successfully");
}
