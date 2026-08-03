// SPDX-License-Identifier: AGPL-3.0-or-later

//! Error types and result alias for tideGlass core operations.

/// Unified error type for tideGlass core library operations.
#[derive(Debug, thiserror::Error)]
pub enum TideGlassError {
    /// A gene signature contained no genes when at least one was required.
    #[error("empty gene signature: {context}")]
    EmptySignature {
        /// Human-readable context describing which signature was empty.
        context: String,
    },

    /// Perturbation or ranked gene data did not meet minimum size requirements.
    #[error("insufficient perturbation data: expected at least {required}, found {actual}")]
    InsufficientData {
        /// Minimum number of items required for the operation.
        required: usize,
        /// Number of items actually available.
        actual: usize,
    },

    /// Weighted Kolmogorov–Smirnov enrichment computation failed.
    #[error("enrichment computation failed: {reason}")]
    Enrichment {
        /// Description of why enrichment failed.
        reason: String,
    },

    /// Permutation-based p-value estimation failed.
    #[error("permutation test failed: {reason}")]
    Permutation {
        /// Description of why the permutation test failed.
        reason: String,
    },

    /// No registered primal advertises the requested JSON-RPC method.
    #[error("capability not found: {method}")]
    CapabilityNotFound {
        /// JSON-RPC method name that could not be resolved.
        method: String,
    },

    /// IPC transport layer failure (connection, timeout, protocol).
    #[error("IPC transport error: {0}")]
    Transport(String),

    /// Data access failure (CAS, federation, local store).
    #[error("data access error: {0}")]
    DataAccess(String),

    /// JSON serialization or deserialization failure.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Standard I/O failure.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result alias wrapping [`TideGlassError`].
pub type Result<T> = std::result::Result<T, TideGlassError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_signature_display_includes_context() {
        let err = TideGlassError::EmptySignature {
            context: "disease up genes".to_owned(),
        };
        assert!(err.to_string().contains("disease up genes"));
    }

    #[test]
    fn insufficient_data_display_includes_counts() {
        let err = TideGlassError::InsufficientData {
            required: 5,
            actual: 2,
        };
        let message = err.to_string();
        assert!(message.contains('5'));
        assert!(message.contains('2'));
    }

    #[test]
    fn serde_json_error_converts_to_serialization_variant() {
        let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let err: TideGlassError = json_err.into();
        assert!(matches!(err, TideGlassError::Serialization(_)));
    }

    #[test]
    fn io_error_converts_to_io_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing file");
        let err: TideGlassError = io_err.into();
        assert!(matches!(err, TideGlassError::Io(_)));
    }
}
