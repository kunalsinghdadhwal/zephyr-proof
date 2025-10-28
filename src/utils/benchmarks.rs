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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_sync() {
        let result = benchmark("test_op", 100, || {
            // Simulate work
            let _sum: u64 = (0..100).sum();
        });

        assert_eq!(result.name, "test_op");
        assert_eq!(result.operations, 100);
        assert!(result.duration.as_nanos() > 0);
    }

    #[tokio::test]
    async fn test_benchmark_async() {
        let result = benchmark_async("async_test", 50, || async {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        })
        .await;

        assert_eq!(result.name, "async_test");
        assert_eq!(result.operations, 50);
        assert!(result.duration.as_millis() >= 10);
    }

    #[test]
    fn test_benchmark_result_display() {
        let result = BenchmarkResult::new("test".to_string(), Duration::from_secs(2), 1000);

        assert_eq!(result.ops_per_sec, 500.0);
    }
}
