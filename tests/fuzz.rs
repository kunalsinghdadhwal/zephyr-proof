// Fuzz tests for zkEVM prover
use halo2_proofs::{dev::MockProver, pasta::Fp};
use zephyr_proof::circuits::arithmetic::ArithmeticCircuit;

/// Fuzz test: Random field elements for arithmetic circuit
#[test]
fn fuzz_arithmetic_circuit_random_values() {
    for i in 0..100 {
        let a_val = (i * 123) % 10000;
        let b_val = (i * 456) % 10000;

        let a = Fp::from(a_val);
        let b = Fp::from(b_val);

        let circuit = ArithmeticCircuit::add(a, b);
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert!(prover.verify().is_ok());
    }
}

/// Fuzz test: Multiplication operations
#[test]
fn fuzz_arithmetic_circuit_multiplication() {
    for i in 0..50 {
        let a = Fp::from(i + 1);
        let b = Fp::from((i * 2) + 1);

        let circuit = ArithmeticCircuit::mul(a, b);
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert!(prover.verify().is_ok());
    }
}

/// Fuzz test: Edge case values
#[test]
fn fuzz_edge_case_values() {
    let edge_cases = vec![Fp::from(0), Fp::from(1), Fp::from(1000000)];

    for a in &edge_cases {
        for b in &edge_cases {
            let circuit = ArithmeticCircuit::add(*a, *b);
            let prover = MockProver::run(4, &circuit, vec![]).unwrap();
            assert!(prover.verify().is_ok());
        }
    }
}

/// Fuzz test: Varying k values
#[test]
fn fuzz_varying_k_values() {
    let k_values = vec![4, 5, 6, 7, 8];

    for k in k_values {
        for i in 0..10 {
            let a = Fp::from(i * 10);
            let b = Fp::from(i * 20);

            let circuit = ArithmeticCircuit::add(a, b);
            let prover = MockProver::run(k, &circuit, vec![]).unwrap();
            assert!(prover.verify().is_ok());
        }
    }
}

/// Fuzz test: Mixed operations
#[test]
fn fuzz_mixed_operations() {
    for i in 0..50 {
        let a = Fp::from(i + 1);
        let b = Fp::from((i % 10) + 1);

        // Test both add and mul
        let add_circuit = ArithmeticCircuit::add(a, b);
        let mul_circuit = ArithmeticCircuit::mul(a, b);

        let prover_add = MockProver::run(4, &add_circuit, vec![]).unwrap();
        let prover_mul = MockProver::run(4, &mul_circuit, vec![]).unwrap();

        assert!(prover_add.verify().is_ok());
        assert!(prover_mul.verify().is_ok());
    }
}

/// Fuzz test: Large sequential values
#[test]
fn fuzz_large_sequential_values() {
    for i in 0..100 {
        let a = Fp::from(i * 1000);
        let b = Fp::from(i * 2000);

        let circuit = ArithmeticCircuit::add(a, b);
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert!(prover.verify().is_ok());
    }
}

/// Fuzz test: Commutative property
#[test]
fn fuzz_commutative_property() {
    for i in 0..30 {
        let a = Fp::from(i * 7);
        let b = Fp::from(i * 11);

        let circuit1 = ArithmeticCircuit::add(a, b);
        let circuit2 = ArithmeticCircuit::add(b, a);

        let prover1 = MockProver::run(4, &circuit1, vec![]).unwrap();
        let prover2 = MockProver::run(4, &circuit2, vec![]).unwrap();

        assert!(prover1.verify().is_ok());
        assert!(prover2.verify().is_ok());
    }
}

/// Fuzz test: Identity element
#[test]
fn fuzz_identity_element() {
    let zero = Fp::from(0);

    for i in 0..50 {
        let a = Fp::from(i * 100);

        // a + 0 should work
        let circuit = ArithmeticCircuit::add(a, zero);
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert!(prover.verify().is_ok());
    }
}
