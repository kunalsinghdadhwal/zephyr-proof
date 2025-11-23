//! Real Transaction Proof Example
//!
//! Demonstrates fetching and proving real Ethereum transactions from a network.
//! This example connects to an RPC endpoint, fetches a transaction, generates a witness,
//! and creates a zero-knowledge proof of the execution.
//!
//! Usage:
//!   cargo run --example real_transaction
//!
//! Note: Requires a running Ethereum node or RPC endpoint (e.g., local Anvil, Infura, etc.)

use zephyr_proof::{
    prove_transaction,
    utils::evm_parser::{fetch_and_execute_tx, trace_to_witness, EvmTrace},
    verify_proof, ProverConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 Real Transaction Proof Example");
    println!("==================================\n");

    // Configuration
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| {
        println!("⚠️  RPC_URL not set, using default: http://localhost:8545");
        "http://localhost:8545".to_string()
    });

    let tx_hash = std::env::var("TX_HASH").unwrap_or_else(|_| {
        println!("⚠️  TX_HASH not set, using mock trace instead");
        String::new()
    });

    println!("⚙️  Configuration:");
    println!("  RPC URL: {}", rpc_url);
    if !tx_hash.is_empty() {
        println!("  TX Hash: {}\n", tx_hash);
    } else {
        println!("  Using mock trace (no TX_HASH provided)\n");
    }

    // Configure prover
    let config = ProverConfig {
        k: 14, // 2^14 = 16,384 rows (suitable for real transactions)
        parallel: true,
        num_threads: None, // Auto-detect
        rpc_url: Some(rpc_url.clone()),
    };

    println!("🔨 Prover Configuration:");
    println!("  Circuit size: 2^{} = {} rows", config.k, 1 << config.k);
    println!("  Parallel: {}", config.parallel);
    println!(
        "  Threads: {}\n",
        config.num_threads.unwrap_or_else(|| num_cpus::get())
    );

    // Fetch and prove transaction
    if !tx_hash.is_empty() {
        println!("📡 Fetching transaction from network...");

        match fetch_and_execute_tx(&tx_hash, &rpc_url).await {
            Ok((trace, gas_used)) => {
                println!("✅ Transaction fetched successfully!");
                println!("  Opcodes: {}", trace.opcodes.len());
                println!("  Gas used: {}", gas_used);
                if let Some(block) = trace.block_number {
                    println!("  Block number: {}", block);
                }
                println!();

                // Display first few opcodes
                println!("📋 First 10 opcodes:");
                for (i, opcode) in trace.opcodes.iter().take(10).enumerate() {
                    println!("  [{}] 0x{:02x}", i, opcode);
                }
                if trace.opcodes.len() > 10 {
                    println!("  ... ({} more)", trace.opcodes.len() - 10);
                }
                println!();

                // Generate witness
                println!("🔬 Generating witness...");
                let witness = trace_to_witness(&trace)?;
                println!("✅ Witness generated!");
                println!("  Opcode cells: {}", witness.opcode_cells.len());
                println!("  Stack cells: {}", witness.stack_cells.len());
                println!("  Gas cells: {}", witness.gas_cells.len());
                println!("  Public inputs: {}", witness.public_inputs.len());
                println!();

                // Generate proof
                println!("🔨 Generating proof...");
                let start = std::time::Instant::now();
                let proof = prove_transaction(&tx_hash, &rpc_url, &config).await?;
                let duration = start.elapsed();

                println!("✅ Proof generated in {:?}\n", duration);

                // Display proof metadata
                println!("📊 Proof Metadata:");
                println!("  Transaction: {}", tx_hash);
                if let Some(block) = proof.metadata.block_number {
                    println!("  Block: {}", block);
                }
                println!("  Opcodes: {}", proof.metadata.opcode_count);
                println!("  Gas used: {}", proof.metadata.gas_used);
                println!("  Proof size: {} bytes", proof.proof.len());
                println!("  VK hash: {}", proof.vk_hash);
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

                // Save proof
                let filename = format!("real_tx_{}.json", &tx_hash[2..12]);
                let proof_json = serde_json::to_string_pretty(&proof)?;
                std::fs::write(&filename, proof_json)?;
                println!("\n💾 Proof saved to: {}", filename);
            }
            Err(e) => {
                println!("❌ Failed to fetch transaction: {}", e);
                println!("\n💡 Tip: Make sure:");
                println!("  1. RPC endpoint is running and accessible");
                println!("  2. Transaction hash is valid and exists");
                println!("  3. Network connectivity is working\n");

                println!("📝 Falling back to mock trace example...\n");
                run_mock_example(&config).await?;
            }
        }
    } else {
        println!("📝 No TX_HASH provided, running mock example...\n");
        run_mock_example(&config).await?;
    }

    println!("\n🎉 Example completed successfully!");
    println!("\n💡 To prove a real transaction, set environment variables:");
    println!("   export RPC_URL=http://localhost:8545");
    println!("   export TX_HASH=0x1234567890abcdef...");
    println!("   cargo run --example real_transaction");

    Ok(())
}

/// Run example with mock trace when no real transaction is available
async fn run_mock_example(config: &ProverConfig) -> Result<(), Box<dyn std::error::Error>> {
    use zephyr_proof::generate_proof;

    println!("🧪 Using mock ADD trace (PUSH1 10, PUSH1 20, ADD)");

    // Create mock trace
    let trace = EvmTrace::mock_add();
    let trace_json = serde_json::to_string(&trace)?;

    println!("  Opcodes: {}", trace.opcodes.len());
    println!("  Gas values: {:?}", trace.gas_values);
    println!();

    // Generate proof
    println!("🔨 Generating proof from mock trace...");
    let start = std::time::Instant::now();
    let proof = generate_proof(&trace_json, config).await?;
    let duration = start.elapsed();

    println!("✅ Proof generated in {:?}\n", duration);

    // Display proof metadata
    println!("📊 Proof Metadata:");
    println!("  Opcodes: {}", proof.metadata.opcode_count);
    println!("  Gas used: {}", proof.metadata.gas_used);
    println!("  Proof size: {} bytes", proof.proof.len());
    println!();

    // Verify proof
    println!("🔍 Verifying proof...");
    let start = std::time::Instant::now();
    let valid = verify_proof(&proof, config).await?;
    let duration = start.elapsed();

    if valid {
        println!("✅ Proof is VALID! (verified in {:?})", duration);
    } else {
        println!("❌ Proof is INVALID!");
        std::process::exit(1);
    }

    // Save proof
    std::fs::write(
        "mock_transaction_proof.json",
        serde_json::to_string_pretty(&proof)?,
    )?;
    println!("\n💾 Proof saved to: mock_transaction_proof.json");

    Ok(())
}
