//! Ordered hierarchical document graph (ADR-011, ADR-017).

use serde::{Deserialize, Serialize};

use crate::DocumentSize;
use crate::error::DocumentError;
use crate::layer::{
    AdjustmentParams, BlendMode, Layer, LayerId, LayerKind, LayerMask, TextContent,
};

/// Hard cap matching the GPU compositor (`phototux_gpu::MAX_LAYERS`).
pub const MAX_LAYERS: usize = 16;

/// Graph schema version embedded in `.ptx` manifests.
pub const GRAPH_SCHEMA_VERSION: u32 = 2;

/// Non-destructive ordered stack with typed nodes. Index 0 = bottom (background).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentGraph {
    pub schema_version: u32,
    /// Stable document identity for recovery / session.
    pub document_id: u128,
    pub size: DocumentSize,
    layers: Vec<Layer>,
    next_id: u64,
    active: Option<LayerId>,
    /// Incremented when composite inputs change.
    pub revision: u64,
}

impl DocumentGraph {
    pub fn new(size: DocumentSize) -> Self {
        let mut g = Self {
            schema_version: GRAPH_SCHEMA_VERSION,
            document_id: new_document_id(),
            size,
            layers: Vec::new(),
            next_id: 1,
            active: None,
            revision: 0,
        };
        let bg = g.alloc_layer("Background");
        let l1 = g.alloc_layer("Layer 1");
        g.layers.push(bg);
        g.layers.push(l1);
        g.active = g.layers.last().map(|l| l.id);
        g.bump();
        g
    }

    /// Create a document containing one flattened raster layer.
    pub fn new_flattened(size: DocumentSize, layer_name: impl Into<String>) -> Self {
        let mut graph = Self {
            schema_version: GRAPH_SCHEMA_VERSION,
            document_id: new_document_id(),
            size,
            layers: Vec::new(),
            next_id: 1,
            active: None,
            revision: 0,
        };
        let mut layer = graph.alloc_layer("Image");
        layer.name = layer_name.into();
        graph.active = Some(layer.id);
        graph.layers.push(layer);
        graph.bump();
        graph
    }

    fn alloc_layer(&mut self, name: &str) -> Layer {
        let id = LayerId(self.next_id);
        self.next_id += 1;
        Layer::new(id, name)
    }

    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn layers_mut(&mut self) -> &mut [Layer] {
        &mut self.layers
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn active_id(&self) -> Option<LayerId> {
        self.active
    }

    pub fn active_index(&self) -> Option<usize> {
        self.active.and_then(|id| self.index_of(id))
    }

    pub fn index_of(&self, id: LayerId) -> Option<usize> {
        self.layers.iter().position(|l| l.id == id)
    }

    pub fn get(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn get_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn set_active(&mut self, id: LayerId) -> bool {
        if self.index_of(id).is_none() {
            return false;
        }
        self.active = Some(id);
        true
    }

    pub fn clear_active(&mut self) {
        self.active = None;
    }

    pub fn set_active_index(&mut self, index: usize) -> bool {
        if let Some(l) = self.layers.get(index) {
            self.active = Some(l.id);
            true
        } else {
            false
        }
    }

    /// Visible raster-capable layers bottom→top for composite (groups expand later).
    pub fn composite_order(&self) -> impl Iterator<Item = &Layer> {
        self.layers.iter().filter(|l| {
            l.visible
                && matches!(
                    l.kind,
                    LayerKind::Raster | LayerKind::Text | LayerKind::Group
                )
        })
    }

    /// Flat raster layers that own GPU textures today.
    pub fn raster_layers(&self) -> impl Iterator<Item = &Layer> {
        self.layers
            .iter()
            .filter(|l| l.kind == LayerKind::Raster || l.kind == LayerKind::Text)
    }

    pub fn can_add_layer(&self) -> bool {
        self.layers.len() < MAX_LAYERS
    }

    /// Add a raster layer on top of the stack.
    ///
    /// # Errors
    /// Returns [`DocumentError::LayerLimitReached`] when the document already has [`MAX_LAYERS`] layers.
    pub fn add_layer_top(&mut self, name: Option<String>) -> Result<LayerId, DocumentError> {
        if !self.can_add_layer() {
            return Err(DocumentError::layer_limit(MAX_LAYERS));
        }
        let n = self
            .layers
            .iter()
            .filter(|l| l.name.starts_with("Layer "))
            .count()
            + 1;
        let name = name.unwrap_or_else(|| format!("Layer {n}"));
        let layer = self.alloc_layer(&name);
        let id = layer.id;
        self.layers.push(layer);
        self.active = Some(id);
        self.bump();
        Ok(id)
    }

    /// Add a group on top.
    ///
    /// # Errors
    /// Returns [`DocumentError::LayerLimitReached`] when the layer cap is reached.
    pub fn add_group_top(&mut self, name: Option<String>) -> Result<LayerId, DocumentError> {
        if !self.can_add_layer() {
            return Err(DocumentError::layer_limit(MAX_LAYERS));
        }
        let id = LayerId(self.next_id);
        self.next_id += 1;
        let layer = Layer::group(id, name.unwrap_or_else(|| "Group".into()));
        self.layers.push(layer);
        self.active = Some(id);
        self.bump();
        Ok(id)
    }

    /// Add a text layer on top.
    ///
    /// # Errors
    /// Returns [`DocumentError::LayerLimitReached`] when the layer cap is reached.
    pub fn add_text_top(
        &mut self,
        name: Option<String>,
        content: TextContent,
    ) -> Result<LayerId, DocumentError> {
        if !self.can_add_layer() {
            return Err(DocumentError::layer_limit(MAX_LAYERS));
        }
        let id = LayerId(self.next_id);
        self.next_id += 1;
        let layer = Layer::text_layer(id, name.unwrap_or_else(|| "Text".into()), content);
        self.layers.push(layer);
        self.active = Some(id);
        self.bump();
        Ok(id)
    }

    /// Add an adjustment layer on top.
    ///
    /// # Errors
    /// Returns [`DocumentError::LayerLimitReached`] when the layer cap is reached.
    pub fn add_adjustment_top(
        &mut self,
        name: Option<String>,
        params: AdjustmentParams,
    ) -> Result<LayerId, DocumentError> {
        if !self.can_add_layer() {
            return Err(DocumentError::layer_limit(MAX_LAYERS));
        }
        let id = LayerId(self.next_id);
        self.next_id += 1;
        let layer =
            Layer::adjustment_layer(id, name.unwrap_or_else(|| "Adjustment".into()), params);
        self.layers.push(layer);
        self.active = Some(id);
        self.bump();
        Ok(id)
    }

    pub fn set_parent(&mut self, id: LayerId, parent: Option<LayerId>) -> Option<Option<LayerId>> {
        if let Some(parent_id) = parent {
            if parent_id == id || self.get(parent_id).map(|l| l.kind) != Some(LayerKind::Group) {
                return None;
            }
        }
        let layer = self.get_mut(id)?;
        let prev = layer.parent;
        layer.parent = parent;
        self.bump();
        Some(prev)
    }

    pub fn set_mask(&mut self, id: LayerId, mask: Option<LayerMask>) -> Option<Option<LayerMask>> {
        let layer = self.get_mut(id)?;
        let prev = layer.mask.clone();
        layer.mask = mask;
        self.bump();
        Some(prev)
    }

    pub fn set_clips_to_below(&mut self, id: LayerId, clips: bool) -> Option<bool> {
        let layer = self.get_mut(id)?;
        let prev = layer.clips_to_below;
        layer.clips_to_below = clips;
        self.bump();
        Some(prev)
    }

    pub fn layer_mask_flags_joined(&self) -> String {
        self.layers
            .iter()
            .map(|l| l.mask_flag().to_string())
            .collect::<Vec<_>>()
            .join("|")
    }

    pub fn layer_clips_joined(&self) -> String {
        self.layers
            .iter()
            .map(|l| if l.clips_to_below { "1" } else { "0" })
            .collect::<Vec<_>>()
            .join("|")
    }

    /// Insert a fully-specified layer (used by undo).
    pub fn insert_layer_at(&mut self, index: usize, layer: Layer) {
        let idx = index.min(self.layers.len());
        let id = layer.id;
        self.layers.insert(idx, layer);
        self.next_id = self.next_id.max(id.0 + 1);
        self.active = Some(id);
        self.bump();
    }

    pub fn remove_layer(&mut self, id: LayerId) -> Option<(usize, Layer)> {
        let idx = self.index_of(id)?;
        if self.layers.len() <= 1 {
            return None;
        }
        // Detach children from removed group.
        for layer in &mut self.layers {
            if layer.parent == Some(id) {
                layer.parent = None;
            }
        }
        let layer = self.layers.remove(idx);
        if self.active == Some(id) {
            self.active = self
                .layers
                .get(
                    idx.saturating_sub(1)
                        .min(self.layers.len().saturating_sub(1)),
                )
                .or(self.layers.last())
                .map(|l| l.id);
        }
        self.bump();
        Some((idx, layer))
    }

    pub fn move_layer(&mut self, id: LayerId, to_index: usize) -> Option<(usize, usize)> {
        let from = self.index_of(id)?;
        let to = to_index.min(self.layers.len().saturating_sub(1));
        if from == to {
            return Some((from, to));
        }
        let layer = self.layers.remove(from);
        self.layers.insert(to, layer);
        self.bump();
        Some((from, to))
    }

    pub fn set_visibility(&mut self, id: LayerId, visible: bool) -> Option<bool> {
        let layer = self.get_mut(id)?;
        let prev = layer.visible;
        layer.visible = visible;
        if prev != visible {
            self.bump();
        }
        Some(prev)
    }

    pub fn set_opacity(&mut self, id: LayerId, opacity: f32) -> Option<f32> {
        let layer = self.get_mut(id)?;
        let prev = layer.opacity;
        layer.set_opacity(opacity);
        if (prev - layer.opacity).abs() > f32::EPSILON {
            self.bump();
        }
        Some(prev)
    }

    pub fn set_blend(&mut self, id: LayerId, blend: BlendMode) -> Option<BlendMode> {
        let layer = self.get_mut(id)?;
        let prev = layer.blend;
        layer.blend = blend;
        if prev != blend {
            self.bump();
        }
        Some(prev)
    }

    pub fn rename(&mut self, id: LayerId, name: String) -> Option<String> {
        let layer = self.get_mut(id)?;
        let prev = std::mem::replace(&mut layer.name, name);
        self.bump();
        Some(prev)
    }

    /// Kind labels bottom→top for QML (`raster|group|…`).
    pub fn layer_kinds_joined(&self) -> String {
        self.layers
            .iter()
            .map(|l| l.kind.as_str())
            .collect::<Vec<_>>()
            .join("|")
    }

    /// Parent ids as decimal or `-` when root.
    pub fn layer_parents_joined(&self) -> String {
        self.layers
            .iter()
            .map(|l| {
                l.parent
                    .map(|id| id.0.to_string())
                    .unwrap_or_else(|| String::from("-"))
            })
            .collect::<Vec<_>>()
            .join("|")
    }
}

fn new_document_id() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = u128::from(std::process::id());
    nanos ^ (pid << 64) ^ 0x5054_582D_444F_432D_u128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_two_layers() {
        let g = DocumentGraph::new(DocumentSize::new(100, 100));
        assert_eq!(g.layer_count(), 2);
        assert_eq!(g.layers()[0].name, "Background");
        assert!(g.active_id().is_some());
        assert_eq!(g.schema_version, GRAPH_SCHEMA_VERSION);
    }

    #[test]
    fn flattened_document_has_named_single_layer() {
        let graph = DocumentGraph::new_flattened(DocumentSize::new(100, 50), "photo.jpg");
        assert_eq!(graph.layer_count(), 1);
        assert_eq!(graph.layers()[0].name, "photo.jpg");
        assert_eq!(graph.active_id(), Some(graph.layers()[0].id));
    }

    #[test]
    fn add_and_remove() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let id = g.add_layer_top(None).expect("add");
        assert_eq!(g.layer_count(), 3);
        assert_eq!(g.active_id(), Some(id));
        assert!(g.remove_layer(id).is_some());
        assert_eq!(g.layer_count(), 2);
    }

    #[test]
    fn rejects_add_past_layer_cap() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        while g.layer_count() < MAX_LAYERS {
            g.add_layer_top(None).expect("fill to cap");
        }
        assert!(g.add_layer_top(None).is_err());
    }

    #[test]
    fn cannot_remove_last() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let a = g.layers()[0].id;
        let b = g.layers()[1].id;
        g.remove_layer(a);
        assert!(g.remove_layer(b).is_none());
        assert_eq!(g.layer_count(), 1);
    }

    #[test]
    fn move_layer_reorders() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let top = g.layers()[1].id;
        g.move_layer(top, 0);
        assert_eq!(g.layers()[0].id, top);
    }

    #[test]
    fn composite_order_skips_hidden() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let id = g.layers()[0].id;
        g.set_visibility(id, false);
        assert_eq!(g.composite_order().count(), 1);
    }

    #[test]
    fn group_and_reparent() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let group = g.add_group_top(Some("G".into())).expect("group");
        let child = g.layers()[0].id;
        assert_eq!(g.set_parent(child, Some(group)), Some(None));
        assert_eq!(g.get(child).and_then(|l| l.parent), Some(group));
    }

    #[test]
    fn mask_flags_and_clips_joined() {
        let mut g = DocumentGraph::new(DocumentSize::new(32, 32));
        let top = g.layers()[1].id;
        assert_eq!(g.layer_mask_flags_joined(), "0|0");
        g.set_mask(top, Some(LayerMask::default()));
        assert_eq!(g.layer_mask_flags_joined(), "0|1");
        g.set_mask(
            top,
            Some(LayerMask {
                enabled: false,
                ..Default::default()
            }),
        );
        assert_eq!(g.layer_mask_flags_joined(), "0|2");
        assert_eq!(g.set_clips_to_below(top, true), Some(false));
        assert_eq!(g.layer_clips_joined(), "0|1");
    }
}
