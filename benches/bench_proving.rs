//! Criterion benchmarks for proof generation
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use halo2_proofs::{dev::MockProver, pasta::Fp};
use zephyr_proof::{
    circuits::{ArithmeticCircuit, EvmCircuit},
    utils::evm_parser::EvmTrace,
};

fn bench_arithmetic_circuit(c: &mut Criterion) {
    let mut group = c.benchmark_group("arithmetic");

    for size in [10, 20, 30].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let a = Fp::from(size);
            let b = Fp::from(size + 1);
            let circuit = ArithmeticCircuit::add(a, b);

            b.iter(|| {
                let prover = MockProver::run(4, &circuit, vec![]).unwrap();
                black_box(prover);
            });
        });
    }

    group.finish();
}

fn bench_evm_circuit(c: &mut Criterion) {
    let mut group = c.benchmark_group("evm_circuit");

    group.bench_function("mock_add", |b| {
        let circuit = EvmCircuit::<Fp>::mock_add();
        let public_inputs = vec![circuit.trace_commitment];

        b.iter(|| {
            let prover = MockProver::run(10, &circuit, vec![public_inputs.clone()]).unwrap();
            black_box(prover);
        });
    });

    group.finish();
}

fn bench_trace_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("trace_parsing");

    group.bench_function("mock_add", |b| {
        b.iter(|| {
            let trace = EvmTrace::mock_add();
            black_box(trace);
        });
    });

    group.bench_function("mock_mul", |b| {
        b.iter(|| {
            let trace = EvmTrace::mock_mul();
            black_box(trace);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_arithmetic_circuit,
    bench_evm_circuit,
    bench_trace_parsing
);
criterion_main!(benches);
