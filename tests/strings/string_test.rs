use miden_vm::{prove, verify, Assembler, DefaultHost, ProvingOptions, StackInputs};

fn concatenate_ascii(input: &str) -> Vec<u64> {
    let bytes = input.as_bytes();
    let mut concatenated_values = Vec::new();

    for chunk in bytes.chunks(8) {
        let mut value: u64 = 0;
        for &byte in chunk.iter() {
            value = (value << 8) | byte as u64;
        }
        concatenated_values.push(value);
    }

    concatenated_values
}

fn decompress_ascii(values: Vec<u64>) -> String {
    let mut decompressed = Vec::new();

    for &value in values.iter() {
        let mut val = value;
        let mut chunk = Vec::new();
        loop {
            let byte = (val & 0xFF) as u8;
            chunk.push(byte);
            if val < 256 {
                break;
            }
            val >>= 8;
        }
        chunk.reverse();
        decompressed.extend(chunk);
    }

    // Remove trailing zeros
    while let Some(&0) = decompressed.last() {
        decompressed.pop();
    }

    String::from_utf8(decompressed).expect("Failed to convert bytes to string")
}

#[test]
fn test_concatenate_and_decompress() {
    let input = "hello world";
    let concatenated = concatenate_ascii(input);
    let decompressed = decompress_ascii(concatenated.clone());

    assert_eq!(input, decompressed);
    println!("Concatenated values: {:?}", concatenated);
    println!("Decompressed string: {}", decompressed);
}

#[test]
fn test_string() {
    // Instantiate the assembler
    let assembler = Assembler::default().with_debug_mode(true);

    // Read the assembly program from a file
    let assembly_code: &str = include_str!("../../src/strings/strings.masm");

    // Compile the program from the loaded assembly code
    let program = assembler
        .compile(assembly_code)
        .expect("Failed to compile the assembly code");

    let input = "hello world";

    // Convert the input string to a concatenated ASCII array
    let concatenated_values = concatenate_ascii(input);
    println!("Concatenated values: {:?}", concatenated_values);

    // input to stack inputs must be a Vec<u64>
    let stack_inputs = StackInputs::try_from_ints(concatenated_values.clone()).unwrap();
    let cloned_inputs = stack_inputs.clone();

    let host = DefaultHost::default();

    // Execute the program and generate a STARK proof
    let (outputs, proof) = prove(&program, stack_inputs, host, ProvingOptions::default())
        .expect("Failed to execute the program and generate a proof");

    println!("Stack output:");
    println!("{:?}", outputs.stack());

    let result = outputs.stack().get(0).unwrap().as_int();
    println!("Result: {}", result);

    let decompressed_string = decompress_ascii(concatenated_values);
    // assert_eq!(input, decompressed_string);
    println!("Decompressed string: {}", decompressed_string);

    verify(program.into(), cloned_inputs, outputs, proof).unwrap();
    println!("Program run successfully");
}
