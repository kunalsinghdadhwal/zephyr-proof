---
applyTo: "**"
---

# Project Overview

This project is a Rust-based CLI tool for generating and verifying zero-knowledge proofs of Ethereum Virtual Machine (EVM) execution traces. It uses the Halo2 proof system integrated with REVM primitives to create efficient zkEVM proofs, supports Ethereum contract interactions via Ethers, and employs parallel processing with Rayon for performance optimization. The tool processes EVM traces, compiles Halo2 circuits with gadgets and curves, and outputs verifiable proofs for blockchain applications.

# Copilot Instructions

This project uses several Rust crates. Below are links to their documentation on [docs.rs](https://docs.rs), which Copilot can reference for accurate completions and context.

## Core Dependencies

### base64

- Docs: [https://docs.rs/base64/0.22.1/base64/](https://docs.rs/base64/0.22.1/base64/)
- Purpose: Encoding and decoding Base64 strings.

### clap

- Docs: [https://docs.rs/clap/4.5.50/clap/](https://docs.rs/clap/4.5.50/clap/)
- Features: `"derive"`
- Purpose: Command-line argument parsing with derive macros.

### ethers

- Docs: [https://docs.rs/ethers/2.0.14/ethers/](https://docs.rs/ethers/2.0.14/ethers/)
- Features: `"abigen"`, `"ws"`
- Purpose: Ethereum interaction library — provides contract bindings, providers, wallets, etc.

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

## Notes for Copilot

- Use these docs for API references and examples.
- When suggesting code:
  - Prefer safe Rust idioms and `?` for error propagation.
  - Follow the feature flags enabled above.
  - If uncertain about a type or method, look it up from the linked docs.rs pages.
