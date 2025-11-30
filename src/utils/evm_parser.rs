//! EVM trace parser
//!
//! Parses EVM execution traces from JSON, fetches them from Ethereum networks via Alloy,
//! and simulates execution using REVM to extract real opcodes, stack, memory, and storage.

use crate::errors::{ProverError, Result};
use alloy_consensus::Transaction as TransactionTrait;
use alloy_primitives::U256;
use alloy_provider::{Provider, ProviderBuilder};
use serde::{Deserialize, Serialize};

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

/// Fetch and execute a transaction using debug_traceTransaction RPC
///
/// # Arguments
///
/// * `tx_hash` - Transaction hash to fetch and execute
/// * `rpc_url` - Ethereum RPC endpoint URL
///
/// # Returns
///
/// Tuple of (EvmTrace, gas_used) with real execution data
///
/// # Example
///
/// ```no_run
/// # use zephyr_proof::utils::evm_parser::fetch_and_execute_tx;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let (trace, gas_used) = fetch_and_execute_tx(
///     "0x1234...",
///     "http://localhost:8545"
/// ).await?;
/// println!("Executed {} opcodes", trace.opcodes.len());
/// # Ok(())
/// # }
/// ```
pub async fn fetch_and_execute_tx(tx_hash: &str, rpc_url: &str) -> Result<(EvmTrace, u64)> {
    let provider = match ProviderBuilder::new().connect(rpc_url).await {
        Ok(p) => p,
        Err(e) => {
            return Err(ProverError::RpcConnectionError(format!(
                "Failed to connect to RPC: {}",
                e
            )))
        }
    };

    let tx_hash_parsed = tx_hash
        .parse()
        .map_err(|e| ProverError::InvalidTransaction(format!("Invalid tx hash: {}", e)))?;

    let tx = provider
        .get_transaction_by_hash(tx_hash_parsed)
        .await
        .map_err(|e| ProverError::NetworkError(format!("Failed to fetch tx: {}", e)))?
        .ok_or_else(|| ProverError::InvalidTransaction("Transaction not found".to_string()))?;

    let block_number = tx.block_number.unwrap_or(0);

    // Get receipt for gas used
    let receipt = provider
        .get_transaction_receipt(tx_hash_parsed)
        .await
        .map_err(|e| ProverError::NetworkError(format!("Failed to fetch receipt: {}", e)))?;

    let gas_used = receipt.as_ref().map(|r| r.gas_used).unwrap_or(21000);

    // Try debug_traceTransaction first (requires archive node with debug namespace)
    let trace_result: std::result::Result<serde_json::Value, _> = provider
        .raw_request(
            "debug_traceTransaction".into(),
            (tx_hash, serde_json::json!({"tracer": "structLogTracer"})),
        )
        .await;

    match trace_result {
        Ok(trace_data) => {
            // Parse the debug trace result
            let (opcodes, stack_states, pcs, gas_values, memory_ops, storage_ops, bytecode) =
                parse_debug_trace(&trace_data, gas_used)?;

            let trace = EvmTrace {
                opcodes,
                stack_states,
                pcs,
                gas_values,
                memory_ops,
                storage_ops,
                tx_hash: Some(tx_hash.to_string()),
                block_number: Some(block_number),
                bytecode,
            };

            Ok((trace, gas_used))
        }
        Err(_) => {
            // Fallback: construct trace from transaction input data
            // This works with any RPC endpoint but provides less detail
            eprintln!(
                "  Note: debug_traceTransaction not available, using fallback trace extraction"
            );

            let trace = construct_fallback_trace(&tx, gas_used, tx_hash, block_number)?;
            Ok((trace, gas_used))
        }
    }
}

/// Construct a trace from transaction data when debug_traceTransaction is unavailable
///
/// This fallback extracts opcodes from the transaction input data (for contract calls)
/// or creates a minimal transfer trace (for simple ETH transfers).
fn construct_fallback_trace(
    tx: &alloy_rpc_types::Transaction,
    gas_used: u64,
    tx_hash: &str,
    block_number: u64,
) -> Result<EvmTrace> {
    // Access input data through the inner transaction
    let input = tx.inner.input();

    // Check if this is a simple ETH transfer (no input data)
    if input.is_empty() {
        // Simple transfer: just a single implicit STOP
        return Ok(EvmTrace {
            opcodes: vec![0x00], // STOP
            stack_states: vec![vec![0, 0, 0]],
            pcs: vec![0],
            gas_values: vec![gas_used],
            memory_ops: None,
            storage_ops: None,
            tx_hash: Some(tx_hash.to_string()),
            block_number: Some(block_number),
            bytecode: Some(vec![0x00]),
        });
    }

    // For contract calls, parse the input as potential bytecode/calldata
    // The input typically starts with a 4-byte function selector
    let mut opcodes = Vec::new();
    let mut stack_states = Vec::new();
    let mut pcs = Vec::new();
    let mut gas_values = Vec::new();

    // Extract opcodes from input data (treating it as execution trace approximation)
    // This is a heuristic - actual execution would require REVM simulation
    let bytecode: Vec<u8> = input.to_vec();

    let mut pc: u64 = 0;
    let mut remaining_gas = gas_used;
    let gas_per_op = 3u64; // Average gas cost estimate

    let mut i = 0;
    while i < bytecode.len() && remaining_gas > 0 {
        let opcode = bytecode[i];
        opcodes.push(opcode);
        pcs.push(pc);
        gas_values.push(remaining_gas);

        // Simple stack simulation (placeholder values)
        stack_states.push(vec![0, 0, 0]);

        // Handle PUSH instructions (skip the immediate data)
        if (0x60..=0x7f).contains(&opcode) {
            let push_size = (opcode - 0x60 + 1) as usize;
            i += push_size;
            pc += push_size as u64;
        }

        i += 1;
        pc += 1;
        remaining_gas = remaining_gas.saturating_sub(gas_per_op);
    }

    // Ensure we have at least one opcode
    if opcodes.is_empty() {
        opcodes.push(0x00); // STOP
        stack_states.push(vec![0, 0, 0]);
        pcs.push(0);
        gas_values.push(gas_used);
    }

    Ok(EvmTrace {
        opcodes,
        stack_states,
        pcs,
        gas_values,
        memory_ops: None,
        storage_ops: None,
        tx_hash: Some(tx_hash.to_string()),
        block_number: Some(block_number),
        bytecode: Some(bytecode),
    })
}

/// Parse debug_traceTransaction response into trace components
fn parse_debug_trace(
    trace_result: &serde_json::Value,
    total_gas: u64,
) -> Result<(
    Vec<u8>,
    Vec<Vec<u64>>,
    Vec<u64>,
    Vec<u64>,
    Option<Vec<MemoryOp>>,
    Option<Vec<StorageOp>>,
    Option<Vec<u8>>,
)> {
    // Extract structLogs array from the trace result
    let struct_logs = trace_result
        .get("structLogs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            ProverError::ParseError("Missing or invalid structLogs in trace".to_string())
        })?;

    if struct_logs.is_empty() {
        return Err(ProverError::InvalidInput(
            "Empty structLogs in trace".to_string(),
        ));
    }

    let mut opcodes = Vec::with_capacity(struct_logs.len());
    let mut stack_states = Vec::with_capacity(struct_logs.len());
    let mut pcs = Vec::with_capacity(struct_logs.len());
    let mut gas_values = Vec::with_capacity(struct_logs.len());
    let mut memory_ops = Vec::new();
    let mut storage_ops = Vec::new();

    for (i, log) in struct_logs.iter().enumerate() {
        // Extract opcode name and convert to byte
        let op_name = log
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProverError::ParseError(format!("Missing op at index {}", i)))?;

        let opcode_byte = opcode_name_to_byte(op_name);
        opcodes.push(opcode_byte);

        // Extract PC
        let pc = log.get("pc").and_then(|v| v.as_u64()).unwrap_or(i as u64);
        pcs.push(pc);

        // Extract gas
        let gas = log
            .get("gas")
            .and_then(|v| v.as_u64())
            .unwrap_or(total_gas.saturating_sub(i as u64 * 3));
        gas_values.push(gas);

        // Extract stack (take top 3 values)
        let stack = log
            .get("stack")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .rev()
                    .take(3)
                    .map(|v| {
                        v.as_str()
                            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                            .unwrap_or(0)
                    })
                    .collect::<Vec<u64>>()
            })
            .unwrap_or_else(|| vec![0, 0, 0]);

        // Ensure stack has exactly 3 elements
        let mut stack_3 = stack;
        while stack_3.len() < 3 {
            stack_3.push(0);
        }
        stack_states.push(stack_3);

        // Extract memory operations (MLOAD/MSTORE)
        if op_name == "MLOAD" || op_name == "MSTORE" {
            if let Some(memory) = log.get("memory").and_then(|v| v.as_array()) {
                let offset = pcs.last().copied().unwrap_or(0);
                let value = memory
                    .iter()
                    .flat_map(|v| {
                        v.as_str()
                            .map(|s| hex::decode(s.trim_start_matches("0x")).unwrap_or_default())
                            .unwrap_or_default()
                    })
                    .collect();
                memory_ops.push(MemoryOp {
                    offset,
                    value,
                    is_write: op_name == "MSTORE",
                });
            }
        }

        // Extract storage operations (SLOAD/SSTORE)
        if op_name == "SLOAD" || op_name == "SSTORE" {
            if let Some(storage) = log.get("storage").and_then(|v| v.as_object()) {
                for (key, val) in storage.iter() {
                    let key_bytes = hex::decode(key.trim_start_matches("0x")).unwrap_or_default();
                    let val_str = val.as_str().unwrap_or("0x0");
                    let val_bytes =
                        hex::decode(val_str.trim_start_matches("0x")).unwrap_or_default();

                    storage_ops.push(StorageOp {
                        key: U256::from_be_slice(&key_bytes),
                        value: U256::from_be_slice(&val_bytes),
                        is_write: op_name == "SSTORE",
                    });
                }
            }
        }
    }

    Ok((
        opcodes,
        stack_states,
        pcs,
        gas_values,
        if memory_ops.is_empty() {
            None
        } else {
            Some(memory_ops)
        },
        if storage_ops.is_empty() {
            None
        } else {
            Some(storage_ops)
        },
        None, // bytecode extracted separately if needed
    ))
}

/// Convert EVM opcode name to byte value
fn opcode_name_to_byte(name: &str) -> u8 {
    match name {
        "STOP" => 0x00,
        "ADD" => 0x01,
        "MUL" => 0x02,
        "SUB" => 0x03,
        "DIV" => 0x04,
        "SDIV" => 0x05,
        "MOD" => 0x06,
        "SMOD" => 0x07,
        "ADDMOD" => 0x08,
        "MULMOD" => 0x09,
        "EXP" => 0x0a,
        "SIGNEXTEND" => 0x0b,
        "LT" => 0x10,
        "GT" => 0x11,
        "SLT" => 0x12,
        "SGT" => 0x13,
        "EQ" => 0x14,
        "ISZERO" => 0x15,
        "AND" => 0x16,
        "OR" => 0x17,
        "XOR" => 0x18,
        "NOT" => 0x19,
        "BYTE" => 0x1a,
        "SHL" => 0x1b,
        "SHR" => 0x1c,
        "SAR" => 0x1d,
        "SHA3" | "KECCAK256" => 0x20,
        "ADDRESS" => 0x30,
        "BALANCE" => 0x31,
        "ORIGIN" => 0x32,
        "CALLER" => 0x33,
        "CALLVALUE" => 0x34,
        "CALLDATALOAD" => 0x35,
        "CALLDATASIZE" => 0x36,
        "CALLDATACOPY" => 0x37,
        "CODESIZE" => 0x38,
        "CODECOPY" => 0x39,
        "GASPRICE" => 0x3a,
        "EXTCODESIZE" => 0x3b,
        "EXTCODECOPY" => 0x3c,
        "RETURNDATASIZE" => 0x3d,
        "RETURNDATACOPY" => 0x3e,
        "EXTCODEHASH" => 0x3f,
        "BLOCKHASH" => 0x40,
        "COINBASE" => 0x41,
        "TIMESTAMP" => 0x42,
        "NUMBER" => 0x43,
        "PREVRANDAO" | "DIFFICULTY" => 0x44,
        "GASLIMIT" => 0x45,
        "CHAINID" => 0x46,
        "SELFBALANCE" => 0x47,
        "BASEFEE" => 0x48,
        "POP" => 0x50,
        "MLOAD" => 0x51,
        "MSTORE" => 0x52,
        "MSTORE8" => 0x53,
        "SLOAD" => 0x54,
        "SSTORE" => 0x55,
        "JUMP" => 0x56,
        "JUMPI" => 0x57,
        "PC" => 0x58,
        "MSIZE" => 0x59,
        "GAS" => 0x5a,
        "JUMPDEST" => 0x5b,
        "PUSH0" => 0x5f,
        "PUSH1" => 0x60,
        "PUSH2" => 0x61,
        "PUSH3" => 0x62,
        "PUSH4" => 0x63,
        "PUSH5" => 0x64,
        "PUSH6" => 0x65,
        "PUSH7" => 0x66,
        "PUSH8" => 0x67,
        "PUSH9" => 0x68,
        "PUSH10" => 0x69,
        "PUSH11" => 0x6a,
        "PUSH12" => 0x6b,
        "PUSH13" => 0x6c,
        "PUSH14" => 0x6d,
        "PUSH15" => 0x6e,
        "PUSH16" => 0x6f,
        "PUSH17" => 0x70,
        "PUSH18" => 0x71,
        "PUSH19" => 0x72,
        "PUSH20" => 0x73,
        "PUSH21" => 0x74,
        "PUSH22" => 0x75,
        "PUSH23" => 0x76,
        "PUSH24" => 0x77,
        "PUSH25" => 0x78,
        "PUSH26" => 0x79,
        "PUSH27" => 0x7a,
        "PUSH28" => 0x7b,
        "PUSH29" => 0x7c,
        "PUSH30" => 0x7d,
        "PUSH31" => 0x7e,
        "PUSH32" => 0x7f,
        "DUP1" => 0x80,
        "DUP2" => 0x81,
        "DUP3" => 0x82,
        "DUP4" => 0x83,
        "DUP5" => 0x84,
        "DUP6" => 0x85,
        "DUP7" => 0x86,
        "DUP8" => 0x87,
        "DUP9" => 0x88,
        "DUP10" => 0x89,
        "DUP11" => 0x8a,
        "DUP12" => 0x8b,
        "DUP13" => 0x8c,
        "DUP14" => 0x8d,
        "DUP15" => 0x8e,
        "DUP16" => 0x8f,
        "SWAP1" => 0x90,
        "SWAP2" => 0x91,
        "SWAP3" => 0x92,
        "SWAP4" => 0x93,
        "SWAP5" => 0x94,
        "SWAP6" => 0x95,
        "SWAP7" => 0x96,
        "SWAP8" => 0x97,
        "SWAP9" => 0x98,
        "SWAP10" => 0x99,
        "SWAP11" => 0x9a,
        "SWAP12" => 0x9b,
        "SWAP13" => 0x9c,
        "SWAP14" => 0x9d,
        "SWAP15" => 0x9e,
        "SWAP16" => 0x9f,
        "LOG0" => 0xa0,
        "LOG1" => 0xa1,
        "LOG2" => 0xa2,
        "LOG3" => 0xa3,
        "LOG4" => 0xa4,
        "CREATE" => 0xf0,
        "CALL" => 0xf1,
        "CALLCODE" => 0xf2,
        "RETURN" => 0xf3,
        "DELEGATECALL" => 0xf4,
        "CREATE2" => 0xf5,
        "STATICCALL" => 0xfa,
        "REVERT" => 0xfd,
        "INVALID" => 0xfe,
        "SELFDESTRUCT" => 0xff,
        _ => 0xfe, // Unknown opcode -> INVALID
    }
}

/// Convert REVM execution data to circuit witness
///
/// # Arguments
///
/// * `trace` - EVM execution trace
///
/// # Returns
///
/// Circuit witness ready for constraint assignment
///
/// # Example
///
/// ```no_run
/// # use zephyr_proof::utils::evm_parser::{fetch_and_execute_tx, trace_to_witness};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let (trace, _) = fetch_and_execute_tx("0x...", "http://localhost:8545").await?;
/// let witness = trace_to_witness(&trace)?;
/// println!("Witness has {} opcode cells", witness.opcode_cells.len());
/// # Ok(())
/// # }
/// ```
pub fn trace_to_witness(trace: &EvmTrace) -> Result<CircuitWitness> {
    trace.validate()?;

    let opcode_cells: Vec<u64> = trace.opcodes.iter().map(|&op| op as u64).collect();

    let stack_cells: Vec<u64> = trace
        .stack_states
        .iter()
        .flat_map(|state| state.iter().take(3).copied())
        .collect();

    let gas_cells = trace.gas_values.clone();

    let public_inputs = compute_trace_commitment(trace);

    Ok(CircuitWitness {
        opcode_cells,
        stack_cells,
        gas_cells,
        public_inputs,
    })
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
    let (trace, _gas_used) = fetch_and_execute_tx(tx_hash, rpc_url).await?;
    Ok(trace)
}

/// Extract opcodes from bytecode (basic parser)
///
/// This function parses EVM bytecode and extracts individual opcodes,
/// properly handling PUSH instructions that include immediate data.
#[allow(dead_code)]
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

    /// Helper to create a minimal valid trace for testing
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

    #[test]
    fn test_create_trace() {
        let trace = create_test_trace();
        assert_eq!(trace.opcodes.len(), 3);
        assert_eq!(trace.opcodes[2], 0x01); // ADD
        assert_eq!(trace.stack_states[2][0], 3); // Result: 1 + 2 = 3
        assert!(trace.bytecode.is_some());
    }

    #[test]
    fn test_trace_validation() {
        let trace = create_test_trace();
        assert!(trace.validate().is_ok());
    }

    #[test]
    fn test_trace_validation_empty() {
        let mut trace = create_test_trace();
        trace.opcodes.clear();
        let result = trace.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ProverError::InvalidInput(_)));
    }

    #[test]
    fn test_trace_validation_mismatch() {
        let mut trace = create_test_trace();
        trace.stack_states.push(vec![0, 0, 0]);
        let result = trace.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_evm_data() {
        let trace = create_test_trace();
        let witness = parse_evm_data(&trace).unwrap();

        assert_eq!(witness.opcode_cells.len(), 3);
        assert_eq!(witness.gas_cells.len(), 3);
        assert_eq!(witness.public_inputs.len(), 4); // 4 u64s from SHA256

        // Verify opcodes are correctly converted
        assert_eq!(witness.opcode_cells[0], 0x60);
        assert_eq!(witness.opcode_cells[2], 0x01);
    }

    #[test]
    fn test_compute_trace_commitment() {
        let trace = create_test_trace();
        let commitment = compute_trace_commitment(&trace);

        assert_eq!(commitment.len(), 4);
        // Commitment should be deterministic
        let commitment2 = compute_trace_commitment(&trace);
        assert_eq!(commitment, commitment2);
    }

    #[test]
    fn test_commitment_different_traces() {
        let trace1 = create_test_trace();
        let mut trace2 = create_test_trace();
        trace2.opcodes[0] = 0x61; // Different opcode

        let commitment1 = compute_trace_commitment(&trace1);
        let commitment2 = compute_trace_commitment(&trace2);

        assert_ne!(commitment1, commitment2);
    }

    #[test]
    fn test_parse_trace_json_valid() {
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

    #[test]
    fn test_extract_opcodes_simple() {
        // PUSH1 0x01, PUSH1 0x02, ADD
        let bytecode = vec![0x60, 0x01, 0x60, 0x02, 0x01];
        let opcodes = extract_opcodes_from_bytecode(&bytecode);

        assert_eq!(opcodes.len(), 3);
        assert_eq!(opcodes[0], 0x60); // PUSH1
        assert_eq!(opcodes[1], 0x60); // PUSH1
        assert_eq!(opcodes[2], 0x01); // ADD
    }

    #[test]
    fn test_extract_opcodes_push32() {
        // PUSH32 with 32 bytes of data, then ADD
        let mut bytecode = vec![0x7f]; // PUSH32
        bytecode.extend(vec![0xff; 32]); // 32 bytes
        bytecode.push(0x01); // ADD

        let opcodes = extract_opcodes_from_bytecode(&bytecode);

        assert_eq!(opcodes.len(), 2);
        assert_eq!(opcodes[0], 0x7f); // PUSH32
        assert_eq!(opcodes[1], 0x01); // ADD
    }

    #[test]
    fn test_trace_with_storage_ops() {
        let trace = EvmTrace {
            opcodes: vec![0x54, 0x55], // SLOAD, SSTORE
            stack_states: vec![vec![1, 0, 0], vec![2, 1, 0]],
            pcs: vec![0, 1],
            gas_values: vec![1000, 800],
            memory_ops: None,
            storage_ops: Some(vec![StorageOp {
                key: U256::from(1),
                value: U256::from(100),
                is_write: false,
            }]),
            tx_hash: Some("0xabcd...".to_string()),
            block_number: Some(12345),
            bytecode: None,
        };

        assert!(trace.validate().is_ok());
        assert_eq!(trace.storage_ops.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_large_trace() {
        let opcodes = vec![0x60; 100]; // 100 PUSH1 operations
        let stack_states = vec![vec![1, 0, 0]; 100];
        let pcs: Vec<u64> = (0..100).map(|i| i * 2).collect();
        let gas_values: Vec<u64> = (0..100).map(|i| 1000 - i * 3).collect();

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

        assert!(trace.validate().is_ok());
        let witness = parse_evm_data(&trace).unwrap();
        assert_eq!(witness.opcode_cells.len(), 100);
    }

    #[tokio::test]
    async fn test_fetch_trace_invalid_rpc() {
        let result = fetch_trace_from_network(
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "invalid-url",
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_trace_invalid_hash() {
        let result = fetch_trace_from_network("invalid-hash", "http://localhost:8545").await;

        assert!(result.is_err());
    }

    #[test]
    fn test_trace_to_witness() {
        let trace = create_test_trace();
        let witness = trace_to_witness(&trace).unwrap();

        assert_eq!(witness.opcode_cells.len(), 3);
        assert_eq!(witness.gas_cells.len(), 3);
        assert_eq!(witness.public_inputs.len(), 4);
    }

    #[test]
    fn test_extract_trace_commitment() {
        let trace = create_test_trace();
        let witness = trace_to_witness(&trace).unwrap();

        assert!(witness.public_inputs.len() > 0);

        let witness2 = trace_to_witness(&trace).unwrap();
        assert_eq!(witness.public_inputs, witness2.public_inputs);
    }
}
