//! Simple ADD operation proof example
//!
//! Demonstrates proving a basic ADD operation using the zkEVM prover.
//! This example creates a mock trace with PUSH1, PUSH1, ADD opcodes and generates a proof.

use zephyr_proof::{generate_proof, verify_proof, ProverConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Simple ADD Operation Proof Example");
    println!("=====================================\n");

    // Create a simple ADD trace: PUSH1 5, PUSH1 3, ADD
    // Opcodes: 0x60 (PUSH1), 0x60 (PUSH1), 0x01 (ADD)
    let trace_json = r#"{
        "opcodes": [96, 96, 1],
        "stack_states": [
            [5, 0, 0],
            [3, 5, 0],
            [8, 0, 0]
        ],
        "pcs": [0, 2, 4],
        "gas_values": [1000, 997, 994],
        "memory_ops": null,
        "storage_ops": null,
        "tx_hash": "0xexample_add",
        "block_number": 1,
        "bytecode": [96, 5, 96, 3, 1]
    }"#;

    println!("📋 Trace:");
    println!("  PUSH1 5    (opcode: 0x60)");
    println!("  PUSH1 3    (opcode: 0x60)");
    println!("  ADD        (opcode: 0x01)");
    println!("  Result: 8\n");

    // Configure prover
    let config = ProverConfig {
        k: 10, // Small circuit for this example (2^10 = 1024 rows)
        parallel: true,
        num_threads: Some(2),
        rpc_url: None,
    };

    println!("⚙️  Prover Configuration:");
    println!("  Circuit size: 2^{} = {} rows", config.k, 1 << config.k);
    println!("  Parallel: {}", config.parallel);
    println!("  Threads: {}\n", config.num_threads.unwrap_or(0));

    // Generate proof
    println!("🔨 Generating proof...");
    let start = std::time::Instant::now();
    let proof = generate_proof(trace_json, &config).await?;
    let duration = start.elapsed();

    println!("✅ Proof generated in {:?}\n", duration);

    // Display proof metadata
    println!("📊 Proof Metadata:");
    println!("  Opcodes: {}", proof.metadata.opcode_count);
    println!("  Gas used: {}", proof.metadata.gas_used);
    println!("  Proof size: {} bytes", proof.proof.len());
    println!("  VK hash: {}", proof.vk_hash);
    println!("  Public inputs: {}", proof.public_inputs.len());
    for (i, input) in proof.public_inputs.iter().enumerate() {
        println!("    [{}] {}", i, input);
    }
    println!();

    // Verify proof
    println!("🔍 Verifying proof...");
    let start = std::time::Instant::now();
    let valid = verify_proof(&proof, &config).await?;
    let duration = start.elapsed();

    if valid {
        println!("✅ Proof is VALID! (verified in {:?})", duration);
    } else {
        println!("❌ Proof is INVALID!");
        std::process::exit(1);
    }

    // Save proof to file
    let proof_json = serde_json::to_string_pretty(&proof)?;
    std::fs::write("simple_add_proof.json", proof_json)?;
    println!("\n💾 Proof saved to: simple_add_proof.json");

    println!("\n🎉 Example completed successfully!");

    Ok(())
}
