# Project Overview

This project is a Rust-based CLI tool for generating and verifying zero-knowledge proofs of Ethereum Virtual Machine (EVM) execution traces. It uses the Halo2 proof system integrated with REVM primitives to create efficient zkEVM proofs, supports Ethereum contract interactions via Alloy, and employs parallel processing with Rayon for performance optimization. The tool processes EVM traces, compiles Halo2 circuits with gadgets and curves, and outputs verifiable proofs for blockchain applications.

# Copilot Instructions

This project uses several Rust crates. Below are links to their documentation on [docs.rs](https://docs.rs), which Copilot can reference for accurate completions and context.

## Core Dependencies

### alloy-contract

- Docs: [https://docs.rs/alloy-contract/1.0.41/alloy_contract/](https://docs.rs/alloy-contract/1.0.41/alloy_contract/)
- Purpose: Interact with on-chain contracts, including call builders and contract interactions.

### alloy-primitives

- Docs: [https://docs.rs/alloy-primitives/1.4.1/alloy_primitives/](https://docs.rs/alloy-primitives/1.4.1/alloy_primitives/)
- Purpose: Primitive types shared across Alloy ecosystem, including unsigned integers and Ethereum-specific types.

### alloy-provider

- Docs: [https://docs.rs/alloy-provider/1.0.41/alloy_provider/](https://docs.rs/alloy-provider/1.0.41/alloy_provider/)
- Purpose: Ethereum JSON-RPC provider trait and implementations for network interactions.

### alloy-sol-macro

- Docs: [https://docs.rs/alloy-sol-macro/1.4.1/alloy_sol_macro/](https://docs.rs/alloy-sol-macro/1.4.1/alloy_sol_macro/)
- Purpose: Procedural macro for parsing Solidity syntax to generate Alloy-compatible types.

### base64

- Docs: [https://docs.rs/base64/0.22.1/base64/](https://docs.rs/base64/0.22.1/base64/)
- Purpose: Encoding and decoding Base64 strings.

### clap

- Docs: [https://docs.rs/clap/4.5.50/clap/](https://docs.rs/clap/4.5.50/clap/)
- Features: `"derive"`
- Purpose: Command-line argument parsing with derive macros.

### halo2_gadgets

- Docs: [https://docs.rs/halo2_gadgets/0.3.1/halo2_gadgets/](https://docs.rs/halo2_gadgets/0.3.1/halo2_gadgets/)
- Purpose: Common cryptographic gadgets for Halo 2 proving system.

### halo2_proofs

- Docs: [https://docs.rs/halo2_proofs/0.3.1/halo2_proofs/](https://docs.rs/halo2_proofs/0.3.1/halo2_proofs/)
- Purpose: Core Halo 2 proof system — constructs and verifies zero-knowledge proofs.

### halo2curves

- Docs: [https://docs.rs/halo2curves/0.9.0/halo2curves/](https://docs.rs/halo2curves/0.9.0/halo2curves/)
- Purpose: Provides elliptic curve implementations for Halo 2 circuits.

### rayon

- Docs: [https://docs.rs/rayon/1.11.0/rayon/](https://docs.rs/rayon/1.11.0/rayon/)
- Purpose: Parallel iterators and task scheduling for CPU-bound computations.

### revm-primitives

- Docs: [https://docs.rs/revm-primitives/21.0.1/revm_primitives/](https://docs.rs/revm-primitives/21.0.1/revm_primitives/)
- Features: `"serde"`
- Purpose: Core data structures and primitives used in the REVM (Rust EVM) project.

### serde

- Docs: [https://docs.rs/serde/1.0.210/serde/](https://docs.rs/serde/1.0.210/serde/)
- Features: `["derive"]`
- Purpose: Serialization and deserialization framework.

### serde_json

- Docs: [https://docs.rs/serde_json/1.0.132/serde_json/](https://docs.rs/serde_json/1.0.132/serde_json/)
- Purpose: JSON serialization and deserialization using Serde.

### thiserror

- Docs: [https://docs.rs/thiserror/1.0.64/thiserror/](https://docs.rs/thiserror/1.0.64/thiserror/)
- Purpose: Derive macro for implementing the std::error::Error trait.

### tokio

- Docs: [https://docs.rs/tokio/1.47.0/tokio/](https://docs.rs/tokio/1.47.0/tokio/)
- Features: `["full"]`
- Purpose: Asynchronous runtime for Rust, enabling async/await patterns.

## Notes for Copilot

- Use these docs for API references and examples.
- When suggesting code:
  - Prefer safe Rust idioms and `?` for error propagation.
  - Follow the feature flags enabled above.
  - If uncertain about a type or method, look it up from the linked docs.rs pages.
