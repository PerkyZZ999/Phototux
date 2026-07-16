//! Free-transform / crop / resize commands (Phase 7).

use serde::{Deserialize, Serialize};

use crate::layer::LayerTransform;

/// Preview state while a transform tool is active (commit = one history entry).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformPreview {
    pub layer_id: u64,
    pub draft: LayerTransform,
    pub constrain_aspect: bool,
}

/// Crop rectangle in document pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CropRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Image / canvas resize request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResizeRequest {
    pub width: u32,
    pub height: u32,
    /// When true, scales layer pixels; when false, only canvas bounds change.
    pub scale_content: bool,
}

impl CropRect {
    pub fn is_valid(self) -> bool {
        self.width > 0 && self.height > 0
    }
}
