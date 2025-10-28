//! zkEVM-Prover Library
//!
//! A comprehensive library for generating and verifying zero-knowledge proofs
//! of Ethereum Virtual Machine (EVM) execution traces using the Halo2 proof system.
//!
//! # Architecture
//!
//! - **Chips**: Low-level Halo2 gadgets for arithmetic and EVM operations
//! - **Circuits**: Composable circuits for EVM state transitions
//! - **Prover**: Parallel proof generation and verification
//! - **Utils**: Trace parsing, benchmarking utilities
//!
//! # Example
//!
//! ```no_run
//! use zephyr_proof::{ProverConfig, generate_proof};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ProverConfig::default();
//! let trace_json = r#"{"opcodes": ["PUSH1", "PUSH1", "ADD"]}"#;
//! let proof = generate_proof(trace_json, &config).await?;
//! println!("Proof generated: {} bytes", proof.proof.len());
//! # Ok(())
//! # }
//! ```

pub mod chips;
pub mod circuits;
pub mod errors;
pub mod prover;
pub mod utils;

use errors::ProverError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for the proof generation process
#[derive(Debug, Clone)]
pub struct ProverConfig {
    /// Security parameter (circuit size = 2^k)
    pub k: u32,
    /// Enable parallel proof generation
    pub parallel: bool,
    /// Number of threads for parallel processing
    pub num_threads: Option<usize>,
    /// Optional RPC URL for fetching real traces
    pub rpc_url: Option<String>,
}

impl Default for ProverConfig {
    fn default() -> Self {
        Self {
            k: 17, // 2^17 = 131072 rows
            parallel: true,
            num_threads: None, // Use all available cores
            rpc_url: None,
        }
    }
}

/// Metadata about the execution trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceInfo {
    /// Number of opcodes in the trace
    pub opcode_count: usize,
    /// Estimated gas used
    pub gas_used: u64,
    /// Transaction hash (if from real network)
    pub tx_hash: Option<String>,
    /// Block number (if from real network)
    pub block_number: Option<u64>,
}

/// Output of proof generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOutput {
    /// Base64-encoded proof bytes
    pub proof: String,
    /// Public inputs (trace commitment)
    pub public_inputs: Vec<String>,
    /// Trace metadata
    pub metadata: TraceInfo,
    /// Verification key hash (for quick VK matching)
    pub vk_hash: String,
}

/// Result type for prover operations
pub type ProverResult<T> = Result<T, ProverError>;

/// Generate a proof from an EVM trace JSON string
///
/// # Arguments
///
/// * `trace_json` - JSON representation of the EVM trace
/// * `config` - Prover configuration
///
/// # Returns
///
/// A `ProofOutput` containing the proof and metadata
pub async fn generate_proof(trace_json: &str, config: &ProverConfig) -> ProverResult<ProofOutput> {
    // Parse trace
    let trace = utils::evm_parser::parse_trace_json(trace_json)?;
    
    // Generate proof using parallel prover
    if config.parallel {
        prover::parallel_prover::generate_proof_parallel(&trace, config).await
    } else {
        prover::parallel_prover::generate_proof_sequential(&trace, config).await
    }
}

/// Verify a proof
///
/// # Arguments
///
/// * `proof_output` - The proof to verify
/// * `config` - Prover configuration (must match proof generation)
///
/// # Returns
///
/// `true` if the proof is valid, `false` otherwise
pub async fn verify_proof(proof_output: &ProofOutput, config: &ProverConfig) -> ProverResult<bool> {
    prover::verifier::verify(proof_output, config).await
}

/// Fetch and prove a real transaction from Ethereum network
///
/// # Arguments
///
/// * `tx_hash` - Transaction hash to fetch and prove
/// * `rpc_url` - Ethereum RPC endpoint URL
/// * `config` - Prover configuration
///
/// # Returns
///
/// A `ProofOutput` for the transaction trace
pub async fn prove_transaction(
    tx_hash: &str,
    rpc_url: &str,
    config: &ProverConfig,
) -> ProverResult<ProofOutput> {
    // Fetch trace via Ethers
    let trace = utils::evm_parser::fetch_trace_from_network(tx_hash, rpc_url).await?;
    
    // Generate proof
    let trace_json = serde_json::to_string(&trace)?;
    generate_proof(&trace_json, config).await
}

/// Create a new prover with default configuration
pub fn new_prover() -> ProverConfig {
    ProverConfig::default()
}

/// Create a new prover with custom parameters
pub fn new_prover_with_params(k: u32, parallel: bool) -> ProverConfig {
    ProverConfig {
        k,
        parallel,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prover_config_default() {
        let config = ProverConfig::default();
        assert_eq!(config.k, 17);
        assert!(config.parallel);
    }

    #[test]
    fn test_new_prover() {
        let prover = new_prover();
        assert_eq!(prover.k, 17);
    }

    #[test]
    fn test_new_prover_with_params() {
        let prover = new_prover_with_params(20, false);
        assert_eq!(prover.k, 20);
        assert!(!prover.parallel);
    }
}
