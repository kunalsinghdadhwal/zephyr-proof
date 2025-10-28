//! Halo2 circuits for EVM proof generation
//!
//! This module contains composable circuits that combine chips
//! to prove complete EVM execution traces.

pub mod arithmetic;
pub mod main_circuit;
pub mod storage;

pub use arithmetic::ArithmeticCircuit;
pub use main_circuit::EvmCircuit;
pub use storage::StorageCircuit;
