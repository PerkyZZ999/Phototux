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
    /// The compositor allocates one texture per layer at the document's size,
    /// so an edge past the device limit cannot be drawn at all — and wgpu
    /// fails it silently, leaving a document that looks open and renders
    /// nothing.
    #[error("{got} px is larger than this GPU can composite — the limit is {max} px on each edge")]
    DimensionTooLarge { got: u32, max: u32 },
}

impl DocumentError {
    pub fn layer_limit(max: usize) -> Self {
        Self::LayerLimitReached { max }
    }

    /// Reject a size the compositor cannot hold, naming the edge that is wrong.
    pub fn check_size(size: crate::DocumentSize) -> Result<(), Self> {
        match size.oversized_edge() {
            Some(got) => Err(Self::DimensionTooLarge {
                got,
                max: crate::MAX_DOCUMENT_DIMENSION,
            }),
            None => Ok(()),
        }
    }
}
