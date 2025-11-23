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

use colored::Colorize;
use zephyr_proof::{
    prove_transaction,
    utils::evm_parser::{fetch_and_execute_tx, trace_to_witness},
    verify_proof, ProverConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Real Transaction Proof Example".cyan().bold());
    println!("==================================\n");

    // Configuration
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| {
        println!(
            "{}",
            "RPC_URL not set, using default: http://localhost:8545".yellow()
        );
        "http://localhost:8545".to_string()
    });

    let tx_hash = std::env::var("TX_HASH").unwrap_or_else(|_| {
        println!("{}", "TX_HASH not set".yellow());
        String::new()
    });

    println!("Configuration:");
    println!("  RPC URL: {}", rpc_url);
    if !tx_hash.is_empty() {
        println!("  TX Hash: {}\n", tx_hash);
    } else {
        println!(
            "{}",
            "  No TX_HASH provided - please set environment variable\n".red()
        );
        println!("To prove a real transaction, set environment variables:");
        println!("   export RPC_URL=http://localhost:8545");
        println!("   export TX_HASH=0x1234567890abcdef...");
        println!("   cargo run --example real_transaction");
        std::process::exit(1);
    }

    // Configure prover
    let config = ProverConfig {
        k: 14, // 2^14 = 16,384 rows (suitable for real transactions)
        parallel: true,
        num_threads: None, // Auto-detect
        rpc_url: Some(rpc_url.clone()),
    };

    println!("{}", "Prover Configuration:".cyan());
    println!("  Circuit size: 2^{} = {} rows", config.k, 1 << config.k);
    println!("  Parallel: {}", config.parallel);
    println!(
        "  Threads: {}\n",
        config.num_threads.unwrap_or_else(|| num_cpus::get())
    );

    // Fetch and prove transaction
    println!("{}", "Fetching transaction from network...".cyan());

    match fetch_and_execute_tx(&tx_hash, &rpc_url).await {
        Ok((trace, gas_used)) => {
            println!("{}", "Transaction fetched successfully!".green().bold());
            println!("  Opcodes: {}", trace.opcodes.len());
            println!("  Gas used: {}", gas_used);
            if let Some(block) = trace.block_number {
                println!("  Block number: {}", block);
            }
            println!();

            // Display first few opcodes
            println!("First 10 opcodes:");
            for (i, opcode) in trace.opcodes.iter().take(10).enumerate() {
                println!("  [{}] 0x{:02x}", i, opcode);
            }
            if trace.opcodes.len() > 10 {
                println!("  ... ({} more)", trace.opcodes.len() - 10);
            }
            println!();

            // Generate witness
            println!("{}", "Generating witness...".cyan());
            let witness = trace_to_witness(&trace)?;
            println!("{}", "Witness generated!".green());
            println!("  Opcode cells: {}", witness.opcode_cells.len());
            println!("  Stack cells: {}", witness.stack_cells.len());
            println!("  Gas cells: {}", witness.gas_cells.len());
            println!("  Public inputs: {}", witness.public_inputs.len());
            println!();

            // Generate proof
            println!("{}", "Generating proof...".cyan());
            let start = std::time::Instant::now();
            let proof = prove_transaction(&tx_hash, &rpc_url, &config).await?;
            let duration = start.elapsed();

            println!(
                "{}",
                format!("Proof generated in {:?}", duration).green().bold()
            );
            println!();

            // Display proof metadata
            println!("{}", "Proof Metadata:".cyan());
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
            println!("{}", "Verifying proof...".cyan());
            let start = std::time::Instant::now();
            let valid = verify_proof(&proof, &config).await?;
            let duration = start.elapsed();

            if valid {
                println!(
                    "{}",
                    format!("Proof is VALID! (verified in {:?})", duration)
                        .green()
                        .bold()
                );
            } else {
                println!("{}", "Proof is INVALID!".red().bold());
                std::process::exit(1);
            }

            // Save proof
            let filename = format!("real_tx_{}.json", &tx_hash[2..12]);
            let proof_json = serde_json::to_string_pretty(&proof)?;
            std::fs::write(&filename, proof_json)?;
            println!("\nProof saved to: {}", filename);
        }
        Err(e) => {
            println!(
                "{}",
                format!("Failed to fetch transaction: {}", e).red().bold()
            );
            println!("\n{}", "Tip: Make sure:".yellow());
            println!("  1. RPC endpoint is running and accessible");
            println!("  2. Transaction hash is valid and exists");
            println!("  3. Network connectivity is working");
            std::process::exit(1);
        }
    }

    println!("\n{}", "Example completed successfully!".green().bold());

    Ok(())
}
