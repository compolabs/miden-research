use miden_vm::{prove, verify, Assembler, DefaultHost, ProvingOptions, StackInputs};
use std::fs;

#[test]
fn test_amm() {
    // Instantiate the assembler
    let assembler = Assembler::default();

    // Read the assembly program from a file
    let filename = "./src/masm/amm.masm"; // Specify the path to your file
    let assembly_code = fs::read_to_string(filename).expect("Failed to read the assembly file"); // This will panic if the file cannot be read

    // Compile the program from the loaded assembly code
    let program = assembler
        .compile(&assembly_code)
        .expect("Failed to compile the assembly code"); // This will panic if the compilation fails

    let amount_in_x = 1000000;

    let stack_inputs = StackInputs::try_from_ints([amount_in_x]).unwrap();
    let cloned_inputs = stack_inputs.clone(); // Clone the inputs for later use

    let host = DefaultHost::default();

    // Execute the program and generate a STARK proof
    let (outputs, proof) = prove(
        &program,
        stack_inputs,              // No input is provided
        host,                      // Using a default host
        ProvingOptions::default(), // Using default options
    )
    .expect("Failed to execute the program and generate a proof"); // This will panic if the proof generation fails

    println!("Stack output:");
    println!("{:?}", outputs.stack());

    // Verify the proof
    verify(program.into(), cloned_inputs, outputs, proof).unwrap();

    println!("Program run successfully");
}
