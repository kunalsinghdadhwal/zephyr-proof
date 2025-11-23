//! Integration tests for zkEVM-Prover
//!
//! Tests end-to-end workflows including trace parsing, proof generation, and verification

use zephyr_proof::{
    generate_proof,
    utils::evm_parser::{parse_evm_data, parse_trace_json, EvmTrace},
    verify_proof, ProverConfig,
};

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
async fn test_end_to_end_proof_generation_and_verification() {
    // Create trace
    let trace = create_test_trace();
    let trace_json = serde_json::to_string(&trace).unwrap();

    // Generate proof
    let config = ProverConfig::default();
    let proof_result = generate_proof(&trace_json, &config).await;
    assert!(proof_result.is_ok());

    let proof = proof_result.unwrap();

    // Verify proof metadata
    assert_eq!(proof.metadata.opcode_count, 3);
    assert_eq!(proof.metadata.gas_used, 6);
    assert!(proof.proof.len() > 0);

    // Verify proof
    let verify_result = verify_proof(&proof, &config).await;
    assert!(
        verify_result.is_ok(),
        "Verification failed: {:?}",
        verify_result
    );
    assert!(verify_result.unwrap(), "Proof should be valid");
}

#[tokio::test]
async fn test_proof_generation_with_different_k_values() {
    let trace = create_test_trace();
    let trace_json = serde_json::to_string(&trace).unwrap();

    for k in [10, 12, 15, 17, 20] {
        let config = ProverConfig {
            k,
            ..Default::default()
        };

        let proof_result = generate_proof(&trace_json, &config).await;
        assert!(proof_result.is_ok(), "Failed with k={}", k);

        let proof = proof_result.unwrap();
        let verify_result = verify_proof(&proof, &config).await;
        assert!(
            verify_result.is_ok(),
            "Verification failed for k={}: {:?}",
            k,
            verify_result
        );
        assert!(verify_result.unwrap(), "Proof should be valid for k={}", k);
    }
}

#[tokio::test]
async fn test_parallel_vs_sequential_proof_generation() {
    let trace = create_test_trace();
    let trace_json = serde_json::to_string(&trace).unwrap();

    // Parallel proof
    let parallel_config = ProverConfig {
        parallel: true,
        ..Default::default()
    };
    let parallel_proof = generate_proof(&trace_json, &parallel_config).await.unwrap();

    // Sequential proof
    let sequential_config = ProverConfig {
        parallel: false,
        ..Default::default()
    };
    let sequential_proof = generate_proof(&trace_json, &sequential_config)
        .await
        .unwrap();

    // Both should have same metadata
    assert_eq!(
        parallel_proof.metadata.opcode_count,
        sequential_proof.metadata.opcode_count
    );
    assert_eq!(
        parallel_proof.metadata.gas_used,
        sequential_proof.metadata.gas_used
    );

    // Both should verify
    assert!(verify_proof(&parallel_proof, &parallel_config)
        .await
        .unwrap());
    assert!(verify_proof(&sequential_proof, &sequential_config)
        .await
        .unwrap());
}

#[tokio::test]
async fn test_invalid_trace_rejected() {
    let invalid_trace = EvmTrace {
        opcodes: vec![],
        stack_states: vec![],
        pcs: vec![],
        gas_values: vec![],
        memory_ops: None,
        storage_ops: None,
        tx_hash: None,
        block_number: None,
        bytecode: None,
    };

    let trace_json = serde_json::to_string(&invalid_trace).unwrap();
    let config = ProverConfig::default();

    let result = generate_proof(&trace_json, &config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_trace_with_metadata() {
    let trace = EvmTrace {
        opcodes: vec![0x60, 0x01],
        stack_states: vec![vec![1, 0, 0], vec![2, 0, 0]],
        pcs: vec![0, 2],
        gas_values: vec![1000, 997],
        memory_ops: None,
        storage_ops: None,
        tx_hash: Some("0xabcdef1234567890".to_string()),
        block_number: Some(15000000),
        bytecode: None,
    };

    let trace_json = serde_json::to_string(&trace).unwrap();
    let config = ProverConfig::default();

    let proof = generate_proof(&trace_json, &config).await.unwrap();

    assert_eq!(
        proof.metadata.tx_hash,
        Some("0xabcdef1234567890".to_string())
    );
    assert_eq!(proof.metadata.block_number, Some(15000000));
    assert!(verify_proof(&proof, &config).await.unwrap());
}

#[test]
fn test_parse_trace_from_json() {
    let json = r#"{
        "opcodes": [96, 96, 1],
        "stack_states": [[1, 0, 0], [2, 1, 0], [3, 0, 0]],
        "pcs": [0, 2, 4],
        "gas_values": [1000, 997, 994],
        "memory_ops": null,
        "storage_ops": null,
        "tx_hash": null,
        "block_number": null,
        "bytecode": [96, 1, 96, 2, 1]
    }"#;

    let result = parse_trace_json(json);
    assert!(result.is_ok());

    let trace = result.unwrap();
    assert_eq!(trace.opcodes.len(), 3);
    assert_eq!(trace.opcodes[2], 1); // ADD opcode
    assert_eq!(trace.stack_states[2][0], 3); // Result of 1 + 2
}

#[test]
fn test_witness_generation() {
    let trace = create_test_trace();
    let witness = parse_evm_data(&trace).unwrap();

    // Verify witness structure
    assert_eq!(witness.opcode_cells.len(), trace.opcodes.len());
    assert_eq!(witness.gas_cells.len(), trace.gas_values.len());
    assert_eq!(witness.public_inputs.len(), 4); // SHA256 hash split into 4 u64s

    // Verify opcode conversion
    for (i, &opcode) in trace.opcodes.iter().enumerate() {
        assert_eq!(witness.opcode_cells[i], opcode as u64);
    }
}

#[tokio::test]
async fn test_large_trace() {
    // Create a larger trace
    let opcodes = vec![0x60; 50]; // 50 PUSH1 operations
    let stack_states = vec![vec![1, 0, 0]; 50];
    let pcs: Vec<u64> = (0..50).map(|i| i * 2).collect();
    let gas_values: Vec<u64> = (0..50).map(|i| 1000 - i * 3).collect();

    let trace = EvmTrace {
        opcodes,
        stack_states,
        pcs,
        gas_values,
        memory_ops: None,
        storage_ops: None,
        tx_hash: None,
        block_number: None,
        bytecode: None,
    };

    let trace_json = serde_json::to_string(&trace).unwrap();
    let config = ProverConfig {
        k: 20, // Larger k for more rows
        ..Default::default()
    };

    let proof_result = generate_proof(&trace_json, &config).await;
    assert!(proof_result.is_ok());

    let proof = proof_result.unwrap();
    assert_eq!(proof.metadata.opcode_count, 50);
}

#[tokio::test]
async fn test_vk_hash_consistency() {
    let trace = create_test_trace();
    let trace_json = serde_json::to_string(&trace).unwrap();
    let config = ProverConfig::default();

    // Generate multiple proofs
    let proof1 = generate_proof(&trace_json, &config).await.unwrap();
    let proof2 = generate_proof(&trace_json, &config).await.unwrap();

    // VK hash should be consistent for same config
    assert_eq!(proof1.vk_hash, proof2.vk_hash);

    // Different k should produce different VK hash
    let config2 = ProverConfig {
        k: 18,
        ..Default::default()
    };
    let proof3 = generate_proof(&trace_json, &config2).await.unwrap();
    assert_ne!(proof1.vk_hash, proof3.vk_hash);
}

#[tokio::test]
async fn test_proof_serialization() {
    let trace = create_test_trace();
    let trace_json = serde_json::to_string(&trace).unwrap();
    let config = ProverConfig::default();

    let proof = generate_proof(&trace_json, &config).await.unwrap();

    // Serialize to JSON
    let proof_json = serde_json::to_string(&proof).unwrap();
    assert!(proof_json.len() > 0);

    // Deserialize back
    let deserialized: zephyr_proof::ProofOutput = serde_json::from_str(&proof_json).unwrap();

    assert_eq!(
        deserialized.metadata.opcode_count,
        proof.metadata.opcode_count
    );
    assert_eq!(deserialized.metadata.gas_used, proof.metadata.gas_used);
    assert_eq!(deserialized.vk_hash, proof.vk_hash);
}

#[tokio::test]
async fn test_multiple_arithmetic_operations() {
    // Test trace with multiple operations: ADD, then SUB
    // Use SUB instead of MUL to keep gas costs consistent at 3 per op
    let trace = EvmTrace {
        opcodes: vec![0x60, 0x60, 0x01, 0x60, 0x03], // PUSH1, PUSH1, ADD, PUSH1, SUB
        stack_states: vec![
            vec![1, 0, 0],
            vec![2, 1, 0],
            vec![3, 0, 0],
            vec![4, 3, 0],
            vec![1, 0, 0], // 4 - 3 = 1
        ],
        pcs: vec![0, 2, 4, 6, 8],
        gas_values: vec![1000, 997, 994, 991, 988],
        memory_ops: None,
        storage_ops: None,
        tx_hash: None,
        block_number: None,
        bytecode: None,
    };

    let trace_json = serde_json::to_string(&trace).unwrap();
    let config = ProverConfig::default();

    let proof = generate_proof(&trace_json, &config).await.unwrap();
    assert_eq!(proof.metadata.opcode_count, 5);
    assert!(verify_proof(&proof, &config).await.unwrap());
}
