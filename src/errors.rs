//! Error types for the zkEVM prover

use thiserror::Error;

/// Main error type for prover operations
#[derive(Error, Debug)]
pub enum ProverError {
    /// Circuit synthesis or constraint error
    #[error("Circuit error: {0}")]
    CircuitError(String),

    /// Trace parsing or validation error
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Proof generation failed
    #[error("Proof generation failed: {0}")]
    ProofGenerationError(String),

    /// Proof verification failed
    #[error("Verification failed: {0}")]
    VerificationError(String),

    /// Network/RPC error when fetching traces
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Real trace fetching/parsing error
    #[error("Real trace error: {0}")]
    RealTraceError(String),

    /// Transaction not found or invalid
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    /// RPC connection failed
    #[error("RPC connection failed: {0}")]
    RpcConnectionError(String),

    /// Invalid input or configuration
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// File I/O error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Base64 encoding/decoding error
    #[error("Base64 error: {0}")]
    Base64Error(String),

    /// Halo2 specific errors
    #[error("Halo2 error: {0}")]
    Halo2Error(String),

    /// EVM execution error
    #[error("EVM error: {0}")]
    EvmError(String),

    /// Resource exhaustion (e.g., out of memory, too many opcodes)
    #[error("Resource limit exceeded: {0}")]
    ResourceError(String),
}

// Implement conversions from common error types

impl From<base64::DecodeError> for ProverError {
    fn from(err: base64::DecodeError) -> Self {
        ProverError::Base64Error(err.to_string())
    }
}

impl From<halo2_proofs::plonk::Error> for ProverError {
    fn from(err: halo2_proofs::plonk::Error) -> Self {
        ProverError::Halo2Error(format!("{:?}", err))
    }
}

// Helper type for results
pub type Result<T> = std::result::Result<T, ProverError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ProverError::CircuitError("test error".to_string());
        assert_eq!(err.to_string(), "Circuit error: test error");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let prover_err: ProverError = io_err.into();
        assert!(matches!(prover_err, ProverError::IoError(_)));
    }
}
