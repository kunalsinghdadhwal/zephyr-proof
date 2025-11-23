//! zkEVM-Prover CLI
//!
//! Command-line interface for generating and verifying zero-knowledge proofs
//! of Ethereum Virtual Machine execution traces.

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use zephyr_proof::{
    fetch_real_trace, generate_proof, prove_transaction, verify_proof, ProofOutput, ProverConfig,
};

#[derive(Parser, Debug)]
#[command(name = "zkevm-prover")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Security parameter k (circuit size = 2^k)
    #[arg(short, long, global = true, default_value = "17")]
    k: u32,

    /// Disable parallel proof generation
    #[arg(long, global = true)]
    no_parallel: bool,

    /// Number of threads for parallel processing
    #[arg(short = 't', long, global = true)]
    threads: Option<usize>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a proof from a trace file
    Prove {
        /// Path to trace JSON file
        trace_file: PathBuf,

        /// Output proof file path
        #[arg(short, long, default_value = "proof.json")]
        output: PathBuf,

        /// Optional RPC URL to fetch additional data
        #[arg(long)]
        rpc_url: Option<String>,
    },

    /// Verify a proof
    Verify {
        /// Path to proof JSON file
        proof_file: PathBuf,
    },

    /// Simulate and prove a real transaction from network
    Simulate {
        /// Transaction hash
        tx_hash: String,

        /// Network RPC URL (e.g., http://localhost:8545 for Anvil)
        #[arg(long, default_value = "http://localhost:8545")]
        rpc_url: String,

        /// Output proof file path
        #[arg(short, long, default_value = "proof.json")]
        output: PathBuf,
    },

    /// Fetch a trace from network without proving
    Fetch {
        /// Transaction hash
        tx_hash: String,

        /// Network RPC URL
        #[arg(long, default_value = "http://localhost:8545")]
        rpc_url: String,

        /// Output trace file path
        #[arg(short, long, default_value = "trace.json")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Create prover config from global args
    let config = ProverConfig {
        k: cli.k,
        parallel: !cli.no_parallel,
        num_threads: cli.threads,
        rpc_url: None,
    };

    match cli.command {
        Commands::Prove {
            trace_file,
            output,
            rpc_url,
        } => {
            println!("Reading trace from: {}", trace_file.display());

            // Read trace file
            let trace_json = std::fs::read_to_string(&trace_file)?;

            // Update config with RPC if provided
            let mut config = config;
            config.rpc_url = rpc_url;

            println!(
                "{}",
                format!(
                    "Generating proof (k={}, parallel={})...",
                    config.k, config.parallel
                )
                .cyan()
            );

            // Generate proof
            let proof_output = generate_proof(&trace_json, &config).await?;

            println!("{}", "Proof generated!".green().bold());
            println!("   Opcodes: {}", proof_output.metadata.opcode_count);
            println!("   Gas used: {}", proof_output.metadata.gas_used);
            println!("   Proof size: {} bytes", proof_output.proof.len());

            // Save proof
            let proof_json = serde_json::to_string_pretty(&proof_output)?;
            std::fs::write(&output, proof_json)?;

            println!("Proof saved to: {}", output.display());
        }

        Commands::Verify { proof_file } => {
            println!("Reading proof from: {}", proof_file.display());

            // Read proof file
            let proof_json = std::fs::read_to_string(&proof_file)?;
            let proof_output: ProofOutput = serde_json::from_str(&proof_json)?;

            println!("{}", format!("Verifying proof (k={})...", config.k).cyan());

            // Verify proof
            let is_valid = verify_proof(&proof_output, &config).await?;

            if is_valid {
                println!("{}", "Proof is VALID!".green().bold());
                println!("   Opcodes: {}", proof_output.metadata.opcode_count);
                println!("   Gas used: {}", proof_output.metadata.gas_used);
                if let Some(tx_hash) = &proof_output.metadata.tx_hash {
                    println!("   TX hash: {}", tx_hash);
                }
            } else {
                println!("{}", "Proof is INVALID!".red().bold());
                std::process::exit(1);
            }
        }

        Commands::Simulate {
            tx_hash,
            rpc_url,
            output,
        } => {
            println!("Fetching transaction: {}", tx_hash);
            println!("   RPC: {}", rpc_url);

            // Update config with RPC
            let mut config = config;
            config.rpc_url = Some(rpc_url.clone());

            println!("{}", "Simulating and generating proof...".cyan());

            // Fetch and prove transaction
            let proof_output = prove_transaction(&tx_hash, &rpc_url, &config).await?;

            println!("{}", "Proof generated for real transaction!".green().bold());
            println!("   TX hash: {}", tx_hash);
            if let Some(block) = proof_output.metadata.block_number {
                println!("   Block: {}", block);
            }
            println!("   Opcodes: {}", proof_output.metadata.opcode_count);
            println!("   Gas used: {}", proof_output.metadata.gas_used);

            // Save proof
            let proof_json = serde_json::to_string_pretty(&proof_output)?;
            std::fs::write(&output, proof_json)?;

            println!("Proof saved to: {}", output.display());
        }

        Commands::Fetch {
            tx_hash,
            rpc_url,
            output,
        } => {
            println!("Fetching transaction trace: {}", tx_hash);
            println!("   RPC: {}", rpc_url);

            // Fetch trace
            let trace = fetch_real_trace(&tx_hash, &rpc_url).await?;

            println!("{}", "Trace fetched!".green().bold());
            println!("   Opcodes: {}", trace.opcodes.len());
            if let Some(block) = trace.block_number {
                println!("   Block: {}", block);
            }

            // Save trace
            let trace_json = serde_json::to_string_pretty(&trace)?;
            std::fs::write(&output, trace_json)?;

            println!("Trace saved to: {}", output.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
