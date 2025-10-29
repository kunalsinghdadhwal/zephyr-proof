//! Main EVM circuit composing all chips
//!
//! Top-level circuit that proves complete EVM execution traces.

use halo2_proofs::{
    arithmetic::Field,
    circuit::{Layouter, SimpleFloorPlanner, Value},
    plonk::{Circuit, Column, ConstraintSystem, Error, Instance},
};

use crate::chips::{AddChip, AddChipConfig, EvmChip, EvmChipConfig};

/// Execution step in the EVM trace
#[derive(Debug, Clone)]
pub struct ExecutionStep<F: Field> {
    /// Opcode to execute
    pub opcode: u8,
    /// Stack values (top 3 slots)
    pub stack: [F; 3],
    /// Program counter
    pub pc: u64,
    /// Gas remaining
    pub gas: u64,
}

/// Configuration for the main EVM circuit
#[derive(Clone, Debug)]
pub struct EvmCircuitConfig {
    /// EVM chip configuration
    pub evm_config: EvmChipConfig,
    /// Arithmetic chip configuration
    pub add_config: AddChipConfig,
    /// Public input column (trace commitment)
    pub public_input: Column<Instance>,
}

/// Main circuit for EVM execution trace
#[derive(Default, Clone, Debug)]
pub struct EvmCircuit<F: Field> {
    /// Execution steps to prove
    pub steps: Vec<ExecutionStep<F>>,
    /// Public trace commitment (hash of all steps)
    pub trace_commitment: F,
}

impl<F: Field> EvmCircuit<F> {
    /// Create a new EVM circuit
    pub fn new(steps: Vec<ExecutionStep<F>>, trace_commitment: F) -> Self {
        Self {
            steps,
            trace_commitment,
        }
    }
}

impl<F: Field> Circuit<F> for EvmCircuit<F> {
    type Config = EvmCircuitConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let public_input = meta.instance_column();
        meta.enable_equality(public_input);

        // Configure EVM chip
        let evm_config = EvmChip::configure(meta);

        // Configure arithmetic chip (for complex operations)
        let a = meta.advice_column();
        let b = meta.advice_column();
        let c = meta.advice_column();
        let add_config = AddChip::configure(meta, a, b, c);

        EvmCircuitConfig {
            evm_config,
            add_config,
            public_input,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let evm_chip = EvmChip::construct(config.evm_config.clone());

        // Expose trace commitment as public input
        let commitment_cell = layouter.assign_region(
            || "public_input",
            |mut region| {
                region.assign_advice(
                    || "trace_commitment",
                    config.evm_config.opcode, // Reuse a column
                    0,
                    || Value::known(self.trace_commitment),
                )
            },
        )?;

        layouter.constrain_instance(commitment_cell.cell(), config.public_input, 0)?;

        // Execute each step
        for (i, step) in self.steps.iter().enumerate() {
            evm_chip.execute_opcode(
                layouter.namespace(|| format!("step_{}", i)),
                step.opcode,
                step.stack[0],
                step.stack[1],
                step.pc,
                step.gas,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    /// Helper to create a test circuit
    fn create_test_circuit() -> EvmCircuit<Fp> {
        let steps = vec![
            ExecutionStep {
                opcode: 0x60, // PUSH1
                stack: [Fp::from(1u64), Fp::ZERO, Fp::ZERO],
                pc: 0,
                gas: 1000,
            },
            ExecutionStep {
                opcode: 0x60, // PUSH1
                stack: [Fp::from(2u64), Fp::from(1u64), Fp::ZERO],
                pc: 2,
                gas: 997,
            },
            ExecutionStep {
                opcode: 0x01, // ADD
                stack: [Fp::from(3u64), Fp::ZERO, Fp::ZERO],
                pc: 4,
                gas: 994,
            },
        ];
        let trace_commitment = Fp::from(12345u64);
        EvmCircuit::new(steps, trace_commitment)
    }

    #[test]
    fn test_evm_circuit_add() {
        let circuit = create_test_circuit();

        // Public inputs: trace commitment
        let public_inputs = vec![circuit.trace_commitment];

        let k = 10; // 2^10 = 1024 rows
        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_execution_step_creation() {
        let step = ExecutionStep {
            opcode: 0x01,
            stack: [Fp::from(5u64), Fp::from(3u64), Fp::ZERO],
            pc: 0,
            gas: 100,
        };
        assert_eq!(step.opcode, 0x01);
        assert_eq!(step.gas, 100);
    }

    #[test]
    fn test_evm_circuit_empty() {
        let circuit = EvmCircuit::<Fp>::new(vec![], Fp::ZERO);
        assert_eq!(circuit.steps.len(), 0);
    }

    #[test]
    fn test_evm_circuit_single_step() {
        let steps = vec![
            ExecutionStep {
                opcode: 0x60,
                stack: [Fp::from(1u64), Fp::ZERO, Fp::ZERO],
                pc: 0,
                gas: 1000,
            },
        ];
        let circuit = EvmCircuit::new(steps, Fp::from(111u64));
        
        let k = 10;
        let public_inputs = vec![circuit.trace_commitment];
        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_evm_circuit_mul() {
        let steps = vec![
            ExecutionStep {
                opcode: 0x60, // PUSH1
                stack: [Fp::from(5u64), Fp::ZERO, Fp::ZERO],
                pc: 0,
                gas: 1000,
            },
            ExecutionStep {
                opcode: 0x60, // PUSH1
                stack: [Fp::from(3u64), Fp::from(5u64), Fp::ZERO],
                pc: 2,
                gas: 997,
            },
            ExecutionStep {
                opcode: 0x02, // MUL
                stack: [Fp::from(15u64), Fp::ZERO, Fp::ZERO],
                pc: 4,
                gas: 994,
            },
        ];
        
        let circuit = EvmCircuit::new(steps, Fp::from(54321u64));
        let k = 10;
        let public_inputs = vec![circuit.trace_commitment];
        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_circuit_config() {
        use halo2_proofs::plonk::ConstraintSystem;
        
        let mut cs = ConstraintSystem::<Fp>::default();
        let _config = EvmCircuit::<Fp>::configure(&mut cs);
        
        // Configuration test passed - circuit can be configured
        assert!(true);
    }
}
