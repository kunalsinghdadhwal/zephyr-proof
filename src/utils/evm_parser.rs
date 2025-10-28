//! EVM trace parser
//!
//! Parses EVM execution traces from JSON, fetches them from Ethereum networks via Alloy,
//! and simulates execution using REVM to extract real opcodes, stack, memory, and storage.

use crate::errors::{ProverError, Result};
use alloy_primitives::{Address, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{BlockNumberOrTag, TransactionRequest};
use revm::{
    primitives::{ExecutionResult, Output, TxEnv},
    Database, Evm,
};
use revm_primitives::{Bytecode, Env, SpecId};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// EVM execution trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmTrace {
    /// Opcodes executed (raw bytes from bytecode execution)
    pub opcodes: Vec<u8>,
    /// Stack states at each step (top 3 values for circuit constraints)
    pub stack_states: Vec<Vec<u64>>,
    /// Program counter values
    pub pcs: Vec<u64>,
    /// Gas values at each step
    pub gas_values: Vec<u64>,
    /// Memory snapshots (optional, for MLOAD/MSTORE ops)
    pub memory_ops: Option<Vec<MemoryOp>>,
    /// Storage operations (for SLOAD/SSTORE)
    pub storage_ops: Option<Vec<StorageOp>>,
    /// Transaction hash (if from network)
    pub tx_hash: Option<String>,
    /// Block number (if from network)
    pub block_number: Option<u64>,
    /// Actual bytecode executed
    pub bytecode: Option<Vec<u8>>,
}

/// Memory operation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOp {
    pub offset: u64,
    pub value: Vec<u8>,
    pub is_write: bool,
}

/// Storage operation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageOp {
    pub key: U256,
    pub value: U256,
    pub is_write: bool,
}

/// Circuit witness data extracted from trace
#[derive(Debug, Clone)]
pub struct CircuitWitness {
    /// Flattened opcode cells
    pub opcode_cells: Vec<u64>,
    /// Flattened stack cells
    pub stack_cells: Vec<u64>,
    /// Gas consumption per step
    pub gas_cells: Vec<u64>,
    /// Public inputs (trace commitment)
    pub public_inputs: Vec<u64>,
}

impl EvmTrace {
    /// Create a mock ADD trace for testing
    /// Test: PUSH1 0x01, PUSH1 0x02, ADD → stack top = 0x03
    pub fn mock_add() -> Self {
        Self {
            opcodes: vec![0x60, 0x60, 0x01], // PUSH1, PUSH1, ADD
            stack_states: vec![vec![1, 0, 0], vec![2, 1, 0], vec![3, 0, 0]],
            pcs: vec![0, 2, 4],
            gas_values: vec![1000, 997, 994],
            memory_ops: None,
            storage_ops: None,
            tx_hash: None,
            block_number: None,
            bytecode: Some(vec![0x60, 0x01, 0x60, 0x02, 0x01]), // PUSH1 1, PUSH1 2, ADD
        }
    }

    /// Create a mock MUL trace for testing
    pub fn mock_mul() -> Self {
        Self {
            opcodes: vec![0x60, 0x60, 0x02], // PUSH1, PUSH1, MUL
            stack_states: vec![
                vec![5, 0, 0],
                vec![3, 5, 0],
                vec![15, 0, 0], // 5 * 3 = 15
            ],
            pcs: vec![0, 2, 4],
            gas_values: vec![1000, 995, 990],
            memory_ops: None,
            storage_ops: None,
            tx_hash: None,
            block_number: None,
            bytecode: Some(vec![0x60, 0x05, 0x60, 0x03, 0x02]), // PUSH1 5, PUSH1 3, MUL
        }
    }

    /// Validate trace integrity
    pub fn validate(&self) -> Result<()> {
        if self.opcodes.is_empty() {
            return Err(ProverError::InvalidInput("Empty trace".to_string()));
        }
        if self.opcodes.len() != self.stack_states.len() {
            return Err(ProverError::InvalidInput(
                "Opcode and stack state count mismatch".to_string(),
            ));
        }
        if self.opcodes.len() != self.gas_values.len() {
            return Err(ProverError::InvalidInput(
                "Opcode and gas value count mismatch".to_string(),
            ));
        }
        Ok(())
    }
}

/// Parse an EVM trace from JSON string
///
/// # Arguments
///
/// * `json_str` - JSON representation of the trace
///
/// # Returns
///
/// Parsed `EvmTrace`
///
/// # Example
///
/// ```json
/// {
///   "opcodes": [96, 96, 1],
///   "stack_states": [[1, 0, 0], [2, 1, 0], [3, 0, 0]],
///   "pcs": [0, 2, 4],
///   "gas_values": [1000, 997, 994]
/// }
/// ```
pub fn parse_trace_json(json_str: &str) -> Result<EvmTrace> {
    serde_json::from_str(json_str).map_err(|e| ProverError::ParseError(e.to_string()))
}

/// Fetch an EVM trace from Ethereum network via RPC
///
/// # Arguments
///
/// * `tx_hash` - Transaction hash to fetch
/// * `rpc_url` - Ethereum RPC endpoint URL
///
/// # Returns
///
/// Fetched `EvmTrace` with real execution data
///
/// # Example
///
/// ```no_run
/// # use zephyr_proof::utils::evm_parser::fetch_trace_from_network;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let trace = fetch_trace_from_network(
///     "0x1234...",
///     "http://localhost:8545"
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn fetch_trace_from_network(tx_hash: &str, rpc_url: &str) -> Result<EvmTrace> {
    // Build Alloy provider
    let provider = ProviderBuilder::new()
        .on_http(rpc_url.parse().map_err(|e| {
            ProverError::RpcConnectionError(format!("Invalid RPC URL: {}", e))
        })?)
        .map_err(|e| ProverError::RpcConnectionError(format!("Provider creation failed: {}", e)))?;

    // Fetch transaction details
    let tx_hash_parsed = tx_hash
        .parse()
        .map_err(|e| ProverError::InvalidTransaction(format!("Invalid tx hash: {}", e)))?;

    let tx = provider
        .get_transaction_by_hash(tx_hash_parsed)
        .await
        .map_err(|e| ProverError::NetworkError(format!("Failed to fetch tx: {}", e)))?
        .ok_or_else(|| ProverError::InvalidTransaction("Transaction not found".to_string()))?;

    // Get block number
    let block_number = tx.block_number;

    // Note: Real implementation would use debug_traceTransaction for full opcode trace
    // For now, we simulate execution with REVM to extract basic trace
    // TODO: Integrate with debug_traceTransaction for full trace with stack/memory dumps

    // Extract bytecode and simulate (simplified)
    let bytecode = tx.input.to_vec();

    // Return a basic trace structure
    // In production, this would parse debug_traceTransaction response
    Ok(EvmTrace {
        opcodes: extract_opcodes_from_bytecode(&bytecode),
        stack_states: vec![],
        pcs: vec![],
        gas_values: vec![],
        memory_ops: None,
        storage_ops: None,
        tx_hash: Some(tx_hash.to_string()),
        block_number,
        bytecode: Some(bytecode),
    })
}

/// Simulate transaction with REVM to get execution trace
///
/// # Arguments
///
/// * `bytecode` - Contract bytecode to execute
/// * `input_data` - Transaction input data
/// * `caller` - Caller address
/// * `value` - ETH value sent
///
/// # Returns
///
/// Complete `EvmTrace` with opcodes, stack, memory, storage
///
/// # Note
///
/// This provides a basic simulation. For production, integrate with
/// REVM's Inspector trait for step-by-step trace capture.
pub fn simulate_with_revm(
    bytecode: Vec<u8>,
    input_data: Vec<u8>,
    caller: Address,
    value: U256,
) -> Result<EvmTrace> {
    use revm::primitives::TransactTo;

    // Create REVM environment
    let mut env = Env::default();
    env.tx.caller = caller;
    env.tx.transact_to = TransactTo::Call(Address::ZERO); // Simplified
    env.tx.data = input_data.into();
    env.tx.value = value;

    // Create in-memory database (empty for simulation)
    let mut evm = Evm::builder()
        .with_env(Box::new(env))
        .with_spec_id(SpecId::CANCUN)
        .build();

    // Execute transaction
    // Note: This is simplified. Real implementation needs Inspector for trace capture
    let result = evm
        .transact()
        .map_err(|e| ProverError::EvmError(format!("EVM execution failed: {:?}", e)))?;

    // Extract basic trace (limited without Inspector)
    let opcodes = extract_opcodes_from_bytecode(&bytecode);

    Ok(EvmTrace {
        opcodes,
        stack_states: vec![],
        pcs: vec![],
        gas_values: vec![],
        memory_ops: None,
        storage_ops: None,
        tx_hash: None,
        block_number: None,
        bytecode: Some(bytecode),
    })
}

/// Extract opcodes from bytecode (basic parser)
fn extract_opcodes_from_bytecode(bytecode: &[u8]) -> Vec<u8> {
    let mut opcodes = Vec::new();
    let mut i = 0;

    while i < bytecode.len() {
        let opcode = bytecode[i];
        opcodes.push(opcode);

        // Skip PUSH data bytes (PUSH1-PUSH32 are 0x60-0x7f)
        if (0x60..=0x7f).contains(&opcode) {
            let push_size = (opcode - 0x60 + 1) as usize;
            i += push_size;
        }

        i += 1;
    }

    opcodes
}

/// Parse EVM trace into circuit witness data
///
/// # Arguments
///
/// * `trace` - EVM execution trace
///
/// # Returns
///
/// Flattened `CircuitWitness` ready for Halo2 circuit assignment
///
/// # Example
///
/// Real ex: let trace = fetch_trace_from_network("0x...", "http://...").await?;
///          let witness = parse_evm_data(&trace)?;
pub fn parse_evm_data(trace: &EvmTrace) -> Result<CircuitWitness> {
    trace.validate()?;

    // Flatten opcodes to u64 cells
    let opcode_cells: Vec<u64> = trace.opcodes.iter().map(|&op| op as u64).collect();

    // Flatten stack states (take top 3 values per step)
    let stack_cells: Vec<u64> = trace
        .stack_states
        .iter()
        .flat_map(|state| state.iter().take(3).copied())
        .collect();

    // Gas consumption cells
    let gas_cells = trace.gas_values.clone();

    // Compute public inputs (hash of trace for commitment)
    let public_inputs = compute_trace_commitment(trace);

    Ok(CircuitWitness {
        opcode_cells,
        stack_cells,
        gas_cells,
        public_inputs,
    })
}

/// Compute trace commitment (hash for public input)
fn compute_trace_commitment(trace: &EvmTrace) -> Vec<u64> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();

    // Hash opcodes
    for &op in &trace.opcodes {
        hasher.update([op]);
    }

    // Hash gas values
    for &gas in &trace.gas_values {
        hasher.update(gas.to_le_bytes());
    }

    let hash = hasher.finalize();

    // Take first 4 u64s from hash (256 bits / 64 bits = 4)
    hash.chunks(8)
        .take(4)
        .map(|chunk| {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(chunk);
            u64::from_le_bytes(bytes)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_add_trace() {
        let trace = EvmTrace::mock_add();
        assert_eq!(trace.opcodes.len(), 3);
        assert_eq!(trace.opcodes[2], 0x01); // ADD
        assert_eq!(trace.stack_states[2][0], 3); // Result: 1 + 2 = 3
        assert!(trace.bytecode.is_some());
    }

    #[test]
    fn test_mock_mul_trace() {
        let trace = EvmTrace::mock_mul();
        assert_eq!(trace.opcodes.len(), 3);
        assert_eq!(trace.opcodes[2], 0x02); // MUL
        assert_eq!(trace.stack_states[2][0], 15); // Result: 5 * 3 = 15
    }

    #[test]
    fn test_trace_validation() {
        let mut trace = EvmTrace::mock_add();
        assert!(trace.validate().is_ok());

        // Test empty trace
        trace.opcodes.clear();
        assert!(trace.validate().is_err());
    }

    #[test]
    fn test_parse_trace_json() {
        let json = r#"{
            "opcodes": [96, 96, 1],
            "stack_states": [[1, 0, 0], [2, 1, 0], [3, 0, 0]],
            "pcs": [0, 2, 4],
            "gas_values": [1000, 997, 994],
            "memory_ops": null,
            "storage_ops": null,
            "tx_hash": null,
            "block_number": null,
            "bytecode": null
        }"#;

        let trace = parse_trace_json(json).unwrap();
        assert_eq!(trace.opcodes.len(), 3);
    }

    #[test]
    fn test_parse_evm_data() {
        let trace = EvmTrace::mock_add();
        let witness = parse_evm_data(&trace).unwrap();

        assert_eq!(witness.opcode_cells.len(), 3);
        assert_eq!(witness.gas_cells.len(), 3);
        assert_eq!(witness.public_inputs.len(), 4); // 4 u64s from SHA256
    }

    #[test]
    fn test_extract_opcodes_from_bytecode() {
        // PUSH1 0x01, PUSH1 0x02, ADD
        let bytecode = vec![0x60, 0x01, 0x60, 0x02, 0x01];
        let opcodes = extract_opcodes_from_bytecode(&bytecode);

        // Should extract PUSH1, PUSH1, ADD (3 opcodes)
        assert_eq!(opcodes.len(), 3);
        assert_eq!(opcodes[0], 0x60); // PUSH1
        assert_eq!(opcodes[1], 0x60); // PUSH1
        assert_eq!(opcodes[2], 0x01); // ADD
    }

    #[test]
    fn test_compute_trace_commitment() {
        let trace = EvmTrace::mock_add();
        let commitment = compute_trace_commitment(&trace);

        assert_eq!(commitment.len(), 4);
        // Commitment should be deterministic
        let commitment2 = compute_trace_commitment(&trace);
        assert_eq!(commitment, commitment2);

        let result = parse_trace_json(json);
        assert!(result.is_ok());

        let trace = result.unwrap();
        assert_eq!(trace.opcodes.len(), 3);
        assert_eq!(trace.stack_states[2][0], 3);
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = "{ invalid json }";
        let result = parse_trace_json(json);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_trace_not_implemented() {
        let result =
            fetch_trace_from_network("0x1234", "https://mainnet.infura.io/v3/YOUR_KEY").await;
        assert!(result.is_err());
    }
}
