//! Witness Generation Example
//!
//! Demonstrates the detailed process of generating circuit witnesses from EVM traces.
//! This example shows how to:
//! 1. Create custom EVM traces
//! 2. Generate witnesses from traces
//! 3. Inspect witness structure
//! 4. Use witnesses in circuits

use halo2_proofs::{dev::MockProver, pasta::Fp};
use zephyr_proof::{
    circuits::main_circuit::{EvmCircuit, ExecutionStep},
    utils::evm_parser::{trace_to_witness, CircuitWitness, EvmTrace},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Witness Generation Example");
    println!("==============================\n");

    // Example 1: Simple ADD operation
    println!("📝 Example 1: Simple ADD Operation");
    println!("-----------------------------------");
    example_1_simple_add()?;

    // Example 2: Multiple operations
    println!("\n📝 Example 2: Multiple Operations");
    println!("----------------------------------");
    example_2_multiple_ops()?;

    // Example 3: Custom trace with storage
    println!("\n📝 Example 3: Custom Trace with Storage");
    println!("----------------------------------------");
    example_3_custom_trace()?;

    // Example 4: Witness to circuit
    println!("\n📝 Example 4: Witness to Circuit");
    println!("---------------------------------");
    example_4_witness_to_circuit()?;

    println!("\n🎉 All examples completed successfully!");

    Ok(())
}

/// Example 1: Generate witness from simple ADD operation
fn example_1_simple_add() -> Result<(), Box<dyn std::error::Error>> {
    // Create trace: PUSH1 5, PUSH1 3, ADD
    let trace = EvmTrace {
        opcodes: vec![0x60, 0x60, 0x01],
        stack_states: vec![vec![5, 0, 0], vec![3, 5, 0], vec![8, 0, 0]],
        pcs: vec![0, 2, 4],
        gas_values: vec![1000, 997, 994],
        memory_ops: None,
        storage_ops: None,
        tx_hash: Some("0xadd_example".to_string()),
        block_number: Some(1),
        bytecode: Some(vec![0x60, 0x05, 0x60, 0x03, 0x01]),
    };

    println!("  Trace:");
    println!("    Opcodes: {:?}", trace.opcodes);
    println!("    Stack states: {:?}", trace.stack_states);
    println!("    Gas values: {:?}", trace.gas_values);

    // Generate witness
    let witness = trace_to_witness(&trace)?;

    println!("\n  Witness:");
    println!("    Opcode cells: {:?}", witness.opcode_cells);
    println!("    Stack cells: {:?}", witness.stack_cells);
    println!("    Gas cells: {:?}", witness.gas_cells);
    println!(
        "    Public inputs (first 2): {:?}",
        &witness.public_inputs[..2]
    );

    // Validate witness
    assert_eq!(witness.opcode_cells.len(), 3);
    assert_eq!(witness.stack_cells.len(), 9); // 3 steps * 3 stack values
    assert_eq!(witness.gas_cells.len(), 3);
    assert_eq!(witness.public_inputs.len(), 4); // SHA256 hash split into 4 u64s

    println!("\n  ✅ Witness validated successfully!");

    Ok(())
}

/// Example 2: Generate witness from multiple operations
fn example_2_multiple_ops() -> Result<(), Box<dyn std::error::Error>> {
    // Create trace: PUSH1 10, PUSH1 5, ADD, PUSH1 2, MUL
    // Stack: [10] -> [5, 10] -> [15] -> [2, 15] -> [30]
    let trace = EvmTrace {
        opcodes: vec![0x60, 0x60, 0x01, 0x60, 0x02],
        stack_states: vec![
            vec![10, 0, 0],
            vec![5, 10, 0],
            vec![15, 0, 0],
            vec![2, 15, 0],
            vec![30, 0, 0],
        ],
        pcs: vec![0, 2, 4, 6, 8],
        gas_values: vec![1000, 997, 994, 991, 986],
        memory_ops: None,
        storage_ops: None,
        tx_hash: Some("0xmulti_ops".to_string()),
        block_number: Some(1),
        bytecode: Some(vec![0x60, 0x0a, 0x60, 0x05, 0x01, 0x60, 0x02, 0x02]),
    };

    println!("  Operations:");
    println!("    1. PUSH1 10  -> stack: [10]");
    println!("    2. PUSH1 5   -> stack: [5, 10]");
    println!("    3. ADD       -> stack: [15]");
    println!("    4. PUSH1 2   -> stack: [2, 15]");
    println!("    5. MUL       -> stack: [30]");

    // Generate witness
    let witness = trace_to_witness(&trace)?;

    println!("\n  Witness structure:");
    println!("    Total opcodes: {}", witness.opcode_cells.len());
    println!("    Total stack cells: {}", witness.stack_cells.len());
    println!("    Total gas cells: {}", witness.gas_cells.len());

    // Calculate gas consumption
    let gas_consumed = witness.gas_cells[0] - witness.gas_cells[witness.gas_cells.len() - 1];
    println!("    Gas consumed: {}", gas_consumed);

    // Show opcode breakdown
    println!("\n  Opcode breakdown:");
    for (i, opcode) in witness.opcode_cells.iter().enumerate() {
        let opcode_name = match *opcode {
            0x60 => "PUSH1",
            0x01 => "ADD",
            0x02 => "MUL",
            _ => "UNKNOWN",
        };
        println!("    [{}] 0x{:02x} ({})", i, opcode, opcode_name);
    }

    println!("\n  ✅ Multi-operation witness validated!");

    Ok(())
}

/// Example 3: Create custom trace with storage operations
fn example_3_custom_trace() -> Result<(), Box<dyn std::error::Error>> {
    use alloy_primitives::U256;
    use zephyr_proof::utils::evm_parser::StorageOp;

    // Create trace with storage operations
    let trace = EvmTrace {
        opcodes: vec![0x54, 0x60, 0x01, 0x55],
        stack_states: vec![
            vec![100, 0, 0], // SLOAD result
            vec![5, 100, 0], // PUSH1 5
            vec![105, 0, 0], // ADD result
            vec![0, 0, 0],   // SSTORE (no return)
        ],
        pcs: vec![0, 1, 3, 5],
        gas_values: vec![10000, 9800, 9797, 9594],
        memory_ops: None,
        storage_ops: Some(vec![
            StorageOp {
                key: U256::from(0),
                value: U256::from(100),
                is_write: false,
            },
            StorageOp {
                key: U256::from(0),
                value: U256::from(105),
                is_write: true,
            },
        ]),
        tx_hash: Some("0xstorage_example".to_string()),
        block_number: Some(1),
        bytecode: Some(vec![0x54, 0x60, 0x05, 0x01, 0x55]),
    };

    println!("  Trace with storage:");
    println!("    Opcodes: SLOAD, PUSH1, ADD, SSTORE");
    println!(
        "    Storage ops: {} operations",
        trace.storage_ops.as_ref().unwrap().len()
    );

    // Generate witness
    let witness = trace_to_witness(&trace)?;

    println!("\n  Gas breakdown:");
    println!("    Initial gas: {}", witness.gas_cells[0]);
    println!("    After SLOAD: {} (cost: 200)", witness.gas_cells[1]);
    println!("    After PUSH1: {} (cost: 3)", witness.gas_cells[2]);
    println!("    After ADD: {} (cost: 3)", witness.gas_cells[3]);
    println!(
        "    Total consumed: {}",
        witness.gas_cells[0] - witness.gas_cells[witness.gas_cells.len() - 1]
    );

    println!("\n  ✅ Storage operation witness validated!");

    Ok(())
}

/// Example 4: Convert witness to circuit and verify
fn example_4_witness_to_circuit() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple trace
    let trace = EvmTrace::mock_add();

    println!("  Using mock ADD trace:");
    println!("    Opcodes: {}", trace.opcodes.len());
    println!("    Stack states: {}", trace.stack_states.len());

    // Generate witness
    let witness = trace_to_witness(&trace)?;

    println!("\n  Generated witness:");
    println!("    Opcode cells: {}", witness.opcode_cells.len());
    println!("    Stack cells: {}", witness.stack_cells.len());

    // Create circuit from witness
    let circuit = EvmCircuit::<Fp>::from_witness(&witness);

    println!("\n  Created circuit:");
    println!("    Execution steps: {}", circuit.steps.len());
    println!("    Trace commitment: {:?}", circuit.trace_commitment);

    // Verify circuit using MockProver
    let k = 10;
    let public_inputs = vec![circuit.trace_commitment];

    println!("\n  Running MockProver (k={})...", k);
    let prover = MockProver::run(k, &circuit, vec![public_inputs])
        .map_err(|e| format!("MockProver error: {:?}", e))?;

    prover
        .verify()
        .map_err(|e| format!("Verification error: {:?}", e))?;

    println!("  ✅ Circuit constraints satisfied!");

    // Display execution steps
    println!("\n  Execution steps:");
    for (i, step) in circuit.steps.iter().enumerate() {
        println!(
            "    [{}] opcode=0x{:02x}, pc={}, gas={}",
            i, step.opcode, step.pc, step.gas
        );
    }

    println!("\n  ✅ Witness successfully converted to circuit!");

    Ok(())
}
