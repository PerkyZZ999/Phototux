//! Bounded immutable pixel snapshot publisher (handbook 17 / DR-005 / DR-028 E).
//!
//! Workers hold [`Arc<PixelSnapshot>`] keyed by document generation. Authority stays
//! in [`crate::DocumentGraph`]; snapshots are reconstructible caches and may be
//! dropped under budget.

use std::sync::Arc;

use thiserror::Error;

use crate::DocumentSnapshotLease;
use crate::cpu_composite::{CpuLayerRef, composite_rgba8};

/// Soft ceiling for a single published composite (64 MiB).
///
/// Keeps interactive sessions from retaining unbounded CPU mirrors of 8K stacks.
pub const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;

/// Immutable pixel snapshot for a document generation.
#[derive(Debug, Clone)]
pub struct PixelSnapshot {
    pub lease: DocumentSnapshotLease,
    /// Flattened straight RGBA8 composite (`width * height * 4`).
    pub composite_rgba: Arc<[u8]>,
}

impl PixelSnapshot {
    pub fn byte_len(&self) -> usize {
        self.composite_rgba.len()
    }

    pub fn matches_generation(&self, generation: u64) -> bool {
        self.lease.generation == generation
    }

    pub fn matches_document(&self, document_id: u128) -> bool {
        self.lease.document_id == document_id
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SnapshotError {
    #[error("no document lease available")]
    NoDocument,
    #[error("snapshot would use {need} bytes (max {max})")]
    BudgetExceeded { need: usize, max: usize },
    #[error("composite failed: {0}")]
    Composite(String),
}

/// Latest published pixel snapshot for one document session.
#[derive(Debug, Default)]
pub struct SnapshotPublisher {
    latest: Option<Arc<PixelSnapshot>>,
}

impl SnapshotPublisher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn latest(&self) -> Option<Arc<PixelSnapshot>> {
        self.latest.clone()
    }

    pub fn clear(&mut self) {
        self.latest = None;
    }

    /// Drop cache when generation advances without a fresh publish.
    pub fn invalidate_if_stale(&mut self, generation: u64) {
        if self
            .latest
            .as_ref()
            .is_some_and(|s| s.lease.generation != generation)
        {
            self.latest = None;
        }
    }

    /// Publish a composite built from CPU layer refs under the given lease.
    pub fn publish_composite(
        &mut self,
        lease: DocumentSnapshotLease,
        layers: &[CpuLayerRef<'_>],
    ) -> Result<Arc<PixelSnapshot>, SnapshotError> {
        let need = (lease.width as usize)
            .saturating_mul(lease.height as usize)
            .saturating_mul(4);
        if need == 0 {
            return Err(SnapshotError::Composite("zero dimensions".into()));
        }
        if need > MAX_SNAPSHOT_BYTES {
            return Err(SnapshotError::BudgetExceeded {
                need,
                max: MAX_SNAPSHOT_BYTES,
            });
        }
        let rgba =
            composite_rgba8(lease.width, lease.height, layers).map_err(SnapshotError::Composite)?;
        let snap = Arc::new(PixelSnapshot {
            lease,
            composite_rgba: Arc::from(rgba.into_boxed_slice()),
        });
        self.latest = Some(Arc::clone(&snap));
        Ok(snap)
    }

    /// Publish an already-composited buffer (host GPU readback path).
    pub fn publish_rgba_buffer(
        &mut self,
        lease: DocumentSnapshotLease,
        rgba: Vec<u8>,
    ) -> Result<Arc<PixelSnapshot>, SnapshotError> {
        let expect = (lease.width as usize)
            .saturating_mul(lease.height as usize)
            .saturating_mul(4);
        if rgba.len() != expect {
            return Err(SnapshotError::Composite(format!(
                "buffer length {} != expected {expect}",
                rgba.len()
            )));
        }
        if rgba.len() > MAX_SNAPSHOT_BYTES {
            return Err(SnapshotError::BudgetExceeded {
                need: rgba.len(),
                max: MAX_SNAPSHOT_BYTES,
            });
        }
        let snap = Arc::new(PixelSnapshot {
            lease,
            composite_rgba: Arc::from(rgba.into_boxed_slice()),
        });
        self.latest = Some(Arc::clone(&snap));
        Ok(snap)
    }
}

/// Helper: one solid layer ref for tests / fixtures.
pub fn solid_layer_rgba(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
    let n = (width as usize) * (height as usize);
    let mut v = Vec::with_capacity(n * 4);
    for _ in 0..n {
        v.extend_from_slice(&rgba);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::BlendMode;

    fn lease(w: u32, h: u32, generation: u64) -> DocumentSnapshotLease {
        DocumentSnapshotLease {
            document_id: 1,
            generation,
            revision: 1,
            width: w,
            height: h,
            active_layer: None,
            layer_count: 1,
        }
    }

    #[test]
    fn publish_composite_is_immutable_and_generation_keyed() {
        let mut pubr = SnapshotPublisher::new();
        let buf = solid_layer_rgba(8, 8, [10, 20, 30, 255]);
        let layers = [CpuLayerRef {
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            rgba: &buf,
        }];
        let snap = pubr
            .publish_composite(lease(8, 8, 3), &layers)
            .expect("publish");
        assert!(snap.matches_generation(3));
        assert_eq!(snap.byte_len(), 8 * 8 * 4);
        pubr.invalidate_if_stale(4);
        assert!(pubr.latest().is_none());
    }

    #[test]
    fn budget_refuses_huge_dims() {
        let mut pubr = SnapshotPublisher::new();
        // 8192² RGBA ≈ 256 MiB > 64 MiB cap.
        let err = pubr
            .publish_composite(lease(8192, 8192, 1), &[])
            .expect_err("budget");
        assert!(matches!(err, SnapshotError::BudgetExceeded { .. }));
    }
}
