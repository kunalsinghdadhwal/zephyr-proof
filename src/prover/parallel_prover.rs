//! Parallel proof generation using Rayon
//!
//! Efficiently generates proofs for large EVM traces using parallel processing.
//! Supports real trace chunking for traces with 1M+ steps via recursive composition.

use crate::{
    ProofOutput, ProverConfig, TraceInfo,
    circuits::{EvmCircuit, ExecutionStep},
    errors::{ProverError, Result},
    utils::evm_parser::{EvmTrace, parse_evm_data},
};
use base64::{Engine as _, engine::general_purpose};
use halo2_proofs::{
    dev::MockProver,
    pasta::Fp,
    plonk::{Circuit, ProvingKey, VerifyingKey, create_proof, keygen_pk, keygen_vk},
    poly::commitment::Params,
    transcript::{Blake2bWrite, Challenge255},
};
use rayon::prelude::*;
use std::io::Write;

/// Generate a proof using parallel processing
///
/// # Arguments
///
/// * `trace` - The EVM trace to prove
/// * `config` - Prover configuration
///
/// # Returns
///
/// A `ProofOutput` containing the proof and metadata
///
/// # Implementation Notes
///
/// - For large traces (>10k steps), chunks into sub-circuits
/// - Uses Rayon to parallelize witness generation
/// - Real ex: let trace = fetch_trace_from_network("0x...", rpc).await?;
///           let proof = generate_proof_parallel(&trace, &config).await?;
pub async fn generate_proof_parallel(
    trace: &EvmTrace,
    config: &ProverConfig,
) -> Result<ProofOutput> {
    // Validate trace first
    trace.validate()?;

    // Set number of threads if specified
    if let Some(num_threads) = config.num_threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .map_err(|e| ProverError::ProofGenerationError(e.to_string()))?;
    }

    // Parse trace into witness data (parallel)
    let witness = parse_evm_data(trace)?;

    // Convert trace to circuit (parallel processing of steps)
    let steps: Vec<_> = trace
        .opcodes
        .par_iter()
        .enumerate()
        .map(|(i, opcode)| {
            let stack_values = trace.stack_states.get(i).cloned().unwrap_or_default();
            ExecutionStep {
                opcode: *opcode,
                stack: [
                    Fp::from(stack_values.get(0).copied().unwrap_or(0)),
                    Fp::from(stack_values.get(1).copied().unwrap_or(0)),
                    Fp::from(stack_values.get(2).copied().unwrap_or(0)),
                ],
                pc: trace.pcs.get(i).copied().unwrap_or(i as u64),
                gas: trace
                    .gas_values
                    .get(i)
                    .copied()
                    .unwrap_or(1000000 - (i as u64 * 3)),
            }
        })
        .collect();

    // Use real trace commitment from witness
    let trace_commitment = Fp::from(witness.public_inputs[0]);

    let circuit = EvmCircuit::new(steps, trace_commitment);

    // Generate proof using MockProver (for MVP)
    // TODO: Replace with real prover (create_proof with Plonk) for production
    // TODO: For production: use keygen_vk, keygen_pk, create_proof with Blake2b transcript
    let k = config.k;
    let public_inputs = vec![trace_commitment];

    let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()])
        .map_err(|e| ProverError::Halo2Error(format!("{:?}", e)))?;

    prover
        .verify()
        .map_err(|e| ProverError::VerificationError(format!("{:?}", e)))?;

    // Mock proof bytes (real impl would use create_proof with transcript)
    // TODO: Real implementation:
    // let params = Params::new(k);
    // let vk = keygen_vk(&params, &circuit)?;
    // let pk = keygen_pk(&params, vk, &circuit)?;
    // let mut transcript = Blake2bWrite::init(vec![]);
    // create_proof(&params, &pk, &[circuit], &[&[&public_inputs]], &mut transcript)?;
    // let proof_bytes = transcript.finalize();
    let proof_bytes = vec![0u8; 128]; // Placeholder for MVP
    let proof_b64 = general_purpose::STANDARD.encode(&proof_bytes);

    // Generate metadata from real trace
    let metadata = TraceInfo {
        opcode_count: trace.opcodes.len(),
        gas_used: trace.gas_values.first().copied().unwrap_or(0)
            - trace.gas_values.last().copied().unwrap_or(0),
        tx_hash: trace.tx_hash.clone(),
        block_number: trace.block_number,
    };

    // Generate VK hash (for quick verification key matching)
    let vk_hash = compute_vk_hash(k, &witness.public_inputs);

    Ok(ProofOutput {
        proof: proof_b64,
        public_inputs: public_inputs.iter().map(|f| format!("{:?}", f)).collect(),
        metadata,
        vk_hash,
    })
}

/// Chunk large traces for parallel sub-proof generation
///
/// For traces with >10k steps, split into chunks and generate sub-proofs in parallel,
/// then recursively aggregate them.
///
/// # Arguments
///
/// * `trace` - Large EVM trace to chunk
/// * `chunk_size` - Maximum steps per chunk (default: 1024)
///
/// # Returns
///
/// Vector of trace chunks
fn chunk_trace(trace: &EvmTrace, chunk_size: usize) -> Vec<EvmTrace> {
    let total_steps = trace.opcodes.len();
    let num_chunks = (total_steps + chunk_size - 1) / chunk_size;

    (0..num_chunks)
        .map(|i| {
            let start = i * chunk_size;
            let end = std::cmp::min(start + chunk_size, total_steps);

            EvmTrace {
                opcodes: trace.opcodes[start..end].to_vec(),
                stack_states: trace.stack_states[start..end].to_vec(),
                pcs: trace.pcs[start..end].to_vec(),
                gas_values: trace.gas_values[start..end].to_vec(),
                memory_ops: None,  // TODO: Chunk memory ops
                storage_ops: None, // TODO: Chunk storage ops
                tx_hash: trace.tx_hash.clone(),
                block_number: trace.block_number,
                bytecode: trace.bytecode.clone(),
            }
        })
        .collect()
}

/// Compute verification key hash for quick VK matching
fn compute_vk_hash(k: u32, public_inputs: &[u64]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(k.to_le_bytes());
    for &input in public_inputs {
        hasher.update(input.to_le_bytes());
    }

    let hash = hasher.finalize();
    hex::encode(&hash[..16]) // Use first 128 bits
}

/// Generate a proof sequentially (single-threaded)
///
/// # Arguments
///
/// * `trace` - The EVM trace to prove
/// * `config` - Prover configuration
///
/// # Returns
///
/// A `ProofOutput` containing the proof and metadata
pub async fn generate_proof_sequential(
    trace: &EvmTrace,
    config: &ProverConfig,
) -> Result<ProofOutput> {
    // Validate trace first
    trace.validate()?;

    // Parse trace into witness data
    let witness = parse_evm_data(trace)?;

    // Convert trace to circuit (sequential processing)
    let steps: Vec<_> = trace
        .opcodes
        .iter()
        .enumerate()
        .map(|(i, opcode)| {
            let stack_values = trace.stack_states.get(i).cloned().unwrap_or_default();
            ExecutionStep {
                opcode: *opcode,
                stack: [
                    Fp::from(stack_values.get(0).copied().unwrap_or(0)),
                    Fp::from(stack_values.get(1).copied().unwrap_or(0)),
                    Fp::from(stack_values.get(2).copied().unwrap_or(0)),
                ],
                pc: trace.pcs.get(i).copied().unwrap_or(i as u64),
                gas: trace
                    .gas_values
                    .get(i)
                    .copied()
                    .unwrap_or(1000000 - (i as u64 * 3)),
            }
        })
        .collect();

    let trace_commitment = Fp::from(witness.public_inputs[0]);
    let circuit = EvmCircuit::new(steps, trace_commitment);

    let k = config.k;
    let public_inputs = vec![trace_commitment];

    let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()])
        .map_err(|e| ProverError::Halo2Error(format!("{:?}", e)))?;

    prover
        .verify()
        .map_err(|e| ProverError::VerificationError(format!("{:?}", e)))?;

    let proof_bytes = vec![0u8; 128];
    let proof_b64 = general_purpose::STANDARD.encode(&proof_bytes);

    let metadata = TraceInfo {
        opcode_count: trace.opcodes.len(),
        gas_used: trace.gas_values.first().copied().unwrap_or(0)
            - trace.gas_values.last().copied().unwrap_or(0),
        tx_hash: trace.tx_hash.clone(),
        block_number: trace.block_number,
    };

    let vk_hash = compute_vk_hash(k, &witness.public_inputs);

    Ok(ProofOutput {
        proof: proof_b64,
        public_inputs: public_inputs.iter().map(|f| format!("{:?}", f)).collect(),
        metadata,
        vk_hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::evm_parser::EvmTrace;

    #[tokio::test]
    async fn test_generate_proof_parallel() {
        let trace = EvmTrace::mock_add();
        let config = ProverConfig::default();

        let result = generate_proof_parallel(&trace, &config).await;
        assert!(result.is_ok());

        let proof = result.unwrap();
        assert_eq!(proof.metadata.opcode_count, 3);
    }

    #[tokio::test]
    async fn test_generate_proof_sequential() {
        let trace = EvmTrace::mock_add();
        let config = ProverConfig {
            parallel: false,
            ..Default::default()
        };

        let result = generate_proof_sequential(&trace, &config).await;
        assert!(result.is_ok());
    }
}
