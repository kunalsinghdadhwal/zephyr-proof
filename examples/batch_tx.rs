//! Batch Transaction Proof Example
//!
//! Demonstrates proving multiple EVM operations in parallel using the zkEVM prover.
//! This example creates multiple traces and generates proofs concurrently.

use colored::Colorize;
use tokio::task::JoinSet;
use zephyr_proof::{generate_proof, verify_proof, ProverConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Batch Transaction Proof Example".cyan().bold());
    println!("===================================\n");

    // Define multiple traces to prove in parallel
    let traces = vec![
        (
            "ADD Operation (5 + 3)",
            r#"{
                "opcodes": [96, 96, 1],
                "stack_states": [[5, 0, 0], [3, 5, 0], [8, 0, 0]],
                "pcs": [0, 2, 4],
                "gas_values": [1000, 997, 994],
                "memory_ops": null,
                "storage_ops": null,
                "tx_hash": "0xbatch_add",
                "block_number": 1,
                "bytecode": [96, 5, 96, 3, 1]
            }"#,
        ),
        (
            "MUL Operation (7 * 6)",
            r#"{
                "opcodes": [96, 96, 2],
                "stack_states": [[7, 0, 0], [6, 7, 0], [42, 0, 0]],
                "pcs": [0, 2, 4],
                "gas_values": [1000, 997, 992],
                "memory_ops": null,
                "storage_ops": null,
                "tx_hash": "0xbatch_mul",
                "block_number": 2,
                "bytecode": [96, 7, 96, 6, 2]
            }"#,
        ),
        (
            "SUB Operation (20 - 8)",
            r#"{
                "opcodes": [96, 96, 3],
                "stack_states": [[20, 0, 0], [8, 20, 0], [12, 0, 0]],
                "pcs": [0, 2, 4],
                "gas_values": [1000, 997, 994],
                "memory_ops": null,
                "storage_ops": null,
                "tx_hash": "0xbatch_sub",
                "block_number": 3,
                "bytecode": [96, 20, 96, 8, 3]
            }"#,
        ),
        (
            "Complex Operation (PUSH, PUSH, ADD, PUSH, MUL)",
            r#"{
                "opcodes": [96, 96, 1, 96, 2],
                "stack_states": [
                    [10, 0, 0],
                    [5, 10, 0],
                    [15, 0, 0],
                    [2, 15, 0],
                    [30, 0, 0]
                ],
                "pcs": [0, 2, 4, 6, 8],
                "gas_values": [1000, 997, 994, 991, 986],
                "memory_ops": null,
                "storage_ops": null,
                "tx_hash": "0xbatch_complex",
                "block_number": 4,
                "bytecode": [96, 10, 96, 5, 1, 96, 2, 2]
            }"#,
        ),
    ];

    println!("Traces to prove: {}\n", traces.len());

    // Configure prover
    let config = ProverConfig {
        k: 12, // 2^12 = 4096 rows
        parallel: true,
        num_threads: Some(4),
        rpc_url: None,
    };

    println!("{}", "Prover Configuration:".cyan());
    println!("  Circuit size: 2^{} = {} rows", config.k, 1 << config.k);
    println!("  Parallel: {}", config.parallel);
    println!("  Threads: {}\n", config.num_threads.unwrap_or(0));

    // Generate proofs in parallel using JoinSet
    println!("{}", "Generating proofs in parallel...".cyan());
    let start = std::time::Instant::now();

    let mut join_set = JoinSet::new();
    for (name, trace_json) in traces.into_iter() {
        let config_clone = config.clone();
        let trace_json = trace_json.to_string();
        let name = name.to_string();

        join_set.spawn(async move {
            let result = generate_proof(&trace_json, &config_clone).await;
            (name, result)
        });
    }

    // Collect results
    let mut proofs = Vec::new();
    let mut total_opcodes = 0;
    let mut total_gas = 0;

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((name, Ok(proof))) => {
                println!(
                    "  {} - {} opcodes, {} gas",
                    name.green(),
                    proof.metadata.opcode_count,
                    proof.metadata.gas_used
                );
                total_opcodes += proof.metadata.opcode_count;
                total_gas += proof.metadata.gas_used;
                proofs.push((name, proof));
            }
            Ok((name, Err(e))) => {
                println!("  {} - Error: {}", name.red(), e);
            }
            Err(e) => {
                println!("  {} {}", "Task error:".red(), e);
            }
        }
    }

    let batch_duration = start.elapsed();
    println!(
        "\n{}",
        format!("All proofs generated in {:?}", batch_duration)
            .green()
            .bold()
    );
    println!("Total opcodes: {}", total_opcodes);
    println!("Total gas: {}\n", total_gas);

    // Verify all proofs in parallel
    println!("{}", "Verifying all proofs...".cyan());
    let start = std::time::Instant::now();

    let mut verify_set = JoinSet::new();
    for (name, proof) in proofs.iter() {
        let config_clone = config.clone();
        let proof_clone = proof.clone();
        let name = name.clone();

        verify_set.spawn(async move {
            let result = verify_proof(&proof_clone, &config_clone).await;
            (name, result)
        });
    }

    let mut all_valid = true;
    while let Some(result) = verify_set.join_next().await {
        match result {
            Ok((name, Ok(valid))) => {
                if valid {
                    println!("  {} - {}", name, "VALID".green());
                } else {
                    println!("  {} - {}", name, "INVALID".red());
                    all_valid = false;
                }
            }
            Ok((name, Err(e))) => {
                println!("  {} - Error: {}", name.red(), e);
                all_valid = false;
            }
            Err(e) => {
                println!("  {} {}", "Verification error:".red(), e);
                all_valid = false;
            }
        }
    }

    let verify_duration = start.elapsed();
    println!(
        "\n{}",
        format!("All proofs verified in {:?}", verify_duration)
            .green()
            .bold()
    );

    if !all_valid {
        println!("{}", "Some proofs failed verification!".red().bold());
        std::process::exit(1);
    }

    // Calculate statistics
    println!("\n{}", "Performance Statistics:".cyan());
    println!(
        "  Average proof time: {:?}",
        batch_duration / proofs.len() as u32
    );
    println!(
        "  Average verify time: {:?}",
        verify_duration / proofs.len() as u32
    );
    println!(
        "  Throughput: {:.2} proofs/second",
        proofs.len() as f64 / batch_duration.as_secs_f64()
    );

    // Save all proofs to files
    println!("\nSaving proofs...");
    for (name, proof) in &proofs {
        let filename = format!(
            "batch_proof_{}.json",
            name.to_lowercase()
                .replace(" ", "_")
                .replace("(", "")
                .replace(")", "")
        );
        let proof_json = serde_json::to_string_pretty(&proof)?;
        std::fs::write(&filename, proof_json)?;
        println!("  Saved: {}", filename);
    }

    println!(
        "\n{}",
        format!("{} proofs generated and verified", proofs.len())
            .green()
            .bold()
    );

    Ok(())
}
