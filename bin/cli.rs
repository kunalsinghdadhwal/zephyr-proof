//! zkEVM-Prover CLI
//!
//! Command-line interface for generating and verifying zero-knowledge proofs
//! of Ethereum Virtual Machine (EVM) execution traces.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use zephyr_proof::{
    generate_proof, new_prover, new_prover_with_params, prove_transaction, verify_proof,
    ProofOutput, ProverConfig,
};

#[derive(Parser)]
#[command(name = "zkevm-prover")]
#[command(author = "Kunal Singh Dadhwal")]
#[command(version = "0.1.0")]
#[command(about = "Generate and verify zero-knowledge proofs of EVM execution traces", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Security parameter (circuit size = 2^k)
    #[arg(short, long, default_value_t = 17, global = true)]
    k: u32,

    /// Disable parallel processing
    #[arg(long, global = true)]
    no_parallel: bool,

    /// Number of threads for parallel processing
    #[arg(short = 't', long, global = true)]
    num_threads: Option<usize>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a proof from an EVM trace file
    Prove {
        /// Path to the trace JSON file
        trace_file: PathBuf,

        /// Output file for the proof (JSON format)
        #[arg(short, long, default_value = "proof.json")]
        output: PathBuf,

        /// Optional RPC URL to fetch trace from network
        #[arg(long)]
        rpc_url: Option<String>,
    },

    /// Verify a proof
    Verify {
        /// Path to the proof JSON file
        proof_file: PathBuf,
    },

    /// Simulate and prove a real transaction from Ethereum network
    Simulate {
        /// Transaction hash to fetch and prove
        tx_hash: String,

        /// Ethereum RPC endpoint URL
        #[arg(long, default_value = "https://mainnet.infura.io/v3/YOUR_KEY")]
        rpc_url: String,

        /// Output file for the proof
        #[arg(short, long, default_value = "proof.json")]
        output: PathBuf,
    },

    /// Generate a mock trace and proof (for testing)
    Mock {
        /// Type of mock trace: add, mul
        #[arg(default_value = "add")]
        trace_type: String,

        /// Output file for the proof
        #[arg(short, long, default_value = "mock_proof.json")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    #[cfg(debug_assertions)]
    {
        tracing_subscriber::fmt::init();
    }

    let cli = Cli::parse();

    // Create prover configuration
    let config = ProverConfig {
        k: cli.k,
        parallel: !cli.no_parallel,
        num_threads: cli.num_threads,
        rpc_url: None,
    };

    match cli.command {
        Commands::Prove {
            trace_file,
            output,
            rpc_url,
        } => {
            println!("🔧 Reading trace from: {}", trace_file.display());

            // Read trace file
            let trace_json = std::fs::read_to_string(&trace_file)?;

            println!(
                "⚙️  Generating proof with k={}, parallel={}",
                config.k, config.parallel
            );

            // Generate proof
            let proof = generate_proof(&trace_json, &config).await?;

            println!("✅ Proof generated successfully!");
            println!("   Opcodes: {}", proof.metadata.opcode_count);
            println!("   Gas used: {}", proof.metadata.gas_used);

            // Save proof
            let proof_json = serde_json::to_string_pretty(&proof)?;
            std::fs::write(&output, proof_json)?;

            println!("💾 Proof saved to: {}", output.display());
        }

        Commands::Verify { proof_file } => {
            println!("🔍 Reading proof from: {}", proof_file.display());

            // Read proof file
            let proof_json = std::fs::read_to_string(&proof_file)?;
            let proof: ProofOutput = serde_json::from_str(&proof_json)?;

            println!("⚙️  Verifying proof...");

            // Verify proof
            let valid = verify_proof(&proof, &config).await?;

            if valid {
                println!("✅ Proof is VALID!");
                println!("   Opcodes: {}", proof.metadata.opcode_count);
                println!("   Gas used: {}", proof.metadata.gas_used);
            } else {
                println!("❌ Proof is INVALID!");
                std::process::exit(1);
            }
        }

        Commands::Simulate {
            tx_hash,
            rpc_url,
            output,
        } => {
            println!("🌐 Fetching transaction: {}", tx_hash);
            println!("   RPC URL: {}", rpc_url);

            // Fetch and prove transaction
            let proof = prove_transaction(&tx_hash, &rpc_url, &config).await?;

            println!("✅ Proof generated for transaction!");
            println!("   Opcodes: {}", proof.metadata.opcode_count);
            println!("   Gas used: {}", proof.metadata.gas_used);

            // Save proof
            let proof_json = serde_json::to_string_pretty(&proof)?;
            std::fs::write(&output, proof_json)?;

            println!("💾 Proof saved to: {}", output.display());
        }

        Commands::Mock { trace_type, output } => {
            use zephyr_proof::utils::evm_parser::EvmTrace;

            println!("🧪 Generating mock {} trace", trace_type);

            // Create mock trace
            let trace = match trace_type.as_str() {
                "add" => EvmTrace::mock_add(),
                "mul" => EvmTrace::mock_mul(),
                _ => {
                    eprintln!("❌ Unknown trace type: {}", trace_type);
                    eprintln!("   Available types: add, mul");
                    std::process::exit(1);
                }
            };

            let trace_json = serde_json::to_string(&trace)?;

            println!("⚙️  Generating proof...");

            // Generate proof
            let proof = generate_proof(&trace_json, &config).await?;

            println!("✅ Mock proof generated!");
            println!("   Opcodes: {}", proof.metadata.opcode_count);
            println!("   Gas used: {}", proof.metadata.gas_used);

            // Save proof
            let proof_json = serde_json::to_string_pretty(&proof)?;
            std::fs::write(&output, proof_json)?;

            println!("💾 Proof saved to: {}", output.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test that CLI parses correctly
        let cli = Cli::parse_from(&["zkevm-prover", "mock", "add"]);
        assert_eq!(cli.k, 17);
        assert!(!cli.no_parallel);
    }
}
