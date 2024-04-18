use miden_vm::{prove, verify, Assembler, DefaultHost, ProvingOptions, StackInputs};
use std::fs;

#[test]
fn test_nprime() {
    let assembler = Assembler::default();

    // Read the assembly program from a file
    let filename = "./src/masm/nprime.masm";
    let assembly_code = fs::read_to_string(filename).expect("Failed to read the assembly file");

    // Compile the program from the loaded assembly code
    let program = assembler
        .compile(&assembly_code)
        .expect("Failed to compile the assembly code");

    let input = 5;

    let stack_inputs = StackInputs::try_from_ints([input]).unwrap();
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
