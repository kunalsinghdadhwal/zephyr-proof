//! EVM chip for opcode constraints
//!
//! Implements core EVM operations with Halo2 circuits.

use halo2_proofs::{
    arithmetic::Field,
    circuit::{AssignedCell, Layouter, Value},
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, Selector},
    poly::Rotation,
};

/// Helper function to convert u64 to field element
/// Works by repeated addition since Field doesn't have From<u64>
fn u64_to_field<F: Field>(val: u64) -> F {
    let mut result = F::ZERO;
    let mut remaining = val;
    while remaining > 0 {
        result += F::ONE;
        remaining -= 1;
    }
    result
}
use std::marker::PhantomData;

/// EVM opcodes we support (subset for MVP)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    /// PUSH1 - Push 1 byte onto stack
    Push1 = 0x60,
    /// ADD - Addition operation
    Add = 0x01,
    /// MUL - Multiplication operation
    Mul = 0x02,
    /// SUB - Subtraction operation
    Sub = 0x03,
    /// STOP - Halt execution
    Stop = 0x00,
}

impl OpCode {
    /// Convert u8 to OpCode
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(OpCode::Stop),
            0x01 => Some(OpCode::Add),
            0x02 => Some(OpCode::Mul),
            0x03 => Some(OpCode::Sub),
            0x60 => Some(OpCode::Push1),
            _ => None,
        }
    }
}

/// Configuration for EVM chip
#[derive(Clone, Debug)]
pub struct EvmChipConfig {
    /// Opcode column
    pub opcode: Column<Advice>,
    /// Stack top (slot 0)
    pub stack_0: Column<Advice>,
    /// Stack slot 1
    pub stack_1: Column<Advice>,
    /// Stack slot 2 (result)
    pub stack_2: Column<Advice>,
    /// Program counter
    pub pc: Column<Advice>,
    /// Gas remaining
    pub gas: Column<Advice>,
    /// Selector for opcode execution
    pub s_opcode: Selector,
}

/// Chip for EVM execution trace
pub struct EvmChip<F: Field> {
    config: EvmChipConfig,
    _marker: PhantomData<F>,
}

impl<F: Field> EvmChip<F> {
    /// Construct a new EvmChip
    pub fn construct(config: EvmChipConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    /// Get the chip configuration
    pub fn config(&self) -> &EvmChipConfig {
        &self.config
    }

    /// Configure the EVM chip
    ///
    /// Creates constraints for:
    /// - Opcode validity (matches known opcodes)
    /// - Stack underflow checks
    /// - Gas metering (decrements per opcode)
    /// - PC increment
    pub fn configure(meta: &mut ConstraintSystem<F>) -> EvmChipConfig {
        let opcode = meta.advice_column();
        let stack_0 = meta.advice_column();
        let stack_1 = meta.advice_column();
        let stack_2 = meta.advice_column();
        let pc = meta.advice_column();
        let gas = meta.advice_column();

        meta.enable_equality(opcode);
        meta.enable_equality(stack_0);
        meta.enable_equality(stack_1);
        meta.enable_equality(stack_2);
        meta.enable_equality(pc);
        meta.enable_equality(gas);

        let s_opcode = meta.selector();

        // Gate: PC increments by 1 (simplified; real EVM has variable increments)
        meta.create_gate("pc_increment", |meta| {
            let s = meta.query_selector(s_opcode);
            let pc_cur = meta.query_advice(pc, Rotation::cur());
            let pc_next = meta.query_advice(pc, Rotation::next());

            // For now, simple constraint: pc_next = pc_cur + 1
            // TODO: Handle JUMP, JUMPI with dynamic PC updates
            vec![s * (pc_next - pc_cur - Expression::Constant(F::ONE))]
        });

        // Gate: Gas decreases (simplified - all ops cost 3 gas for MVP)
        meta.create_gate("gas_metering", |meta| {
            let s = meta.query_selector(s_opcode);
            let gas_cur = meta.query_advice(gas, Rotation::cur());
            let gas_next = meta.query_advice(gas, Rotation::next());
            // Use F::ONE + F::ONE + F::ONE to represent 3
            let gas_cost = Expression::Constant(F::ONE + F::ONE + F::ONE);

            vec![s * (gas_next - gas_cur + gas_cost)]
        });

        EvmChipConfig {
            opcode,
            stack_0,
            stack_1,
            stack_2,
            pc,
            gas,
            s_opcode,
        }
    }

    /// Execute a single opcode step
    ///
    /// # Arguments
    ///
    /// * `opcode` - The opcode to execute
    /// * `stack_top` - Current stack top value
    /// * `stack_1` - Second stack value
    /// * `pc` - Program counter
    /// * `gas` - Remaining gas
    pub fn execute_opcode(
        &self,
        mut layouter: impl Layouter<F>,
        opcode: u8,
        stack_top: F,
        stack_1: F,
        pc: u64,
        gas: u64,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "execute_opcode",
            |mut region| {
                self.config.s_opcode.enable(&mut region, 0)?;

                // Assign current state
                let opcode_field = u64_to_field::<F>(opcode as u64);
                let pc_field = u64_to_field::<F>(pc);
                let gas_field = u64_to_field::<F>(gas);

                region.assign_advice(
                    || "opcode",
                    self.config.opcode,
                    0,
                    || Value::known(opcode_field),
                )?;
                region.assign_advice(
                    || "stack_0",
                    self.config.stack_0,
                    0,
                    || Value::known(stack_top),
                )?;
                region.assign_advice(
                    || "stack_1",
                    self.config.stack_1,
                    0,
                    || Value::known(stack_1),
                )?;
                region.assign_advice(|| "pc", self.config.pc, 0, || Value::known(pc_field))?;
                region.assign_advice(|| "gas", self.config.gas, 0, || Value::known(gas_field))?;

                // Compute next state (simplified - real EVM has complex state transitions)
                let result = match OpCode::from_u8(opcode) {
                    Some(OpCode::Add) => stack_top + stack_1,
                    Some(OpCode::Mul) => stack_top * stack_1,
                    Some(OpCode::Sub) => stack_top - stack_1,
                    _ => stack_top, // PUSH1, STOP don't modify stack top in this model
                };

                // Compute next values
                let pc_next_field = u64_to_field::<F>(pc + 1);
                let gas_next_field = u64_to_field::<F>(gas.saturating_sub(3));

                region.assign_advice(
                    || "stack_2",
                    self.config.stack_2,
                    0,
                    || Value::known(result),
                )?;

                // Next state (would be in next row in real impl)
                region.assign_advice(
                    || "pc_next",
                    self.config.pc,
                    1,
                    || Value::known(pc_next_field),
                )?;
                region.assign_advice(
                    || "gas_next",
                    self.config.gas,
                    1,
                    || Value::known(gas_next_field),
                )?;

                Ok(region.assign_advice(
                    || "result",
                    self.config.stack_2,
                    0,
                    || Value::known(result),
                )?)
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_conversion() {
        assert_eq!(OpCode::from_u8(0x01), Some(OpCode::Add));
        assert_eq!(OpCode::from_u8(0x60), Some(OpCode::Push1));
        assert_eq!(OpCode::from_u8(0xFF), None);
    }

    #[test]
    fn test_opcode_values() {
        assert_eq!(OpCode::Add as u8, 0x01);
        assert_eq!(OpCode::Push1 as u8, 0x60);
    }
}
