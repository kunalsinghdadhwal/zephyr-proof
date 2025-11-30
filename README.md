# Zephyr-Proof

Production-grade modular Halo2 zkEVM prover for real Ethereum transaction traces. Generates succinct zero-knowledge proofs of EVM execution using REVM simulation and Alloy RPC.

![Untitled diagram-2025-11-30-194002](https://github.com/user-attachments/assets/6d4e7dbd-e74f-4803-b1ce-296883e7aa2c)
## Overview

Zephyr-Proof transforms Ethereum transactions into verifiable Halo2 proofs without compromising accuracy or performance. Built for zk-rollups, privacy-preserving bridges, and on-chain verification of off-chain computations.

### Core Capabilities

- Fetch real transaction traces via Alloy RPC using `debug_traceTransaction` (with fallback for public RPCs)
- Modular chip architecture proving selective opcodes with gas metering and stack validation
- Parallel proving with Rayon and recursive aggregation for traces exceeding 1M steps
- Base64 proof output with metadata for on-chain settlement
- WASM-compatible library for integration

## Installation

```bash
git clone https://github.com/kunalsinghdadhwal/zephyr-proof
cd zephyr-proof
cargo build --release
```

### Build Targets

```bash
# Library only
cargo build --release --lib

# Main CLI
cargo build --release --bin zkevm-prover

# Verifier CLI
cargo build --release --bin verifier-cli --features cli

# Benchmarks
cargo build --release --bin benchmark --features bench
```

## Usage

### Simulate and Prove a Real Transaction

```bash
./target/release/zkevm-prover simulate <TX_HASH> \
  --rpc-url https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY \
  -o proof.json
```

### Prove from Trace File

```bash
./target/release/zkevm-prover prove trace.json -o proof.json
```

### Verify a Proof

```bash
./target/release/zkevm-prover verify proof.json
```

### Fetch Trace Only

```bash
./target/release/zkevm-prover fetch <TX_HASH> \
  --rpc-url http://localhost:8545 \
  -o trace.json
```

### CLI Options

```
zkevm-prover [OPTIONS] <COMMAND>

Options:
  -k, --k <K>              Circuit size as power of 2 [default: 17]
  --no-parallel            Disable parallel processing
  -t, --threads <N>        Thread count for Rayon

Commands:
  prove     Generate proof from trace JSON
  verify    Verify proof file
  simulate  Fetch transaction and generate proof
  fetch     Fetch trace without proving
```

## Library API

### Generate Proof from Trace

```rust
use zephyr_proof::{ProverConfig, generate_proof, verify_proof};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ProverConfig::default();
    
    let trace_json = r#"{
        "opcodes": [96, 96, 1],
        "stack_states": [[1,0,0], [2,1,0], [3,0,0]],
        "pcs": [0, 2, 4],
        "gas_values": [1000, 997, 994]
    }"#;
    
    let proof = generate_proof(trace_json, &config).await?;
    let valid = verify_proof(&proof, &config).await?;
    
    assert!(valid);
    Ok(())
}
```

### Fetch and Prove Real Transaction

```rust
use zephyr_proof::utils::evm_parser::{fetch_and_execute_tx, trace_to_witness};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (trace, gas_used) = fetch_and_execute_tx(
        "0x1234567890abcdef...",
        "http://localhost:8545"
    ).await?;
    
    let witness = trace_to_witness(&trace)?;
    
    // Use witness with EvmCircuit for proof generation
    Ok(())
}
```


### Execution Flow

```
Transaction Hash
       |
       v
[Alloy RPC] debug_traceTransaction (with fallback for public RPCs)
       |
       v
EvmTrace { opcodes, stack_states, gas_values, storage_ops }
       |
       v
[trace_to_witness] CircuitWitness with SHA256 commitment
       |
       v
[EvmCircuit] Configure chips, assign witnesses (num_steps preserved)
       |
       v
[generate_proof_parallel] Halo2 create_proof with Blake2b transcript
       |
       v
ProofOutput { proof, public_inputs, metadata, num_steps, k, vk_hash }
```

## Proof Output Structure

```rust
pub struct ProofOutput {
    pub proof: String,           // Base64-encoded Halo2 proof
    pub public_inputs: Vec<String>, // Trace commitment
    pub metadata: TraceInfo,     // Opcode count, gas used, tx hash
    pub num_steps: usize,        // Execution steps (for VK reconstruction)
    pub k: u32,                  // Circuit size parameter
    pub vk_hash: String,         // Verification key hash
}
```

The `num_steps` field is critical for verification. The verifier reconstructs the circuit structure using this value to generate a matching verification key.

## Supported Opcodes

| Category   | Opcodes                                      |
|------------|----------------------------------------------|
| Arithmetic | ADD, SUB, MUL, DIV, MOD, ADDMOD, MULMOD      |
| Comparison | LT, GT, EQ, ISZERO                           |
| Bitwise    | AND, OR, XOR, NOT, SHL, SHR                  |
| Stack      | POP, PUSH1-PUSH32, DUP1-DUP16, SWAP1-SWAP16  |
| Memory     | MLOAD, MSTORE, MSTORE8, MSIZE                |
| Storage    | SLOAD, SSTORE                                |
| Control    | JUMP, JUMPI, JUMPDEST, STOP, RETURN, REVERT  |
| Context    | ADDRESS, CALLER, CALLVALUE, CALLDATALOAD     |

Gas costs follow EIP-150 specifications.

## Configuration

```rust
ProverConfig {
    k: 17,                    // 2^17 = 131,072 rows
    parallel: true,           // Rayon parallelism
    num_threads: None,        // Auto-detect cores
    rpc_url: None,            // RPC endpoint
}
```

### Circuit Sizing

| k Value | Max Rows | Recommended Use       |
|---------|----------|-----------------------|
| 14      | 16,384   | Small traces (<1k ops)|
| 17      | 131,072  | Medium traces         |
| 20      | 1M+      | Large traces          |

Traces exceeding circuit capacity are automatically chunked.

## Proof System Details

### Proof Generation

Uses Halo2 v0.3.1 with Pasta curves (pallas/vesta). Proofs are generated using:

- `keygen_vk` and `keygen_pk` for key generation
- `create_proof` with Blake2bWrite transcript
- Proofs serialized as base64 for transport

### Verification

- Reconstructs circuit structure using `num_steps` from proof
- Generates matching VK with `keygen_vk` 
- `verify_proof` with SingleVerifier strategy
- Blake2bRead transcript for Fiat-Shamir
- Returns boolean validity

### Chunked Proving

For large traces:
1. Split trace into 2^14 row chunks
2. Generate sub-proofs in parallel via Rayon
3. Aggregate metadata across chunks
4. Future: Recursive SNARK aggregation


## Error Types

```rust
pub enum ProverError {
    RpcConnectionError(String),    // Network failures
    InvalidTransaction(String),    // Malformed tx data
    EvmError(String),              // Execution failures
    ProofGenerationError(String),  // Constraint violations
    VerificationError(String),     // Invalid proofs
    Halo2Error(String),            // Halo2 internal errors
    Base64Error(String),           // Encoding errors
}
```

## Author
[Kunal Singh Dadhwal](https://kunalsinghdev.xyz)  
