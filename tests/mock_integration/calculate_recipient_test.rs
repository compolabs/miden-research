use miden_objects::{
    assembly::{AssemblyContext, ProgramAst},
    notes::{NoteInputs, NoteRecipient, NoteScript},
    vm::CodeBlock,
    Digest, Felt, Hasher, NoteError,
};
use miden_vm::{prove, verify, Assembler, DefaultHost, ProvingOptions, StackInputs};

pub fn new_note_script(
    code: ProgramAst,
    assembler: &Assembler,
) -> Result<(NoteScript, CodeBlock), NoteError> {
    // Compile the code in the context with phantom calls enabled
    let code_block = assembler
        .compile_in_context(
            &code,
            &mut AssemblyContext::for_program(Some(&code)).with_phantom_calls(true),
        )
        .map_err(NoteError::ScriptCompilationError)?;

    // Use the from_parts method to create a NoteScript instance
    let note_script = NoteScript::from_parts(code, code_block.hash());

    Ok((note_script, code_block))
}

#[test]
fn test__get_recipient_hash() {
    // Instantiate the assembler
    let assembler = Assembler::default().with_debug_mode(true);

    // Read the assembly program from a file
    let assembly_code: &str = include_str!("../../src/hash/rpo_test.masm");

    // Compile the program from the loaded assembly code
    let program = assembler
        .compile(assembly_code)
        .expect("Failed to compile the assembly code");

    let stack_inputs = StackInputs::try_from_ints([]).unwrap();
    let cloned_inputs = stack_inputs.clone();

    let host = DefaultHost::default();

    // Execute the program and generate a STARK proof
    let (outputs, proof) = prove(&program, stack_inputs, host, ProvingOptions::default())
        .expect("Failed to execute the program and generate a proof");

    println!("Stack Output: {:?}", outputs.stack());

    let inputs = NoteInputs::new(vec![Felt::new(2)]).unwrap();

    println!("Inputs Hash: {:?}", inputs.commitment());

    let serial_num = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let serial_num_hash = Hasher::merge(&[serial_num.into(), Digest::default()]);

    println!("Serial Number Hash: {:?}", serial_num_hash);

    let note_script_code = include_str!("../../src/hash/basic_note.masm");
    let note_script = new_note_script(ProgramAst::parse(note_script_code).unwrap(), &assembler)
        .unwrap()
        .0;

    let note_script_hash: Digest = note_script.hash();

    println!("Note Script Hash: {:?}", note_script_hash);

    let serial_script_hash = Hasher::merge(&[serial_num_hash, note_script_hash]);

    println!("Serial Script Hash: {:?}", serial_script_hash);

    // let recipient_1 = Hasher::merge(&[serial_script_hash, inputs.commitment()]);
    // println!("recipient_1: {:?}", recipient_1);
    let recipient = NoteRecipient::new(serial_num, note_script, inputs);
    print!("Recipient: {:?}", recipient.digest());

    verify(program.into(), cloned_inputs, outputs, proof).unwrap();
    println!("Program run successfully");
}

#[test]
fn test_recipient_hash_proc() {
    // Instantiate the assembler
    let assembler = Assembler::default().with_debug_mode(true);

    // Read the assembly program from a file
    let assembly_code: &str = include_str!("../../src/hash/recipient_hash_proc.masm");

    // Compile the program from the loaded assembly code
    let program = assembler
        .compile(assembly_code)
        .expect("Failed to compile the assembly code");

    let stack_inputs = StackInputs::try_from_ints([]).unwrap();
    let cloned_inputs = stack_inputs.clone();

    let host = DefaultHost::default();

    // Execute the program and generate a STARK proof
    let (outputs, proof) = prove(&program, stack_inputs, host, ProvingOptions::default())
        .expect("Failed to execute the program and generate a proof");

    let inputs = NoteInputs::new(vec![Felt::new(2)]).unwrap();

    let serial_num = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let serial_num_hash = Hasher::merge(&[serial_num.into(), Digest::default()]);

    let note_script_code = include_str!("../../src/hash/basic_note.masm");
    let note_script = new_note_script(ProgramAst::parse(note_script_code).unwrap(), &assembler)
        .unwrap()
        .0;

    let note_script_hash: Digest = note_script.hash();

    let serial_script_hash = Hasher::merge(&[serial_num_hash, note_script_hash]);

    let recipient_1 = Hasher::merge(&[serial_script_hash, inputs.commitment()]);
    
    let recipient = NoteRecipient::new(serial_num, note_script, inputs);
    
    println!("Stack Output: {:?}", outputs.stack());
    print!("Recipient: {:?}", recipient.digest());

    assert_eq!(recipient_1, recipient.digest());

    verify(program.into(), cloned_inputs, outputs, proof).unwrap();
}
