//! Typed errors for document/session mutation (library paths; no stringly errors).

use thiserror::Error;

/// Recoverable document-graph failures.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DocumentError {
    #[error("layer limit reached ({max}); remove a layer before adding another")]
    LayerLimitReached { max: usize },
    #[error("added layer missing from graph")]
    LayerMissingAfterAdd,
    #[error("no document is open")]
    NoDocument,
}

impl DocumentError {
    pub fn layer_limit(max: usize) -> Self {
        Self::LayerLimitReached { max }
    }
}
