//! Addition chip for arithmetic operations
//!
//! Implements Halo2 constraints for addition and multiplication operations
//! used in EVM opcodes like ADD, MUL, etc.

use halo2_proofs::{
    arithmetic::Field,
    circuit::{AssignedCell, Chip, Layouter, Region, Value},
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, Selector},
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

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::circuit::SimpleFloorPlanner;
    use halo2_proofs::plonk::Circuit;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    #[derive(Default)]
    struct TestCircuit {
        a: Fp,
        b: Fp,
    }

    impl Circuit<Fp> for TestCircuit {
        type Config = AddChipConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
            let a = meta.advice_column();
            let b = meta.advice_column();
            let c = meta.advice_column();
            AddChip::configure(meta, a, b, c)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fp>,
        ) -> Result<(), Error> {
            let chip = AddChip::construct(config);
            chip.add(layouter.namespace(|| "add"), self.a, self.b)?;
            Ok(())
        }
    }

    #[test]
    fn test_add_chip() {
        let a = Fp::from(5);
        let b = Fp::from(3);
        let circuit = TestCircuit { a, b };

        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }
}
