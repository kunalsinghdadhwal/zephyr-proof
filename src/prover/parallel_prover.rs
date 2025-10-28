//! Parallel proof generation using Rayon
//!
//! Efficiently generates proofs for large EVM traces using parallel processing.

use crate::{
    ProofOutput, ProverConfig, TraceInfo,
    circuits::{EvmCircuit, ExecutionStep},
    errors::{ProverError, Result},
    utils::evm_parser::EvmTrace,
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
pub async fn generate_proof_parallel(
    trace: &EvmTrace,
    config: &ProverConfig,
) -> Result<ProofOutput> {
    // Set number of threads if specified
    if let Some(num_threads) = config.num_threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .map_err(|e| ProverError::ProofGenerationError(e.to_string()))?;
    }

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

    // Mock trace commitment (TODO: Use Poseidon hash)
    let trace_commitment = Fp::from(trace.opcodes.len() as u64);

    let circuit = EvmCircuit::new(steps, trace_commitment);

    // Generate proof using MockProver (for MVP)
    // TODO: Replace with real prover (create_proof) for production
    let k = config.k;
    let public_inputs = vec![trace_commitment];

    let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()])
        .map_err(|e| ProverError::Halo2Error(format!("{:?}", e)))?;

    prover
        .verify()
        .map_err(|e| ProverError::VerificationError(format!("{:?}", e)))?;

    // Mock proof bytes (real impl would use create_proof)
    let proof_bytes = vec![0u8; 128]; // Placeholder
    let proof_b64 = general_purpose::STANDARD.encode(&proof_bytes);

    // Generate metadata
    let metadata = TraceInfo {
        opcode_count: trace.opcodes.len(),
        gas_used: trace.gas_values.first().copied().unwrap_or(0)
            - trace.gas_values.last().copied().unwrap_or(0),
        tx_hash: trace.tx_hash.clone(),
        block_number: trace.block_number,
    };

    // Mock VK hash
    let vk_hash = format!("vk_{}", k);

    Ok(ProofOutput {
        proof: proof_b64,
        public_inputs: public_inputs.iter().map(|f| format!("{:?}", f)).collect(),
        metadata,
        vk_hash,
    })
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

    let trace_commitment = Fp::from(trace.opcodes.len() as u64);
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

    let vk_hash = format!("vk_{}", k);

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
