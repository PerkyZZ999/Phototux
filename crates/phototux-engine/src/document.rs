//! Ordered layer stack document graph (ADR-011).

use crate::DocumentSize;
use crate::layer::{BlendMode, Layer, LayerId};

/// Non-destructive ordered stack. Index 0 = bottom (background).
#[derive(Debug, Clone)]
pub struct DocumentGraph {
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
            size,
            layers: Vec::new(),
            next_id: 1,
            active: None,
            revision: 0,
        };
        // Default: Background + Layer 1 (IA)
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

    pub fn set_active_index(&mut self, index: usize) -> bool {
        if let Some(l) = self.layers.get(index) {
            self.active = Some(l.id);
            true
        } else {
            false
        }
    }

    /// Visible layers bottom→top for composite.
    pub fn composite_order(&self) -> impl Iterator<Item = &Layer> {
        self.layers.iter().filter(|l| l.visible)
    }

    pub fn add_layer_top(&mut self, name: Option<String>) -> LayerId {
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
        id
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
            return None; // keep at least one layer
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
        let id = g.add_layer_top(None);
        assert_eq!(g.layer_count(), 3);
        assert_eq!(g.active_id(), Some(id));
        assert!(g.remove_layer(id).is_some());
        assert_eq!(g.layer_count(), 2);
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
}
