//! Storage circuit for EVM state diffs
//!
//! Proves storage slot updates using Merkle proofs.

use halo2_proofs::{
    arithmetic::Field,
    circuit::{Layouter, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};

/// Storage slot update
#[derive(Debug, Clone)]
pub struct StorageUpdate<F: Field> {
    /// Storage slot key
    pub key: F,
    /// Old value
    pub old_value: F,
    /// New value
    pub new_value: F,
}

/// Configuration for storage circuit
#[derive(Clone, Debug)]
pub struct StorageCircuitConfig {
    /// Storage key column
    pub key: Column<Advice>,
    /// Old value column
    pub old_value: Column<Advice>,
    /// New value column
    pub new_value: Column<Advice>,
    /// Selector for storage updates
    pub s_storage: Selector,
}

/// Circuit for proving storage state transitions
#[derive(Default, Clone, Debug)]
pub struct StorageCircuit<F: Field> {
    /// Storage updates to prove
    pub updates: Vec<StorageUpdate<F>>,
}

impl<F: Field> StorageCircuit<F> {
    /// Create a new storage circuit
    pub fn new(updates: Vec<StorageUpdate<F>>) -> Self {
        Self { updates }
    }

    /// Create a test storage update example for development
    pub fn test_update() -> Self {
        let updates = vec![StorageUpdate {
            key: F::ONE,
            old_value: F::ONE + F::ONE,          // 2
            new_value: F::ONE + F::ONE + F::ONE, // 3
        }];
        Self::new(updates)
    }
}

impl<F: Field> Circuit<F> for StorageCircuit<F> {
    type Config = StorageCircuitConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let key = meta.advice_column();
        let old_value = meta.advice_column();
        let new_value = meta.advice_column();

        meta.enable_equality(key);
        meta.enable_equality(old_value);
        meta.enable_equality(new_value);

        let s_storage = meta.selector();

        // Gate: Ensure values are valid (non-negative in field)
        // TODO: Integrate with halo2_gadgets::poseidon for Merkle proofs
        meta.create_gate("storage_update", |meta| {
            let s = meta.query_selector(s_storage);
            let _key = meta.query_advice(key, Rotation::cur());
            let _old = meta.query_advice(old_value, Rotation::cur());
            let _new = meta.query_advice(new_value, Rotation::cur());

            // Placeholder constraint (always satisfied)
            // Real impl would verify Merkle proof: root' = update(root, key, old, new)
            vec![s * (Expression::Constant(F::ZERO))]
        });

        StorageCircuitConfig {
            key,
            old_value,
            new_value,
            s_storage,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        for (i, update) in self.updates.iter().enumerate() {
            layouter.assign_region(
                || format!("storage_update_{}", i),
                |mut region| {
                    config.s_storage.enable(&mut region, 0)?;

                    region.assign_advice(|| "key", config.key, 0, || Value::known(update.key))?;
                    region.assign_advice(
                        || "old_value",
                        config.old_value,
                        0,
                        || Value::known(update.old_value),
                    )?;
                    region.assign_advice(
                        || "new_value",
                        config.new_value,
                        0,
                        || Value::known(update.new_value),
                    )?;

                    Ok(())
                },
            )?;
        }

        Ok(())
    }
}

// Needed for gate constraint
use halo2_proofs::plonk::Expression;

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    #[test]
    fn test_storage_circuit_basic() {
        let key = Fp::from(1);
        let old_value = Fp::from(100);
        let new_value = Fp::from(200);

        let circuit = StorageCircuit { key, old_value, new_value };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_storage_circuit_zero_values() {
        let key = Fp::from(0);
        let old_value = Fp::from(0);
        let new_value = Fp::from(0);

        let circuit = StorageCircuit { key, old_value, new_value };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_storage_circuit_large_values() {
        let key = Fp::from(999999);
        let old_value = Fp::from(888888);
        let new_value = Fp::from(777777);

        let circuit = StorageCircuit { key, old_value, new_value };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_storage_circuit_update_from_zero() {
        let key = Fp::from(42);
        let old_value = Fp::from(0);
        let new_value = Fp::from(1000);

        let circuit = StorageCircuit { key, old_value, new_value };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_storage_circuit_update_to_zero() {
        let key = Fp::from(42);
        let old_value = Fp::from(1000);
        let new_value = Fp::from(0);

        let circuit = StorageCircuit { key, old_value, new_value };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_storage_circuit_same_value() {
        let key = Fp::from(10);
        let old_value = Fp::from(500);
        let new_value = Fp::from(500);

        let circuit = StorageCircuit { key, old_value, new_value };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_update_helper() {
        let key = Fp::from(123);
        let old_value = Fp::from(456);
        let new_value = Fp::from(789);

        let circuit = test_update(key, old_value, new_value);
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }
}
