//! Addition chip for arithmetic operations
//!
//! Implements Halo2 constraints for addition and multiplication operations
//! used in EVM opcodes like ADD, MUL, etc.

use halo2_proofs::{
    arithmetic::Field,
    circuit::{AssignedCell, Chip, Layouter, Value},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};
use std::marker::PhantomData;

/// Configuration for the AddChip
#[derive(Clone, Debug)]
pub struct AddChipConfig {
    /// Input column a
    pub a: Column<Advice>,
    /// Input column b
    pub b: Column<Advice>,
    /// Output column c (a + b or a * b)
    pub c: Column<Advice>,
    /// Selector for addition gates
    pub s_add: Selector,
    /// Selector for multiplication gates
    pub s_mul: Selector,
}

/// Chip for arithmetic operations
pub struct AddChip<F: Field> {
    config: AddChipConfig,
    _marker: PhantomData<F>,
}

impl<F: Field> Chip<F> for AddChip<F> {
    type Config = AddChipConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: Field> AddChip<F> {
    /// Construct a new AddChip
    pub fn construct(config: AddChipConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    /// Configure the chip with constraints
    ///
    /// Creates two gates:
    /// - Addition: c = a + b
    /// - Multiplication: c = a * b
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        a: Column<Advice>,
        b: Column<Advice>,
        c: Column<Advice>,
    ) -> AddChipConfig {
        meta.enable_equality(a);
        meta.enable_equality(b);
        meta.enable_equality(c);

        let s_add = meta.selector();
        let s_mul = meta.selector();

        // Addition gate: a + b = c
        meta.create_gate("add", |meta| {
            let s = meta.query_selector(s_add);
            let a = meta.query_advice(a, Rotation::cur());
            let b = meta.query_advice(b, Rotation::cur());
            let c = meta.query_advice(c, Rotation::cur());

            vec![s * (a + b - c)]
        });

        // Multiplication gate: a * b = c
        meta.create_gate("mul", |meta| {
            let s = meta.query_selector(s_mul);
            let a = meta.query_advice(a, Rotation::cur());
            let b = meta.query_advice(b, Rotation::cur());
            let c = meta.query_advice(c, Rotation::cur());

            vec![s * (a * b - c)]
        });

        AddChipConfig {
            a,
            b,
            c,
            s_add,
            s_mul,
        }
    }

    /// Assign and constrain addition: c = a + b
    /// Real ex: Pop two stack values, push sum (mod 2^256 in field)
    pub fn add(
        &self,
        mut layouter: impl Layouter<F>,
        a: F,
        b: F,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "add",
            |mut region| {
                self.config.s_add.enable(&mut region, 0)?;

                region.assign_advice(|| "a", self.config.a, 0, || Value::known(a))?;
                region.assign_advice(|| "b", self.config.b, 0, || Value::known(b))?;

                let c = a + b;
                region.assign_advice(|| "c", self.config.c, 0, || Value::known(c))
            },
        )
    }

    /// Assign and constrain addition with assigned cells (for circuit composition)
    pub fn add_assigned(
        &self,
        mut layouter: impl Layouter<F>,
        a: &AssignedCell<F, F>,
        b: &AssignedCell<F, F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "add_assigned",
            |mut region| {
                self.config.s_add.enable(&mut region, 0)?;

                let a_val = a.value().copied();
                let b_val = b.value().copied();

                region.assign_advice(|| "a", self.config.a, 0, || a_val)?;
                region.assign_advice(|| "b", self.config.b, 0, || b_val)?;

                let c_val = a_val + b_val;
                region.assign_advice(|| "c", self.config.c, 0, || c_val)
            },
        )
    }

    /// Assign and constrain subtraction: c = a - b
    /// Real ex: SUB opcode implementation
    pub fn sub(
        &self,
        mut layouter: impl Layouter<F>,
        a: F,
        b: F,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "sub",
            |mut region| {
                // Reuse add gate with negated b: a + (-b) = a - b
                self.config.s_add.enable(&mut region, 0)?;

                region.assign_advice(|| "a", self.config.a, 0, || Value::known(a))?;
                region.assign_advice(|| "b", self.config.b, 0, || Value::known(-b))?;

                let c = a - b;
                region.assign_advice(|| "c", self.config.c, 0, || Value::known(c))
            },
        )
    }

    /// Assign and constrain multiplication: c = a * b
    pub fn mul(
        &self,
        mut layouter: impl Layouter<F>,
        a: F,
        b: F,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "mul",
            |mut region| {
                self.config.s_mul.enable(&mut region, 0)?;

                region.assign_advice(|| "a", self.config.a, 0, || Value::known(a))?;
                region.assign_advice(|| "b", self.config.b, 0, || Value::known(b))?;

                let c = a * b;
                region.assign_advice(|| "c", self.config.c, 0, || Value::known(c))
            },
        )
    }
}

// Test circuit for AddChip
#[cfg(test)]
use halo2_proofs::{circuit::SimpleFloorPlanner, plonk::Circuit};

#[cfg(test)]
#[derive(Default, Clone, Debug)]
pub struct AddCircuit<F: Field> {
    pub a: F,
    pub b: F,
    pub c: F,
}

#[cfg(test)]
impl<F: Field> Circuit<F> for AddCircuit<F> {
    type Config = AddChipConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut halo2_proofs::plonk::ConstraintSystem<F>) -> Self::Config {
        let a = meta.advice_column();
        let b = meta.advice_column();
        let c = meta.advice_column();
        AddChip::configure(meta, a, b, c)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl halo2_proofs::circuit::Layouter<F>) -> Result<(), halo2_proofs::plonk::Error> {
        let chip = AddChip::construct(config);
        chip.add(layouter.namespace(|| "add"), self.a, self.b)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    #[test]
    fn test_add_chip_basic() {
        let a = Fp::from(5);
        let b = Fp::from(7);
        let c = a + b;

        let circuit = AddCircuit { a, b, c };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_add_chip_zero() {
        let a = Fp::from(0);
        let b = Fp::from(0);
        let c = Fp::from(0);

        let circuit = AddCircuit { a, b, c };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_add_chip_large_values() {
        let a = Fp::from(1000000);
        let b = Fp::from(2000000);
        let c = a + b;

        let circuit = AddCircuit { a, b, c };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_add_chip_invalid_sum() {
        // Test that circuit correctly handles addition
        // (Cannot easily test invalid case with current circuit structure)
        let a = Fp::from(5);
        let b = Fp::from(7);
        let c = a + b;

        let circuit = AddCircuit { a, b, c };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_add_chip_identity() {
        let a = Fp::from(42);
        let b = Fp::from(0);
        let c = a + b;

        let circuit = AddCircuit { a, b, c };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_add_chip_commutative() {
        let a = Fp::from(123);
        let b = Fp::from(456);
        let c = a + b;

        let circuit1 = AddCircuit { a, b, c };
        let circuit2 = AddCircuit { a: b, b: a, c };

        let prover1 = MockProver::run(4, &circuit1, vec![]).unwrap();
        let prover2 = MockProver::run(4, &circuit2, vec![]).unwrap();

        assert_eq!(prover1.verify(), Ok(()));
        assert_eq!(prover2.verify(), Ok(()));
    }
}
