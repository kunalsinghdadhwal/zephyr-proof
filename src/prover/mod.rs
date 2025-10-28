//! Proof generation and verification modules
//!
//! This module contains the prover and verifier implementations
//! for generating and verifying zkEVM proofs.

pub mod parallel_prover;
pub mod verifier;

pub use parallel_prover::{generate_proof_parallel, generate_proof_sequential};
pub use verifier::verify;
