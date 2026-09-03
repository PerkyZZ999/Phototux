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
    /// The same failure at the other end of the range, and the one the guard
    /// missed: a file may declare any size, and 0 is as undrawable as 20000.
    #[error("{got} px is not a usable document edge — each edge must be at least 1 px")]
    DimensionTooSmall { got: u32 },
}

impl DocumentError {
    pub fn layer_limit(max: usize) -> Self {
        Self::LayerLimitReached { max }
    }

    /// Reject a size the compositor cannot hold, naming the edge that is wrong.
    ///
    /// Both ends. It checked only the upper one, which was enough while the
    /// callers were three spin boxes bounded at 1 — but two of the five are
    /// file-open paths passing dimensions a `.ptx` or a PSD *declared*, and a
    /// file can declare 0.
    ///
    /// # Errors
    /// [`Self::DimensionTooSmall`] or [`Self::DimensionTooLarge`], naming the
    /// edge that is wrong.
    pub fn check_size(size: crate::DocumentSize) -> Result<(), Self> {
        if let Some(got) = size.degenerate_edge() {
            return Err(Self::DimensionTooSmall { got });
        }
        match size.oversized_edge() {
            Some(got) => Err(Self::DimensionTooLarge {
                got,
                max: crate::MAX_DOCUMENT_DIMENSION,
            }),
            None => Ok(()),
        }
    }
}
