//! Differential tests for zkEVM-Prover
//!
//! Tests that compare different implementations or approaches

use zephyr_proof::{generate_proof, utils::evm_parser::EvmTrace, ProverConfig};

/// Helper to create a test trace
fn create_test_trace() -> EvmTrace {
    EvmTrace {
        opcodes: vec![0x60, 0x60, 0x01], // PUSH1, PUSH1, ADD
        stack_states: vec![vec![1, 0, 0], vec![2, 1, 0], vec![3, 0, 0]],
        pcs: vec![0, 2, 4],
        gas_values: vec![1000, 997, 994],
        memory_ops: None,
        storage_ops: None,
        tx_hash: None,
        block_number: None,
        bytecode: Some(vec![0x60, 0x01, 0x60, 0x02, 0x01]),
    }
}

#[tokio::test]
async fn test_parallel_sequential_equivalence() {
    let trace = create_test_trace();
    let trace_json = serde_json::to_string(&trace).unwrap();

    // Generate proof with parallel
    let config_parallel = ProverConfig {
        parallel: true,
        k: 17,
        num_threads: None,
        rpc_url: None,
    };
    let proof_parallel = generate_proof(&trace_json, &config_parallel).await.unwrap();

    // Generate proof with sequential
    let config_sequential = ProverConfig {
        parallel: false,
        k: 17,
        num_threads: None,
        rpc_url: None,
    };
    let proof_sequential = generate_proof(&trace_json, &config_sequential)
        .await
        .unwrap();

    // Both should produce same metadata
    assert_eq!(
        proof_parallel.metadata.opcode_count,
        proof_sequential.metadata.opcode_count
    );
    assert_eq!(
        proof_parallel.metadata.gas_used,
        proof_sequential.metadata.gas_used
    );

    // VK hash should be the same
    assert_eq!(proof_parallel.vk_hash, proof_sequential.vk_hash);
}

#[tokio::test]
async fn test_different_k_values_produce_different_vks() {
    let trace = create_test_trace();
    let trace_json = serde_json::to_string(&trace).unwrap();

    let k_values = vec![10, 12, 15, 17, 20];
    let mut vk_hashes = Vec::new();

    for k in k_values {
        let config = ProverConfig {
            k,
            ..Default::default()
        };
        let proof = generate_proof(&trace_json, &config).await.unwrap();
        vk_hashes.push(proof.vk_hash);
    }

    // All VK hashes should be unique
    for i in 0..vk_hashes.len() {
        for j in (i + 1)..vk_hashes.len() {
            assert_ne!(
                vk_hashes[i], vk_hashes[j],
                "k values {} and {} produced same VK",
                i, j
            );
        }
    }
}

#[tokio::test]
async fn test_deterministic_proof_generation() {
    let trace = create_test_trace();
    let trace_json = serde_json::to_string(&trace).unwrap();
    let config = ProverConfig::default();

    // Generate multiple proofs
    let proof1 = generate_proof(&trace_json, &config).await.unwrap();
    let proof2 = generate_proof(&trace_json, &config).await.unwrap();
    let proof3 = generate_proof(&trace_json, &config).await.unwrap();

    // All should have same metadata
    assert_eq!(proof1.metadata.opcode_count, proof2.metadata.opcode_count);
    assert_eq!(proof2.metadata.opcode_count, proof3.metadata.opcode_count);

    assert_eq!(proof1.metadata.gas_used, proof2.metadata.gas_used);
    assert_eq!(proof2.metadata.gas_used, proof3.metadata.gas_used);

    // VK hash should be deterministic
    assert_eq!(proof1.vk_hash, proof2.vk_hash);
    assert_eq!(proof2.vk_hash, proof3.vk_hash);
}

#[tokio::test]
async fn test_different_traces_produce_different_commitments() {
    let config = ProverConfig::default();

    // Trace 1: ADD operation
    let trace1 = EvmTrace {
        opcodes: vec![0x60, 0x60, 0x01],
        stack_states: vec![vec![1, 0, 0], vec![2, 1, 0], vec![3, 0, 0]],
        pcs: vec![0, 2, 4],
        gas_values: vec![1000, 997, 994],
        memory_ops: None,
        storage_ops: None,
        tx_hash: None,
        block_number: None,
        bytecode: None,
    };

    // Trace 2: Different opcodes - use PUSH1, PUSH1, SUB
    let trace2 = EvmTrace {
        opcodes: vec![0x60, 0x60, 0x03],
        stack_states: vec![vec![5, 0, 0], vec![3, 5, 0], vec![2, 0, 0]],
        pcs: vec![0, 2, 4],
        gas_values: vec![1000, 997, 994],
        memory_ops: None,
        storage_ops: None,
        tx_hash: None,
        block_number: None,
        bytecode: None,
    };

    let json1 = serde_json::to_string(&trace1).unwrap();
    let json2 = serde_json::to_string(&trace2).unwrap();

    let proof1 = generate_proof(&json1, &config).await.unwrap();
    let proof2 = generate_proof(&json2, &config).await.unwrap();

    // Public inputs should be different
    assert_ne!(proof1.public_inputs, proof2.public_inputs);
}

#[tokio::test]
async fn test_trace_length_affects_gas() {
    let config = ProverConfig::default();

    // Short trace
    let trace_short = EvmTrace {
        opcodes: vec![0x60, 0x60, 0x01],
        stack_states: vec![vec![1, 0, 0], vec![2, 1, 0], vec![3, 0, 0]],
        pcs: vec![0, 2, 4],
        gas_values: vec![1000, 997, 994],
        memory_ops: None,
        storage_ops: None,
        tx_hash: None,
        block_number: None,
        bytecode: None,
    };

    // Long trace
    let opcodes_long = vec![0x60; 20];
    let stack_states_long = vec![vec![1, 0, 0]; 20];
    let pcs_long: Vec<u64> = (0..20).map(|i| i * 2).collect();
    let gas_values_long: Vec<u64> = (0..20).map(|i| 1000 - i * 3).collect();

    let trace_long = EvmTrace {
        opcodes: opcodes_long,
        stack_states: stack_states_long,
        pcs: pcs_long,
        gas_values: gas_values_long,
        memory_ops: None,
        storage_ops: None,
        tx_hash: None,
        block_number: None,
        bytecode: None,
    };

    let json_short = serde_json::to_string(&trace_short).unwrap();
    let json_long = serde_json::to_string(&trace_long).unwrap();

    let proof_short = generate_proof(&json_short, &config).await.unwrap();
    let proof_long = generate_proof(&json_long, &config).await.unwrap();

    // Longer trace should have higher gas usage
    assert!(proof_long.metadata.gas_used > proof_short.metadata.gas_used);
    assert!(proof_long.metadata.opcode_count > proof_short.metadata.opcode_count);
}

#[tokio::test]
async fn test_thread_count_doesnt_affect_result() {
    let trace = create_test_trace();
    let trace_json = serde_json::to_string(&trace).unwrap();

    // Test with None (auto-detect) and with parallel disabled
    // Avoid setting specific thread counts as Rayon global pool can only be initialized once
    let configs = vec![
        ProverConfig {
            parallel: true,
            num_threads: None,
            ..Default::default()
        },
        ProverConfig {
            parallel: false,
            num_threads: None,
            ..Default::default()
        },
    ];

    let mut proofs = Vec::new();
    for config in configs {
        let proof = generate_proof(&trace_json, &config).await.unwrap();
        proofs.push(proof);
    }

    // All should produce same metadata
    for i in 1..proofs.len() {
        assert_eq!(
            proofs[0].metadata.opcode_count,
            proofs[i].metadata.opcode_count
        );
        assert_eq!(proofs[0].metadata.gas_used, proofs[i].metadata.gas_used);
    }
}

#[test]
fn test_commitment_collision_resistance() {
    use std::collections::HashSet;
    use zephyr_proof::utils::evm_parser::EvmTrace;

    let mut commitments = HashSet::new();

    // Generate many different traces
    for i in 0..100 {
        let trace = EvmTrace {
            opcodes: vec![0x60, (i % 256) as u8],
            stack_states: vec![vec![i as u64, 0, 0], vec![(i + 1) as u64, 0, 0]],
            pcs: vec![0, 2],
            gas_values: vec![1000, 997],
            memory_ops: None,
            storage_ops: None,
            tx_hash: None,
            block_number: None,
            bytecode: None,
        };

        let witness = zephyr_proof::utils::evm_parser::parse_evm_data(&trace).unwrap();
        let commitment = format!("{:?}", witness.public_inputs);

        // Should not have collisions
        assert!(
            commitments.insert(commitment),
            "Commitment collision detected for trace {}",
            i
        );
    }

    assert_eq!(commitments.len(), 100);
}
