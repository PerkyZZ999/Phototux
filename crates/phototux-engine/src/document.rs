//! Ordered hierarchical document graph (ADR-011, ADR-017).

use serde::{Deserialize, Serialize};

use crate::DocumentSize;
use crate::color_mgmt::DocumentColorState;
use crate::error::DocumentError;
use crate::layer::{
    AdjustmentParams, BlendMode, FillContent, FilterEffect, FilterParams, Layer, LayerId,
    LayerKind, LayerMask, LayerTransform, MAX_BLUR_RADIUS, ShapeContent, TextContent,
};
use crate::paths::PathDocument;

/// Hard cap matching the GPU compositor (`phototux_gpu::MAX_LAYERS`).
pub const MAX_LAYERS: usize = 16;

/// What to call a new shape layer, from the `kind` key its content carries.
///
/// Photoshop names a shape layer after the shape — "Rectangle 1", "Ellipse 2"
/// — rather than after the fact that it is a shape, and with four kinds in the
/// stack "Shape 1" through "Shape 4" says nothing the panel could not already
/// see from the badge. The stars, arrows and rounded rectangles the presets
/// offer all record themselves as `polygon`, which is what they are as far as
/// the path is concerned, so they are named as polygons here too.
///
/// An unrecognised key falls back to the kind name rather than being refused:
/// a document written by a later version can carry a shape kind this one does
/// not know, and refusing to name it would refuse to open the file.
fn shape_stem(content: &ShapeContent) -> &'static str {
    match content.kind.as_str() {
        "rect" => "Rectangle",
        "ellipse" => "Ellipse",
        "line" => "Line",
        "polygon" => "Polygon",
        _ => "Shape",
    }
}

/// How far a `parent` chain is followed before the walk gives up.
///
/// A well-formed graph nests far shallower than this — `MAX_LAYERS` is 16, so
/// even a group inside a group inside a group cannot get near it. The cap is
/// there for the malformed case: a `parent` chain corrupted into a cycle by a
/// hand-edited or truncated document would otherwise hang the walk rather
/// than return something wrong.
const MAX_NESTING_DEPTH: usize = 64;

/// Graph schema version embedded in `.ptx` manifests.
pub const GRAPH_SCHEMA_VERSION: u32 = 3;

/// Non-destructive ordered stack with typed nodes. Index 0 = bottom (background).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentGraph {
    pub schema_version: u32,
    /// Stable document identity for recovery / session.
    ///
    /// A `u128` wider than `u64::MAX`, which `serde_json::Value` cannot hold —
    /// `to_value` on a graph fails with "number out of range". `.ptx` is
    /// unaffected because it goes through `to_string`/`from_str`, which carry
    /// the full width; only a detour through `Value` would truncate it.
    pub document_id: u128,
    pub size: DocumentSize,
    layers: Vec<Layer>,
    next_id: u64,
    active: Option<LayerId>,
    /// Incremented when composite inputs change.
    pub revision: u64,
    /// Monotonic document generation for snapshot leases / save receipts (handbook Phase 2).
    #[serde(default)]
    pub generation: u64,
    /// Color profile metadata (assign ≠ convert).
    #[serde(default)]
    pub color: DocumentColorState,
    /// Free vector paths (stroke-to-raster; Shape kind is separate).
    #[serde(default)]
    pub paths: PathDocument,
    /// Opaque extension blobs (capability seams / P12); host must not interpret.
    #[serde(default)]
    pub extension_data: Vec<ExtensionBlob>,
}

/// Opaque keyed payload for future extension contribution (DR-009 seams).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionBlob {
    pub key: String,
    pub bytes: Vec<u8>,
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
            generation: 1,
            color: DocumentColorState::default(),
            paths: PathDocument::default(),
            extension_data: Vec::new(),
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
            generation: 1,
            color: DocumentColorState::default(),
            paths: PathDocument::default(),
            extension_data: Vec::new(),
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

    /// The next free default name for `stem` — `Group 1`, `Group 2`, and so on.
    ///
    /// Photoshop numbers every default name, and the reason is the layers
    /// panel: a stack holding three layers all called "Group" tells the user
    /// nothing about which is which, and renaming one to find out is work the
    /// editor should have done. Only raster layers were numbered here; groups,
    /// text, shapes and fills all took a bare kind name, and two Levels
    /// adjustments were both called "Levels".
    ///
    /// The number is the lowest that is free rather than a running count of
    /// what exists. Counting names that merely *start with* the stem — which is
    /// what the raster path used to do — hands out a duplicate as soon as one
    /// is deleted: add three layers, delete the first, and the next add is a
    /// second "Layer 3".
    #[must_use]
    pub fn next_default_name(&self, stem: &str) -> String {
        // One more candidate than there are layers, so by the pigeonhole
        // principle at least one of them is free however the existing names
        // are arranged.
        (1..=self.layers.len() + 1)
            .map(|n| format!("{stem} {n}"))
            .find(|candidate| self.layers.iter().all(|l| &l.name != candidate))
            .expect("more candidates than layers, so one is always free")
    }

    /// Replace `ids` with one fresh empty layer at the lowest of their
    /// positions, and return its id.
    ///
    /// The graph half of Merge Down and Merge Visible: the host holds the
    /// composited pixels and writes them into the layer this returns. The
    /// merged layer is fresh rather than one of the originals reused, for the
    /// reason [`Self::flatten_to_single_layer`] gives — it carries none of
    /// their masks, blend modes or effects, because those are in the pixels
    /// now.
    ///
    /// `None` when no id names a layer.
    pub fn replace_with_merged_layer(&mut self, ids: &[LayerId], name: &str) -> Option<LayerId> {
        let mut positions: Vec<usize> = ids.iter().filter_map(|id| self.index_of(*id)).collect();
        if positions.is_empty() {
            return None;
        }
        positions.sort_unstable();
        let at = positions[0];
        let parent = self.layers.get(at).and_then(|l| l.parent);
        self.layers.retain(|l| !ids.contains(&l.id));
        let mut layer = self.alloc_layer(name);
        layer.parent = parent;
        let id = layer.id;
        self.layers.insert(at.min(self.layers.len()), layer);
        self.active = Some(id);
        self.bump();
        Some(id)
    }

    /// Every layer inside `group`, transitively, in stack order.
    ///
    /// Membership is the `parent` chain rather than a range of indices: a
    /// group is a parent in a flat list, and a nested group's children name
    /// the *inner* group as their parent, so walking indices between the
    /// group and its neighbours would either miss them or take layers that
    /// merely happen to sit there. The walk is depth-capped, so a `parent`
    /// chain corrupted into a cycle by a malformed document returns what it
    /// has rather than hanging.
    #[must_use]
    pub fn descendants_of(&self, group: LayerId) -> Vec<LayerId> {
        self.layers
            .iter()
            .filter(|layer| {
                let mut parent = layer.parent;
                for _ in 0..MAX_NESTING_DEPTH {
                    match parent {
                        Some(id) if id == group => return true,
                        Some(id) => parent = self.get(id).and_then(|l| l.parent),
                        None => return false,
                    }
                }
                false
            })
            .map(|layer| layer.id)
            .collect()
    }

    /// How deeply `layer` sits inside groups — `0` at the root of the stack.
    ///
    /// The panel indents a row by this, which is the only cue that a layer is
    /// inside the group above it: a group is a parent in a flat list, so
    /// without the indent a grouped stack and an ungrouped one draw
    /// identically. Depth-capped like [`Self::descendants_of`], and for the
    /// same reason — an indent computed from a cyclic chain would otherwise
    /// run the row off the edge of the dock.
    #[must_use]
    pub fn depth_of(&self, layer: LayerId) -> usize {
        let mut depth = 0;
        let mut parent = self.get(layer).and_then(|l| l.parent);
        while let Some(id) = parent {
            depth += 1;
            if depth == MAX_NESTING_DEPTH {
                break;
            }
            parent = self.get(id).and_then(|l| l.parent);
        }
        depth
    }

    /// Replace the whole stack with one empty layer and return its id.
    ///
    /// The graph half of Flatten Image: the host holds the composited pixels
    /// and writes them into the layer this returns. A fresh id rather than
    /// reusing the bottom layer's, because the flattened layer is not the
    /// bottom layer with different pixels — it has no mask, no clipping, no
    /// blend mode and no effects, and reusing the id would leave every
    /// reference to the old one silently pointing at something else.
    pub fn flatten_to_single_layer(&mut self, name: &str) -> LayerId {
        let layer = self.alloc_layer(name);
        let id = layer.id;
        self.layers.clear();
        self.layers.push(layer);
        self.active = Some(id);
        self.bump();
        id
    }

    /// A fresh layer record with an unused id, for callers that build the rest
    /// of it themselves — duplicating a layer, chiefly, where every field but
    /// the id and the name comes from the source.
    pub fn alloc_layer_record(&mut self, name: &str) -> Layer {
        self.alloc_layer(name)
    }

    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    /// Advance the document generation (command commits / host pixel commits).
    pub fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1).max(1);
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

    /// Visible layers bottom→top for composite (groups expand later).
    pub fn composite_order(&self) -> impl Iterator<Item = &Layer> {
        self.layers.iter().filter(|l| {
            l.visible
                && matches!(
                    l.kind,
                    LayerKind::Raster
                        | LayerKind::Text
                        | LayerKind::Group
                        | LayerKind::Adjustment
                        | LayerKind::Fill
                        | LayerKind::Shape
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
        let name = name.unwrap_or_else(|| self.next_default_name("Layer"));
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
        let layer = Layer::group(id, name.unwrap_or_else(|| self.next_default_name("Group")));
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
        let layer = Layer::text_layer(
            id,
            name.unwrap_or_else(|| self.next_default_name("Text")),
            content,
        );
        self.layers.push(layer);
        self.active = Some(id);
        self.bump();
        Ok(id)
    }

    /// Add a shape layer on top (DR-027).
    ///
    /// # Errors
    /// Returns [`DocumentError::LayerLimitReached`] when the layer cap is reached.
    pub fn add_shape_top(
        &mut self,
        name: Option<String>,
        content: ShapeContent,
    ) -> Result<LayerId, DocumentError> {
        if !self.can_add_layer() {
            return Err(DocumentError::layer_limit(MAX_LAYERS));
        }
        let id = LayerId(self.next_id);
        self.next_id += 1;
        let layer = Layer::shape_layer(
            id,
            name.unwrap_or_else(|| self.next_default_name(shape_stem(&content))),
            content,
        );
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
        let layer = Layer::adjustment_layer(
            id,
            name.unwrap_or_else(|| self.next_default_name("Adjustment")),
            params,
        );
        self.layers.push(layer);
        self.active = Some(id);
        self.bump();
        Ok(id)
    }

    /// Add a solid fill layer on top.
    ///
    /// # Errors
    /// Returns [`DocumentError::LayerLimitReached`] when the layer cap is reached.
    pub fn add_fill_top(
        &mut self,
        name: Option<String>,
        content: FillContent,
    ) -> Result<LayerId, DocumentError> {
        if !self.can_add_layer() {
            return Err(DocumentError::layer_limit(MAX_LAYERS));
        }
        let id = LayerId(self.next_id);
        self.next_id += 1;
        let layer = Layer::fill_layer(
            id,
            name.unwrap_or_else(|| self.next_default_name("Fill")),
            content,
        );
        self.layers.push(layer);
        self.active = Some(id);
        self.bump();
        Ok(id)
    }

    pub fn set_fill(
        &mut self,
        id: LayerId,
        fill: Option<FillContent>,
    ) -> Option<Option<FillContent>> {
        let layer = self.get_mut(id)?;
        if layer.kind != LayerKind::Fill {
            return None;
        }
        let prev = layer.fill.clone();
        layer.fill = fill;
        self.bump();
        Some(prev)
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

    /// Replace a layer's blend ranges, returning the ones it had.
    pub fn set_blend_if(
        &mut self,
        id: LayerId,
        blend_if: crate::BlendIf,
    ) -> Option<crate::BlendIf> {
        let layer = self.get_mut(id)?;
        let prev = layer.blend_if;
        layer.blend_if = blend_if;
        self.bump();
        Some(prev)
    }

    /// Replace a layer's transform, returning the one it had.
    ///
    /// Used by align/distribute, which moves layers by writing their
    /// translation rather than by baking pixels — the composite already
    /// honours `layer.transform`, so the move stays non-destructive and
    /// survives into `.ptx`.
    pub fn set_transform(
        &mut self,
        id: LayerId,
        transform: LayerTransform,
    ) -> Option<LayerTransform> {
        let layer = self.get_mut(id)?;
        let prev = layer.transform;
        layer.transform = transform;
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

    /// Reorder the stack to match `order` (must be a permutation of current layer ids).
    pub fn reorder_stack(&mut self, order: &[LayerId]) -> bool {
        if order.len() != self.layers.len() {
            return false;
        }
        let mut next = Vec::with_capacity(order.len());
        for id in order {
            let Some(layer) = self.layers.iter().find(|l| l.id == *id).cloned() else {
                return false;
            };
            next.push(layer);
        }
        if next
            .iter()
            .map(|l| l.id)
            .eq(self.layers.iter().map(|l| l.id))
        {
            return true;
        }
        self.layers = next;
        self.bump();
        true
    }

    pub fn stack_order(&self) -> Vec<LayerId> {
        self.layers.iter().map(|l| l.id).collect()
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

    /// Replace adjustment parameters on an adjustment layer.
    pub fn set_adjustment(
        &mut self,
        id: LayerId,
        params: Option<AdjustmentParams>,
    ) -> Option<Option<AdjustmentParams>> {
        let layer = self.get_mut(id)?;
        if layer.kind != LayerKind::Adjustment {
            return None;
        }
        let prev = layer.adjustment.clone();
        layer.adjustment = params.map(AdjustmentParams::clamped);
        self.bump();
        Some(prev)
    }

    /// Replace the full nondestructive effect stack on a layer.
    pub fn set_effects(
        &mut self,
        id: LayerId,
        effects: Vec<FilterEffect>,
    ) -> Option<Vec<FilterEffect>> {
        let layer = self.get_mut(id)?;
        let prev = layer.effects.clone();
        layer.effects = effects
            .into_iter()
            .map(|mut effect| {
                effect.params = effect.params.clamped();
                effect.opacity = effect.opacity.clamp(0.0, 1.0);
                effect
            })
            .collect();
        self.bump();
        Some(prev)
    }

    /// Append a Gaussian Blur effect to a raster layer. Returns `(prev_effects, effect_id)`.
    pub fn add_gaussian_blur(
        &mut self,
        id: LayerId,
        radius: f32,
    ) -> Option<(Vec<FilterEffect>, u64)> {
        self.add_filter_effect(id, |effect_id| {
            FilterEffect::gaussian_blur(effect_id, radius.clamp(0.0, MAX_BLUR_RADIUS))
        })
    }

    /// Append a Motion Blur effect to a raster layer.
    pub fn add_motion_blur(
        &mut self,
        id: LayerId,
        distance: f32,
        angle_deg: f32,
    ) -> Option<(Vec<FilterEffect>, u64)> {
        self.add_filter_effect(id, |effect_id| {
            FilterEffect::motion_blur(effect_id, distance, angle_deg)
        })
    }

    /// Append an Emboss effect to a raster layer.
    pub fn add_emboss(
        &mut self,
        id: LayerId,
        strength: f32,
        angle_deg: f32,
    ) -> Option<(Vec<FilterEffect>, u64)> {
        self.add_filter_effect(id, |effect_id| {
            FilterEffect::emboss(effect_id, strength, angle_deg)
        })
    }

    /// Append a Sharpen effect to a raster layer.
    pub fn add_sharpen(&mut self, id: LayerId, amount: f32) -> Option<(Vec<FilterEffect>, u64)> {
        self.add_filter_effect(id, |effect_id| FilterEffect::sharpen(effect_id, amount))
    }

    /// Append a Noise effect to a raster layer.
    pub fn add_noise(&mut self, id: LayerId, amount: f32) -> Option<(Vec<FilterEffect>, u64)> {
        self.add_filter_effect(id, |effect_id| FilterEffect::noise(effect_id, amount))
    }

    /// Append an effect of `params` to a raster layer, named for its kind.
    ///
    /// One entry point rather than a wrapper per kind: those wrappers were a
    /// second list of the filter vocabulary, and the command layer had a third
    /// mapping kind keys onto them, so a kind added to `FilterParams` reached
    /// neither.
    pub fn add_effect(
        &mut self,
        id: LayerId,
        params: FilterParams,
    ) -> Option<(Vec<FilterEffect>, u64)> {
        self.add_filter_effect(id, |effect_id| FilterEffect {
            id: effect_id,
            name: params.label().to_owned(),
            enabled: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            params: params.clamped(),
        })
    }

    fn add_filter_effect(
        &mut self,
        id: LayerId,
        make: impl FnOnce(u64) -> FilterEffect,
    ) -> Option<(Vec<FilterEffect>, u64)> {
        let layer = self.get(id)?;
        if layer.kind != LayerKind::Raster {
            return None;
        }
        let effect_id = layer
            .effects
            .iter()
            .map(|e| e.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let prev = layer.effects.clone();
        let mut next = prev.clone();
        next.push(make(effect_id));
        let _ = self.set_effects(id, next)?;
        Some((prev, effect_id))
    }

    /// Update the first Gaussian Blur effect radius (creates nothing).
    pub fn set_gaussian_radius(&mut self, id: LayerId, radius: f32) -> Option<Vec<FilterEffect>> {
        let layer = self.get(id)?;
        let prev = layer.effects.clone();
        let mut next = prev.clone();
        let radius = radius.clamp(0.0, MAX_BLUR_RADIUS);
        let mut found = false;
        for effect in &mut next {
            if let FilterParams::GaussianBlur { radius: r } = &mut effect.params {
                if (*r - radius).abs() < f32::EPSILON {
                    return Some(prev);
                }
                *r = radius;
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
        let _ = self.set_effects(id, next)?;
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
    fn extension_blob_roundtrips_in_graph_json() {
        let mut g = DocumentGraph::new(DocumentSize::new(8, 8));
        g.extension_data.push(crate::ExtensionBlob {
            key: "com.example.plugin".into(),
            bytes: vec![1, 2, 3, 4],
        });
        let json = serde_json::to_string(&g).expect("ser");
        let back: DocumentGraph = serde_json::from_str(&json).expect("de");
        assert_eq!(back.extension_data.len(), 1);
        assert_eq!(back.extension_data[0].key, "com.example.plugin");
        assert_eq!(back.extension_data[0].bytes, vec![1, 2, 3, 4]);
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

    #[test]
    fn every_default_name_is_numbered() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let group = g.add_group_top(None).expect("group");
        let text = g
            .add_text_top(None, crate::layer::TextContent::default())
            .expect("text");
        let fill = g
            .add_fill_top(None, crate::layer::FillContent::default())
            .expect("fill");

        assert_eq!(g.get(group).expect("group").name, "Group 1");
        assert_eq!(g.get(text).expect("text").name, "Text 1");
        assert_eq!(g.get(fill).expect("fill").name, "Fill 1");
    }

    /// Two of the same kind must not end up with the same name — that is the
    /// whole point, because the panel shows the name and nothing else.
    #[test]
    fn a_second_group_is_not_also_group_one() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let first = g.add_group_top(None).expect("group");
        let second = g.add_group_top(None).expect("group");
        assert_eq!(g.get(first).expect("group").name, "Group 1");
        assert_eq!(g.get(second).expect("group").name, "Group 2");
    }

    /// The number is the lowest free one, not a count of what exists.
    ///
    /// Counting handed out a duplicate the moment a layer was deleted: three
    /// layers, delete the first, and the count says two so the next add is a
    /// second "Layer 3".
    #[test]
    fn a_deleted_number_is_reused_rather_than_duplicated() {
        // A new document already ships a "Layer 1" above its background, so
        // the first layer the user adds is "Layer 2". Counting names that
        // start with the stem got this right only until something was deleted.
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let two = g.add_layer_top(None).expect("layer");
        let three = g.add_layer_top(None).expect("layer");
        assert_eq!(g.get(two).expect("layer").name, "Layer 2");
        assert_eq!(g.get(three).expect("layer").name, "Layer 3");

        g.remove_layer(two);
        let next = g.add_layer_top(None).expect("layer");
        assert_eq!(
            g.get(next).expect("layer").name,
            "Layer 2",
            "the freed number comes back rather than a second Layer 3"
        );
    }

    /// A name the user typed is theirs. The next default steps over it rather
    /// than colliding with it.
    #[test]
    fn a_renamed_layer_still_reserves_its_number() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let renamed = g.add_layer_top(None).expect("layer");
        g.get_mut(renamed).expect("layer").name = "Layer 3".to_owned();
        let next = g.add_layer_top(None).expect("layer");
        assert_eq!(g.get(next).expect("layer").name, "Layer 2");
        let after = g.add_layer_top(None).expect("layer");
        assert_eq!(g.get(after).expect("layer").name, "Layer 4");
    }

    #[test]
    fn a_shape_layer_is_named_for_its_shape() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let named = |g: &mut DocumentGraph, kind: &str| {
            let content = ShapeContent {
                kind: kind.to_owned(),
                ..Default::default()
            };
            let id = g.add_shape_top(None, content).expect("shape");
            g.get(id).expect("shape").name.clone()
        };
        assert_eq!(named(&mut g, "rect"), "Rectangle 1");
        assert_eq!(named(&mut g, "ellipse"), "Ellipse 1");
        assert_eq!(named(&mut g, "polygon"), "Polygon 1");
        assert_eq!(named(&mut g, "line"), "Line 1");
        assert_eq!(
            named(&mut g, "hyperbola"),
            "Shape 1",
            "a kind from a later version is named rather than refused"
        );
    }

    #[test]
    fn depth_counts_the_groups_a_layer_is_inside() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let outer = g.add_group_top(None).expect("group");
        let inner = g.add_group_top(None).expect("group");
        let leaf = g.add_layer_top(None).expect("layer");
        g.get_mut(inner).expect("group").parent = Some(outer);
        g.get_mut(leaf).expect("layer").parent = Some(inner);

        assert_eq!(g.depth_of(outer), 0, "a root group is not inside anything");
        assert_eq!(g.depth_of(inner), 1);
        assert_eq!(g.depth_of(leaf), 2);
    }

    #[test]
    fn depth_of_an_unknown_layer_is_zero() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let gone = g.add_layer_top(None).expect("layer");
        g.remove_layer(gone);
        assert_eq!(
            g.depth_of(gone),
            0,
            "a row for a layer that is no longer there draws unindented \
             rather than panicking"
        );
    }

    /// The cap is not decoration. A `.ptx` edited by hand can name a group as
    /// its own ancestor, and the panel asks for a depth on every row it draws.
    #[test]
    fn a_cyclic_parent_chain_terminates_at_the_cap() {
        let mut g = DocumentGraph::new(DocumentSize::new(64, 64));
        let a = g.add_group_top(None).expect("group");
        let b = g.add_group_top(None).expect("group");
        g.get_mut(a).expect("group").parent = Some(b);
        g.get_mut(b).expect("group").parent = Some(a);

        assert_eq!(g.depth_of(a), MAX_NESTING_DEPTH);
        assert_eq!(
            g.descendants_of(a).len(),
            g.layer_count() - g.layers().iter().filter(|l| l.parent.is_none()).count(),
            "the descendant walk stops too, rather than hanging"
        );
    }
}

/// Where the existing image sits inside a resized canvas.
///
/// Photoshop's nine-cell anchor grid. The vocabulary is here rather than in
/// the shell because the offset is arithmetic the engine can test: a canvas
/// grown by an odd number of pixels has to land somewhere, and "somewhere"
/// being different in the dialog and in the resize is exactly the kind of
/// half-pixel drift nobody notices until an edge is wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CanvasAnchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    #[default]
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl CanvasAnchor {
    /// Reading order, which is the order the dialog draws the grid in.
    pub const ALL: [Self; 9] = [
        Self::TopLeft,
        Self::Top,
        Self::TopRight,
        Self::Left,
        Self::Center,
        Self::Right,
        Self::BottomLeft,
        Self::Bottom,
        Self::BottomRight,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TopLeft => "top-left",
            Self::Top => "top",
            Self::TopRight => "top-right",
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
            Self::BottomLeft => "bottom-left",
            Self::Bottom => "bottom",
            Self::BottomRight => "bottom-right",
        }
    }

    /// Parse a wire id. `None` rather than a default, because anchoring
    /// somewhere the user did not pick moves the whole image.
    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.as_str() == id)
    }

    /// Offset of the old image's top-left corner inside the new canvas.
    ///
    /// Negative when the canvas shrinks and the image is centred or trailing —
    /// the caller clips.
    #[must_use]
    pub fn offset(self, old: DocumentSize, new: DocumentSize) -> (i64, i64) {
        let dx = i64::from(new.width) - i64::from(old.width);
        let dy = i64::from(new.height) - i64::from(old.height);
        let x = match self {
            Self::TopLeft | Self::Left | Self::BottomLeft => 0,
            Self::Top | Self::Center | Self::Bottom => dx.div_euclid(2),
            Self::TopRight | Self::Right | Self::BottomRight => dx,
        };
        let y = match self {
            Self::TopLeft | Self::Top | Self::TopRight => 0,
            Self::Left | Self::Center | Self::Right => dy.div_euclid(2),
            Self::BottomLeft | Self::Bottom | Self::BottomRight => dy,
        };
        (x, y)
    }
}

#[cfg(test)]
mod canvas_anchor_tests {
    use super::{CanvasAnchor, DocumentSize};

    #[test]
    fn every_anchor_round_trips_through_its_wire_id() {
        for anchor in CanvasAnchor::ALL {
            assert_eq!(CanvasAnchor::parse(anchor.as_str()), Some(anchor));
        }
        assert_eq!(CanvasAnchor::parse("middle"), None);
    }

    #[test]
    fn growing_places_the_image_where_the_anchor_says() {
        let old = DocumentSize::new(100, 100);
        let new = DocumentSize::new(200, 300);
        assert_eq!(CanvasAnchor::TopLeft.offset(old, new), (0, 0));
        assert_eq!(CanvasAnchor::Center.offset(old, new), (50, 100));
        assert_eq!(CanvasAnchor::BottomRight.offset(old, new), (100, 200));
        assert_eq!(CanvasAnchor::Top.offset(old, new), (50, 0));
        assert_eq!(CanvasAnchor::Left.offset(old, new), (0, 100));
    }

    /// Shrinking is the same arithmetic with a negative difference, and the
    /// centred case must floor rather than truncate toward zero — otherwise a
    /// canvas grown by one pixel and one shrunk by one anchor differently.
    #[test]
    fn shrinking_offsets_are_negative_and_floor() {
        let old = DocumentSize::new(100, 100);
        assert_eq!(
            CanvasAnchor::Center.offset(old, DocumentSize::new(99, 99)),
            (-1, -1)
        );
        assert_eq!(
            CanvasAnchor::Center.offset(old, DocumentSize::new(101, 101)),
            (0, 0)
        );
        assert_eq!(
            CanvasAnchor::BottomRight.offset(old, DocumentSize::new(50, 50)),
            (-50, -50)
        );
    }
}
