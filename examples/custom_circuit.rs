//! Custom Circuit Example
//!
//! Demonstrates low-level circuit construction and constraint verification.
//! This example shows how to:
//! 1. Build custom execution steps
//! 2. Create circuits from scratch
//! 3. Use individual chips (AddChip, EvmChip)
//! 4. Verify constraints with MockProver

use halo2_proofs::{
    arithmetic::Field,
    circuit::{Layouter, SimpleFloorPlanner, Value},
    dev::MockProver,
    pasta::Fp,
    plonk::{Circuit, ConstraintSystem, Error},
};
use zephyr_proof::{
    chips::{AddChip, AddChipConfig, EvmChip, EvmChipConfig},
    circuits::main_circuit::{EvmCircuit, ExecutionStep},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Custom Circuit Example");
    println!("=========================\n");

    // Example 1: Simple AddChip usage
    println!("📝 Example 1: AddChip Direct Usage");
    println!("-----------------------------------");
    example_1_add_chip()?;

    // Example 2: EvmChip with multiple operations
    println!("\n📝 Example 2: EvmChip Multiple Operations");
    println!("------------------------------------------");
    example_2_evm_chip()?;

    // Example 3: Custom EVM circuit
    println!("\n📝 Example 3: Custom EVM Circuit");
    println!("---------------------------------");
    example_3_custom_circuit()?;

    // Example 4: Complex execution flow
    println!("\n📝 Example 4: Complex Execution Flow");
    println!("-------------------------------------");
    example_4_complex_flow()?;

    println!("\n🎉 All custom circuit examples completed!");

    Ok(())
}

/// Example 1: Direct AddChip usage
fn example_1_add_chip() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Default, Clone, Debug)]
    struct TestAddCircuit {
        a: Fp,
        b: Fp,
    }

    impl Circuit<Fp> for TestAddCircuit {
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

            // Perform addition
            chip.add(layouter.namespace(|| "add"), self.a, self.b)?;

            // Perform multiplication
            chip.mul(layouter.namespace(|| "mul"), self.a, self.b)?;

            // Perform subtraction
            chip.sub(layouter.namespace(|| "sub"), self.a, self.b)?;

            Ok(())
        }
    }

    let a = Fp::from(42);
    let b = Fp::from(17);

    println!("  Testing AddChip:");
    println!("    a = {}", 42);
    println!("    b = {}", 17);
    println!("    a + b = {}", 42 + 17);
    println!("    a * b = {}", 42 * 17);
    println!("    a - b = {}", 42 - 17);

    let circuit = TestAddCircuit { a, b };
    let k = 4;

    let prover = MockProver::run(k, &circuit, vec![])?;
    prover
        .verify()
        .map_err(|e| format!("Verification failed: {:?}", e))?;

    println!("\n  ✅ AddChip constraints satisfied!");

    Ok(())
}

/// Example 2: EvmChip with multiple operations
fn example_2_evm_chip() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Default, Clone, Debug)]
    struct TestEvmCircuit {
        operations: Vec<(u8, Fp, Fp, u64, u64)>, // (opcode, stack_0, stack_1, pc, gas)
    }

    impl Circuit<Fp> for TestEvmCircuit {
        type Config = EvmChipConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
            EvmChip::configure(meta)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fp>,
        ) -> Result<(), Error> {
            let chip = EvmChip::construct(config);

            for (i, (opcode, stack_0, stack_1, pc, gas)) in self.operations.iter().enumerate() {
                chip.execute_opcode(
                    layouter.namespace(|| format!("op_{}", i)),
                    *opcode,
                    *stack_0,
                    *stack_1,
                    *pc,
                    *gas,
                )?;
            }

            Ok(())
        }
    }

    let operations = vec![
        (0x01, Fp::from(10), Fp::from(5), 0, 1000), // ADD
        (0x02, Fp::from(7), Fp::from(6), 2, 997),   // MUL
        (0x03, Fp::from(20), Fp::from(8), 4, 994),  // SUB
    ];

    println!("  Testing EvmChip operations:");
    println!("    [0] ADD: 10 + 5 = 15");
    println!("    [1] MUL: 7 * 6 = 42");
    println!("    [2] SUB: 20 - 8 = 12");

    let circuit = TestEvmCircuit { operations };
    let k = 6;

    let prover = MockProver::run(k, &circuit, vec![])?;
    prover
        .verify()
        .map_err(|e| format!("Verification failed: {:?}", e))?;

    println!("\n  ✅ EvmChip constraints satisfied!");

    Ok(())
}

/// Example 3: Custom EVM circuit with manual step creation
fn example_3_custom_circuit() -> Result<(), Box<dyn std::error::Error>> {
    // Manually create execution steps
    let steps = vec![
        ExecutionStep {
            opcode: 0x60, // PUSH1
            stack: [Fp::from(100), Fp::ZERO, Fp::ZERO],
            pc: 0,
            gas: 10000,
        },
        ExecutionStep {
            opcode: 0x60, // PUSH1
            stack: [Fp::from(50), Fp::from(100), Fp::ZERO],
            pc: 2,
            gas: 9997,
        },
        ExecutionStep {
            opcode: 0x01, // ADD
            stack: [Fp::from(150), Fp::ZERO, Fp::ZERO],
            pc: 4,
            gas: 9994,
        },
        ExecutionStep {
            opcode: 0x60, // PUSH1
            stack: [Fp::from(2), Fp::from(150), Fp::ZERO],
            pc: 6,
            gas: 9991,
        },
        ExecutionStep {
            opcode: 0x02, // MUL
            stack: [Fp::from(300), Fp::ZERO, Fp::ZERO],
            pc: 8,
            gas: 9988,
        },
    ];

    println!("  Custom execution flow:");
    println!("    PUSH1 100  -> stack: [100]");
    println!("    PUSH1 50   -> stack: [50, 100]");
    println!("    ADD        -> stack: [150]");
    println!("    PUSH1 2    -> stack: [2, 150]");
    println!("    MUL        -> stack: [300]");
    println!("    Final result: 300");

    // Create trace commitment (mock hash)
    let trace_commitment = Fp::from(12345);

    // Build circuit
    let circuit = EvmCircuit::new(steps, trace_commitment);

    let k = 10;
    let public_inputs = vec![trace_commitment];

    println!("\n  Circuit configuration:");
    println!("    Execution steps: {}", circuit.steps.len());
    println!("    Circuit size: 2^{} = {} rows", k, 1 << k);
    println!("    Public inputs: {}", public_inputs.len());

    let prover = MockProver::run(k, &circuit, vec![public_inputs])?;
    prover
        .verify()
        .map_err(|e| format!("Verification failed: {:?}", e))?;

    println!("\n  ✅ Custom circuit constraints satisfied!");

    Ok(())
}

/// Example 4: Complex execution flow with gas tracking
fn example_4_complex_flow() -> Result<(), Box<dyn std::error::Error>> {
    use zephyr_proof::chips::evm_chip::OpCode;

    // Simulate a complex contract execution
    let mut steps = Vec::new();
    let mut pc = 0u64;
    let mut gas = 100000u64;
    let mut stack_values = vec![0u64; 3];

    // PUSH1 10
    stack_values[0] = 10;
    steps.push(ExecutionStep {
        opcode: 0x60,
        stack: [
            Fp::from(stack_values[0]),
            Fp::from(stack_values[1]),
            Fp::from(stack_values[2]),
        ],
        pc,
        gas,
    });
    gas -= OpCode::Push1.gas_cost();
    pc += 2;

    // PUSH1 20
    stack_values[1] = stack_values[0];
    stack_values[0] = 20;
    steps.push(ExecutionStep {
        opcode: 0x60,
        stack: [
            Fp::from(stack_values[0]),
            Fp::from(stack_values[1]),
            Fp::from(stack_values[2]),
        ],
        pc,
        gas,
    });
    gas -= OpCode::Push1.gas_cost();
    pc += 2;

    // ADD (10 + 20 = 30)
    stack_values[0] = stack_values[0] + stack_values[1];
    stack_values[1] = 0;
    steps.push(ExecutionStep {
        opcode: 0x01,
        stack: [
            Fp::from(stack_values[0]),
            Fp::from(stack_values[1]),
            Fp::from(stack_values[2]),
        ],
        pc,
        gas,
    });
    gas -= OpCode::Add.gas_cost();
    pc += 1;

    // PUSH1 5
    stack_values[1] = stack_values[0];
    stack_values[0] = 5;
    steps.push(ExecutionStep {
        opcode: 0x60,
        stack: [
            Fp::from(stack_values[0]),
            Fp::from(stack_values[1]),
            Fp::from(stack_values[2]),
        ],
        pc,
        gas,
    });
    gas -= OpCode::Push1.gas_cost();
    pc += 2;

    // MUL (30 * 5 = 150)
    stack_values[0] = stack_values[0] * stack_values[1];
    stack_values[1] = 0;
    steps.push(ExecutionStep {
        opcode: 0x02,
        stack: [
            Fp::from(stack_values[0]),
            Fp::from(stack_values[1]),
            Fp::from(stack_values[2]),
        ],
        pc,
        gas,
    });
    gas -= OpCode::Mul.gas_cost();
    pc += 1;

    println!("  Complex execution:");
    println!("    Initial gas: 100000");
    println!("    Operations:");
    println!("      1. PUSH1 10   (gas: -3)  -> stack: [10]");
    println!("      2. PUSH1 20   (gas: -3)  -> stack: [20, 10]");
    println!("      3. ADD        (gas: -3)  -> stack: [30]");
    println!("      4. PUSH1 5    (gas: -3)  -> stack: [5, 30]");
    println!("      5. MUL        (gas: -5)  -> stack: [150]");
    println!("    Final gas: {}", gas);
    println!("    Gas consumed: {}", 100000 - gas);
    println!("    Result: 150");

    let trace_commitment = Fp::from(54321);
    let circuit = EvmCircuit::new(steps, trace_commitment);

    let k = 10;
    let public_inputs = vec![trace_commitment];

    let prover = MockProver::run(k, &circuit, vec![public_inputs])?;
    prover
        .verify()
        .map_err(|e| format!("Verification failed: {:?}", e))?;

    println!("\n  ✅ Complex flow constraints satisfied!");
    println!("  ✅ Gas tracking validated!");

    Ok(())
}
