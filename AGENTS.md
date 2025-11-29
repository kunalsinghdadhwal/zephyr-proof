# zephyr-proof  

## Project Description
**zephyr-proof** is a production-grade, modular Rust CLI toolkit for generating and verifying zero-knowledge proofs of real Ethereum Virtual Machine (EVM) execution traces. Built for zk-rollups, privacy-preserving bridges, and on-chain verification of off-chain computations, it transforms Ethereum transactions into succinct Halo2 proofs without compromising accuracy or performance.

**Core Purpose**:  
- Fetch real tx traces via Alloy RPC (no mocks—always simulate with REVM for 100% EVM fidelity).  
- Modular chips/circuits prove selective opcodes (ADD, MLOAD, SLOAD, etc.) with gas metering, stack/memory checks, and storage diffs.  
- Parallel proving (Rayon) + recursive aggregation for 1M+ step traces.  
- Outputs: Base64 proofs + metadata for on-chain settlement.  
- Targets: zkEVM devs, rollup builders, auditors—WASM-ready lib for integration.

**Why It Matters**: Enables fast, verifiable off-chain EVM execution, reducing L1 costs while maintaining trustless security. Start with `cargo run -- simulate <tx-hash> --rpc-url <url>` for end-to-end proofing.

**Non-negotiable rules. Deviation = critical bug rejection. Focus: Development first (real impls, no mocks). Testing/benching deferred.**

### 1. Project Identity
- Production-grade modular Halo2 zkEVM prover for real Ethereum tx traces.
- Ethereum stack = Alloy only (ethers banned).
- EVM execution = REVM only (`revm` + `revm-primitives`).
- Proof system = Halo2 v0.3.1 (halo2_proofs, halo2_gadgets, halo2curves 0.9.0; Pasta curves only).
- Parallelism = Rayon only.
- Async = Tokio only (`features = ["full"]`).
- Crate type = `rlib` + `cdylib` (WASM-compatible).
- Binaries: `zkevm-prover` (src/main.rs), `verifier-cli` (bin/cli.rs), `benchmark` (bin/benchmark.rs).

### 2. Control Flow (Exact Sequence — No Variations)
1. **CLI Entry** (clap + tokio::main in src/main.rs/bin/cli.rs): Parse args (tx hash, RPC URL, output file).
2. **Trace Fetch/Exec** (utils/evm_parser.rs only): Async Alloy HttpProvider fetches tx/block → REVM EVM::transact_commit() → Extract real EVMData (opcodes, stack, memory, storage diffs).
3. **Witness Prep** (utils/evm_parser.rs): parse_evm_data(&EVMData) → Flatten to real CircuitWitness (vectors for cells; range-check values).
4. **Circuit Build** (circuits/main_circuit.rs): EvmCircuit configures chips (evm_chip dispatches real opcodes; add_chip/arithmetic for computations; storage for diffs) → Assign real witnesses to advice columns.
5. **Proof Gen** (prover/parallel_prover.rs): Chunk witness by rows (e.g., 2^14/step) → Rayon par_iter on real create_proof (Plonk + Blake2b transcript; no MockProver) → Aggregate sub-proofs recursively.
6. **Verify** (prover/verifier.rs): Load VerifyingKey → verify_proof with transcript → Output base64 proof + metadata (tx hash, gas used).
7. **Output** (CLI): Serialize proof (base64 + serde_json) to file; log success/fail.

**No mocks: All traces real (from REVM); all proofs full Plonk (no MockProver); validate inputs (tx existence, trace integrity).**

### 3. Development Patterns (Relaxed for Speed)
- `.unwrap()`, `.expect()` → Allowed in dev code (remove for prod).
- `panic!()` → Allowed only in stubs (with TODO to replace).
- `unimplemented!()`, `todo!()` → Allowed with deadline TODO (e.g., "// TODO: Real impl by [date]").
- `println!`, `debug!` → Allowed in dev; migrate to tracing later.
- Mock data/stubs → Banned; use real REVM sim from day 1.

### 4. Error Handling
- Lib: Prefer `Result<_, crate::Error>` (`thiserror::Error` derive); unwrap/expect OK for dev.
- Bin: `anyhow::Result<_>` + `?` where possible; unwrap OK for quick iteration.

### 5. Dependency Rules
- Alloy crates only (no ethers/web3).
- Halo2 = v0.3.1 only (no 0.2/pse).
- Tokio = full features only.

**API References (Use These Exact Links — No Assumptions):**
- alloy-* (all): https://alloy.rs/llms-full.txt
- base64: https://context7.com/websites/rs_base64_base64/llms.txt
- clap: https://context7.com/clap-rs/clap/llms.txt
- halo2_gadgets: https://docs.rs/halo2_gadgets/0.3.1/halo2_gadgets/
- halo2_proofs: https://docs.rs/halo2_proofs/0.3.1/halo2_proofs/
- halo2curves: https://docs.rs/halo2curves/0.9.0/halo2curves/
- rayon: https://context7.com/rayon-rs/rayon/llms.txt
- revm: https://context7.com/bluealloy/revm/llms.txt
- revm-primitives: https://docs.rs/revm-primitives/21.0.1/revm_primitives/
- serde: https://context7.com/websites/rs_serde/llms.txt
- serde_json: https://context7.com/serde-rs/json/llms.txt
- thiserror: https://context7.com/websites/rs_thiserror/llms.txt
- tokio: https://context7.com/websites/rs_tokio_tokio/llms.txt

### 6. Architecture
- `src/chips/`: Pure Halo2 chips (e.g., add_chip for real arithmetic; evm_chip dispatches opcodes).
- `src/circuits/`: Composes chips (main_circuit: real witnesses → layout; arithmetic/storage: gadgets).
- `src/prover/`: Real proving (parallel_prover: chunk + par_iter; verifier: transcript verify).
- `src/utils/evm_parser.rs`: Sole REVM/Alloy touchpoint (fetch_and_execute_tx async; trace_to_witness).
- `src/lib.rs`: Re-exports only.
- `src/main.rs`: Thin async CLI dispatch.

### 7. Chip/Circuit Rules
- Chips: `impl Chip<F> + Instructions<F>` (F = pasta::Fq).
- Circuits: Generic over FieldExt; public inputs hashed (Poseidon/Keccak).
- Constraints: Real EVM (gas/opcode, stack/memory range-checks, storage merkle).

### 8. Performance/Security
- Chunk traces (1M+ steps) via recursion.
- No raw witnesses exposed.
- Range-check all values (halo2_gadgets::utilities::range).

### 9. Documentation
- Public items: Doc + example.
- New chips: Diagram in `docs/circuit_diagrams/`.

**Bound by these rules. Generate only compliant, real-use-case dev code (unwrap OK for iteration).**