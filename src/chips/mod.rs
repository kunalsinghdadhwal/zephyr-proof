//! Halo2 chips for EVM operations
//!
//! This module contains low-level Halo2 gadgets that implement
//! cryptographic constraints for EVM opcodes and arithmetic operations.

pub mod add_chip;
pub mod evm_chip;

pub use add_chip::*;
pub use evm_chip::*;
