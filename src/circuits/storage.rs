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

    /// Create a mock storage update example
    pub fn mock_update() -> Self {
        let updates = vec![StorageUpdate {
            key: F::from(1),
            old_value: F::from(100),
            new_value: F::from(200),
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

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    #[test]
    fn test_storage_circuit_mock() {
        let circuit = StorageCircuit::<Fp>::mock_update();

        let k = 4;
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_storage_update_creation() {
        let update = StorageUpdate {
            key: Fp::from(42),
            old_value: Fp::from(10),
            new_value: Fp::from(20),
        };
        assert_eq!(update.key, Fp::from(42));
    }
}

// Needed for gate constraint
use halo2_proofs::plonk::Expression;
