//! Benchmarking utilities
//!
//! Utilities for benchmarking circuit synthesis and proof generation.

use std::time::{Duration, Instant};

/// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Operation name
    pub name: String,
    /// Duration of the operation
    pub duration: Duration,
    /// Number of operations performed
    pub operations: usize,
    /// Operations per second
    pub ops_per_sec: f64,
}

impl BenchmarkResult {
    /// Create a new benchmark result
    pub fn new(name: String, duration: Duration, operations: usize) -> Self {
        let ops_per_sec = operations as f64 / duration.as_secs_f64();
        Self {
            name,
            duration,
            operations,
            ops_per_sec,
        }
    }

    /// Display the benchmark result
    pub fn display(&self) {
        println!("Benchmark: {}", self.name);
        println!("  Duration: {:?}", self.duration);
        println!("  Operations: {}", self.operations);
        println!("  Ops/sec: {:.2}", self.ops_per_sec);
    }
}

/// Benchmark a function
///
/// # Arguments
///
/// * `name` - Name of the benchmark
/// * `operations` - Number of operations
/// * `f` - Function to benchmark
///
/// # Returns
///
/// `BenchmarkResult` with timing information
pub fn benchmark<F>(name: &str, operations: usize, mut f: F) -> BenchmarkResult
where
    F: FnMut(),
{
    let start = Instant::now();
    f();
    let duration = start.elapsed();

    BenchmarkResult::new(name.to_string(), duration, operations)
}

/// Benchmark an async function
///
/// # Arguments
///
/// * `name` - Name of the benchmark
/// * `operations` - Number of operations
/// * `f` - Async function to benchmark
///
/// # Returns
///
/// `BenchmarkResult` with timing information
pub async fn benchmark_async<F, Fut>(name: &str, operations: usize, f: F) -> BenchmarkResult
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let start = Instant::now();
    f().await;
    let duration = start.elapsed();

    BenchmarkResult::new(name.to_string(), duration, operations)
}

/// Benchmark circuit synthesis
///
/// This would be used with Criterion in benches/
pub fn bench_add_opcode() {
    use crate::circuits::ArithmeticCircuit;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    let a = Fp::from(10);
    let b = Fp::from(20);
    let circuit = ArithmeticCircuit::add(a, b);

    let _prover = MockProver::run(4, &circuit, vec![]).unwrap();
}

/// Benchmark proof generation
pub fn bench_proof_generation() {
    // TODO: Implement with real prover
    // This would benchmark create_proof for various circuit sizes
}
