//! Arithmetic circuit for basic math operations
//!
//! Composable circuit for proving arithmetic operations with range checks.

use halo2_proofs::{
    arithmetic::Field,
    circuit::{Layouter, SimpleFloorPlanner},
    plonk::{Circuit, ConstraintSystem, Error},
};

use crate::chips::{AddChip, AddChipConfig};

/// Circuit for arithmetic operations
#[derive(Default, Clone, Debug)]
pub struct ArithmeticCircuit<F: Field> {
    /// First operand
    pub a: F,
    /// Second operand
    pub b: F,
    /// Operation: 0=add, 1=mul
    pub op: u8,
}

impl<F: Field> ArithmeticCircuit<F> {
    /// Create a new arithmetic circuit
    pub fn new(a: F, b: F, op: u8) -> Self {
        Self { a, b, op }
    }

    /// Create an addition circuit
    pub fn add(a: F, b: F) -> Self {
        Self::new(a, b, 0)
    }

    /// Create a multiplication circuit
    pub fn mul(a: F, b: F) -> Self {
        Self::new(a, b, 1)
    }
}

impl<F: Field> Circuit<F> for ArithmeticCircuit<F> {
    type Config = AddChipConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let a = meta.advice_column();
        let b = meta.advice_column();
        let c = meta.advice_column();
        AddChip::configure(meta, a, b, c)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let chip = AddChip::construct(config);

        match self.op {
            0 => {
                chip.add(layouter.namespace(|| "add"), self.a, self.b)?;
            }
            1 => {
                chip.mul(layouter.namespace(|| "mul"), self.a, self.b)?;
            }
            _ => {
                // Default to addition for unsupported ops
                chip.add(layouter.namespace(|| "add_default"), self.a, self.b)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    #[test]
    fn test_arithmetic_circuit_add() {
        let a = Fp::from(10);
        let b = Fp::from(20);
        let result = a + b;

        let circuit = ArithmeticCircuit { a, b, result };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_arithmetic_circuit_zero() {
        let a = Fp::from(0);
        let b = Fp::from(0);
        let result = Fp::from(0);

        let circuit = ArithmeticCircuit { a, b, result };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_arithmetic_circuit_large_values() {
        let a = Fp::from(999999);
        let b = Fp::from(888888);
        let result = a + b;

        let circuit = ArithmeticCircuit { a, b, result };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_arithmetic_circuit_invalid() {
        let a = Fp::from(10);
        let b = Fp::from(20);
        let result = Fp::from(999); // Wrong result

        let circuit = ArithmeticCircuit { a, b, result };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert!(prover.verify().is_err());
    }

    #[test]
    fn test_arithmetic_circuit_with_k8() {
        let a = Fp::from(42);
        let b = Fp::from(58);
        let result = a + b;

        let circuit = ArithmeticCircuit { a, b, result };
        let prover = MockProver::run(8, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }
}
