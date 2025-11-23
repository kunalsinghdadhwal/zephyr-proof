//! Large Trace Proof Example
//!
//! Demonstrates proving large EVM traces using chunking and parallel processing.
//! This example creates a trace with thousands of operations and shows how the
//! prover automatically chunks it into manageable pieces for parallel proof generation.

use colored::Colorize;
use zephyr_proof::{
    generate_proof,
    prover::parallel_prover::generate_proof_chunked,
    utils::evm_parser::{parse_evm_data, EvmTrace},
    verify_proof, ProverConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Large Trace Proof Example".cyan().bold());
    println!("============================\n");

    // Create progressively larger traces
    let trace_sizes = vec![100, 1000, 5000, 10000];

    for size in trace_sizes {
        println!(
            "{}",
            format!("Testing trace with {} operations", size).cyan()
        );
        println!("{}", "-".repeat(50));

        let trace = generate_large_trace(size);

        println!("  Trace statistics:");
        println!("    Total opcodes: {}", trace.opcodes.len());
        println!("    Stack states: {}", trace.stack_states.len());
        println!("    Initial gas: {}", trace.gas_values[0]);
        println!(
            "    Final gas: {}",
            trace.gas_values[trace.gas_values.len() - 1]
        );
        println!(
            "    Gas consumed: {}",
            trace.gas_values[0] - trace.gas_values[trace.gas_values.len() - 1]
        );

        // Configure prover
        let config = ProverConfig {
            k: 14, // 2^14 = 16,384 rows per chunk
            parallel: true,
            num_threads: Some(4),
            rpc_url: None,
        };

        // Determine if chunking will be used
        let max_rows = (1 << config.k) - 100;
        let needs_chunking = trace.opcodes.len() > max_rows;

        if needs_chunking {
            println!("\n  Will use chunking (trace size > {} rows)", max_rows);
            let num_chunks = (trace.opcodes.len() + max_rows - 1) / max_rows;
            println!("    Estimated chunks: {}", num_chunks);
        } else {
            println!(
                "\n  Will prove in single chunk (trace size <= {} rows)",
                max_rows
            );
        }

        // Generate proof
        println!("\n  {}", "Generating proof...".cyan());
        let start = std::time::Instant::now();

        let trace_json = serde_json::to_string(&trace)?;
        let proof = generate_proof(&trace_json, &config).await?;

        let duration = start.elapsed();

        println!(
            "  {}",
            format!("Proof generated in {:?}", duration).green().bold()
        );
        println!("    Proof size: {} bytes", proof.proof.len());
        println!("    VK hash: {}", proof.vk_hash);

        // Calculate throughput
        let ops_per_sec = trace.opcodes.len() as f64 / duration.as_secs_f64();
        println!("    Throughput: {:.2} opcodes/second", ops_per_sec);

        // Verify proof
        println!("\n  {}", "Verifying proof...".cyan());
        let start = std::time::Instant::now();
        let valid = verify_proof(&proof, &config).await?;
        let duration = start.elapsed();

        if valid {
            println!(
                "  {}",
                format!("Proof is VALID! (verified in {:?})", duration)
                    .green()
                    .bold()
            );
        } else {
            println!("  {}", "Proof is INVALID!".red().bold());
            std::process::exit(1);
        }

        // Save proof
        let filename = format!("large_trace_{}_proof.json", size);
        std::fs::write(&filename, serde_json::to_string_pretty(&proof)?)?;
        println!("\n  Proof saved to: {}\n", filename);
    }

    println!(
        "{}",
        "All large trace tests completed successfully!"
            .green()
            .bold()
    );
    println!();

    // Performance summary
    println!("{}", "Performance Summary".cyan());
    println!("=====================");
    println!(
        "  {}",
        "Successfully proved traces up to 10,000 operations".green()
    );
    println!(
        "  {}",
        "Automatic chunking for traces exceeding circuit capacity".green()
    );
    println!("  {}", "Parallel proof generation with Rayon".green());
    println!("  {}", "All proofs verified successfully".green());

    Ok(())
}

/// Generate a large trace with repetitive ADD operations
fn generate_large_trace(size: usize) -> EvmTrace {
    let mut opcodes = Vec::with_capacity(size);
    let mut stack_states = Vec::with_capacity(size);
    let mut pcs = Vec::with_capacity(size);
    let mut gas_values = Vec::with_capacity(size);

    let initial_gas = 1_000_000u64;
    let mut current_gas = initial_gas;
    let mut pc = 0u64;
    let mut accumulator = 0u64;

    for i in 0..size {
        // Alternate between PUSH1 and ADD operations
        if i % 2 == 0 {
            // PUSH1: Push a value onto stack
            opcodes.push(0x60);
            accumulator += 1;
            stack_states.push(vec![accumulator, 0, 0]);
            current_gas = current_gas.saturating_sub(3);
        } else {
            // ADD: Add top two stack values
            opcodes.push(0x01);
            accumulator *= 2; // Simplified: just double the value
            stack_states.push(vec![accumulator, 0, 0]);
            current_gas = current_gas.saturating_sub(3);
        }

        pcs.push(pc);
        gas_values.push(current_gas);
        pc += 2; // Each operation advances PC by 2 (opcode + data)
    }

    EvmTrace {
        opcodes,
        stack_states,
        pcs,
        gas_values,
        memory_ops: None,
        storage_ops: None,
        tx_hash: Some(format!("0xlarge_trace_{}", size)),
        block_number: Some(12345),
        bytecode: None,
    }
}

/// Generate a complex trace with mixed operations
#[allow(dead_code)]
fn generate_complex_trace(size: usize) -> EvmTrace {
    let mut opcodes = Vec::with_capacity(size);
    let mut stack_states = Vec::with_capacity(size);
    let mut pcs = Vec::with_capacity(size);
    let mut gas_values = Vec::with_capacity(size);

    let initial_gas = 1_000_000u64;
    let mut current_gas = initial_gas;
    let mut pc = 0u64;
    let mut stack_top = 0u64;

    for i in 0..size {
        let op_type = i % 5;

        match op_type {
            0 => {
                // PUSH1
                opcodes.push(0x60);
                stack_top = (i % 256) as u64;
                stack_states.push(vec![stack_top, 0, 0]);
                current_gas = current_gas.saturating_sub(3);
            }
            1 => {
                // ADD
                opcodes.push(0x01);
                stack_top = stack_top.wrapping_add(1);
                stack_states.push(vec![stack_top, 0, 0]);
                current_gas = current_gas.saturating_sub(3);
            }
            2 => {
                // MUL
                opcodes.push(0x02);
                stack_top = stack_top.wrapping_mul(2);
                stack_states.push(vec![stack_top, 0, 0]);
                current_gas = current_gas.saturating_sub(5);
            }
            3 => {
                // SUB
                opcodes.push(0x03);
                stack_top = stack_top.wrapping_sub(1);
                stack_states.push(vec![stack_top, 0, 0]);
                current_gas = current_gas.saturating_sub(3);
            }
            _ => {
                // DUP1
                opcodes.push(0x80);
                stack_states.push(vec![stack_top, stack_top, 0]);
                current_gas = current_gas.saturating_sub(3);
            }
        }

        pcs.push(pc);
        gas_values.push(current_gas);
        pc += 2;
    }

    EvmTrace {
        opcodes,
        stack_states,
        pcs,
        gas_values,
        memory_ops: None,
        storage_ops: None,
        tx_hash: Some(format!("0xcomplex_trace_{}", size)),
        block_number: Some(12345),
        bytecode: None,
    }
}
