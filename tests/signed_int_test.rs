use miden_vm::{prove, verify, Assembler, DefaultHost, ProvingOptions, StackInputs};

const OFFSET: u128 = 9223372034707292160; // (2^64 - 2^32) / 2

fn to_machine_format(x: i64) -> u128 {
    if x < 0 {
        // Correctly calculate the offset to handle negative numbers
        let offset = (1u128 << 64) - (1u128 << 32);
        offset - (x.abs() as u128)
    } else {
        x as u128
    }
}

fn to_normal_format(x: u128) -> i128 {
    println!("x: {}, offset:{}", x, OFFSET);
    if x > OFFSET {
        x as i128 - ((1 << 64) - (1 << 32))
    } else {
        x as i128
    }
}

#[test]
fn test_signed_int_masm() {
    // Instantiate the assembler
    let assembler = Assembler::default().with_debug_mode(true);

    // Read the assembly program from a file
    let assembly_code: &str = include_str!("../src/masm/signed_int/signed_add.masm");

    // Compile the program from the loaded assembly code
    let program = assembler
        .compile(assembly_code)
        .expect("Failed to compile the assembly code");

    let input_a: i64 = -100;
    let input_b: i64 = -100;

    let machine_input_a = to_machine_format(input_a as i64) as u64;
    let machine_input_b = to_machine_format(input_b as i64) as u64;

    println!("Machine input a: {}", machine_input_a);
    println!("Machine input b: {}", machine_input_b);

    let stack_inputs = StackInputs::try_from_ints([machine_input_a, machine_input_b]).unwrap();
    let cloned_inputs = stack_inputs.clone();

    let host = DefaultHost::default();

    // Execute the program and generate a STARK proof
    let (outputs, proof) = prove(&program, stack_inputs, host, ProvingOptions::default())
        .expect("Failed to execute the program and generate a proof");

    println!("Stack output:");
    println!("{:?}", outputs.stack());

    let raw_result = outputs.stack().get(0).unwrap().as_int();
    let result = to_normal_format(raw_result as u128) as i64;

    let expected_result = input_a + input_b;

    println!("raw_result: {}, result: {}", raw_result, result);
    println!("Expected result: {}", expected_result);

    assert_eq!(result, expected_result);

    verify(program.into(), cloned_inputs, outputs, proof).unwrap();
    println!("Program run successfully");
}
