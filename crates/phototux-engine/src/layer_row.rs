//! One row of the layers panel, as data (handbook 01).
//!
//! The panel used to be fed six pipe-joined strings — names, visibility,
//! kinds, mask flags, clipping and selection — each rebuilt whenever anything
//! about any layer changed, then split back apart in QML and re-indexed per
//! delegate. Six projections of one list, index-aligned by convention only:
//! nothing checked that they were the same length, and a delegate reading past
//! the end of one of them silently fell back to a default that looked like real
//! data (a visible, unselected, unmasked raster layer).
//!
//! A row is the same information with the alignment made structural. Building
//! them here rather than in the qtbridge object keeps the two things that are
//! actually easy to get wrong — the display order and the flag encodings —
//! testable without a Qt session (DR-022).

use crate::document::DocumentGraph;
use crate::layer::LayerId;

/// A layer as the panel needs to draw it, in display order.
///
/// Deliberately flat and owned: this crosses into Qt as a model item, and a
/// row that borrowed from the graph would pin it for as long as the view held
/// the row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerRow {
    pub name: String,
    /// `raster`, `text`, `shape`, … — the panel picks its icon from this.
    pub kind: String,
    /// One-letter marker, empty for an ordinary raster layer.
    pub kind_badge: String,
    /// Display name of the kind, for tooltips and assistive technology.
    pub kind_label: String,
    pub visible: bool,
    /// `0` no mask, `1` mask enabled, `2` mask disabled.
    ///
    /// Three states rather than two booleans because that is what
    /// [`crate::layer::Layer::mask_flag`] already answers, and splitting it
    /// here would make a fourth, impossible state representable.
    pub mask_flag: i32,
    pub clips_to_below: bool,
    pub selected: bool,
    /// Whether this is the single active layer, as distinct from being part of
    /// a multi-layer selection.
    pub active: bool,
    /// Position in the graph's own bottom→top order.
    ///
    /// Rows are emitted top→bottom because that is how the panel shows them,
    /// which means the row's position in this list is *not* the index the
    /// engine's commands take. Carrying the engine index on the row is what
    /// lets the delegate call `selectLayerClick` without recomputing
    /// `count - 1 - i`, an inversion that was previously written out at every
    /// call site.
    pub stack_index: i32,
    /// How many groups this layer is inside — `0` at the root of the stack.
    ///
    /// The panel indents by this. It is a property of the graph rather than
    /// of the row's neighbours: rows are a flat list, so a delegate cannot
    /// tell a group's child from the layer that merely follows the group
    /// without being told.
    pub depth: i32,
}

/// Build the panel's rows for `document`, top of the stack first.
///
/// `selected` is the object-selection set, which is independent of the active
/// layer: a layer can be active without being in it, and several layers can be
/// selected with only one active.
#[must_use]
pub fn layer_rows(document: &DocumentGraph, selected: &[LayerId]) -> Vec<LayerRow> {
    let active = document.active_id();
    let count = document.layers().len();
    document
        .layers()
        .iter()
        .enumerate()
        .map(|(index, layer)| LayerRow {
            name: layer.name.clone(),
            kind: layer.kind.as_str().to_owned(),
            kind_badge: layer.kind.badge().to_owned(),
            kind_label: layer.kind.label().to_owned(),
            visible: layer.visible,
            mask_flag: i32::from(layer.mask_flag()),
            clips_to_below: layer.clips_to_below,
            selected: selected.contains(&layer.id),
            active: active == Some(layer.id),
            stack_index: i32::try_from(index).unwrap_or(i32::MAX),
            depth: i32::try_from(document.depth_of(layer.id)).unwrap_or(i32::MAX),
        })
        .rev()
        .take(count)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentSize, LayerKind};

    /// A graph whose stack is exactly `names`, bottom first.
    ///
    /// `DocumentGraph::new` seeds a starter stack of its own and offers no
    /// public way to empty itself, so seeded layers are reused by renaming and
    /// any surplus is removed. Every index in the tests below then means the
    /// position it says, rather than that position plus however many layers a
    /// new document happens to come with.
    fn document_with(names: &[&str]) -> DocumentGraph {
        let mut document = DocumentGraph::new(DocumentSize::new(64, 64));
        let seeded: Vec<LayerId> = document.layers().iter().map(|l| l.id).collect();
        for (i, name) in names.iter().enumerate() {
            match seeded.get(i) {
                Some(id) => document.get_mut(*id).expect("seeded layer").name = (*name).to_owned(),
                None => {
                    document
                        .add_layer_top(Some((*name).to_owned()))
                        .expect("add layer");
                }
            }
        }
        for id in seeded.iter().skip(names.len()) {
            document.remove_layer(*id);
        }
        assert_eq!(
            document.layers().len(),
            names.len(),
            "helper must produce exactly the requested stack"
        );
        if let Some(first) = document.layers().first().map(|l| l.id) {
            document.set_active(first);
        }
        document
    }

    fn layer_id_at(document: &DocumentGraph, index: usize) -> LayerId {
        document.layers()[index].id
    }

    #[test]
    fn rows_are_emitted_top_of_stack_first() {
        let document = document_with(&["bottom", "middle", "top"]);
        let rows = layer_rows(&document, &[]);
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["top", "middle", "bottom"],
            "the panel draws the top of the stack first"
        );
    }

    /// The row's own position is display order; `stack_index` is what the
    /// engine's commands take. Conflating them is the off-byte-one this type
    /// exists to remove.
    #[test]
    fn stack_index_stays_in_graph_order_while_rows_are_reversed() {
        let document = document_with(&["bottom", "middle", "top"]);
        let rows = layer_rows(&document, &[]);
        assert_eq!(rows[0].name, "top");
        assert_eq!(rows[0].stack_index, 2, "top of the stack is the last layer");
        assert_eq!(rows[2].name, "bottom");
        assert_eq!(rows[2].stack_index, 0);
    }

    /// A graph cannot be emptied — `remove_layer` refuses the last one — so
    /// "no rows" is not a state the panel reaches through this function. It
    /// reaches it by having no document at all, which the caller represents as
    /// an empty list without asking here.
    #[test]
    fn a_graph_always_yields_at_least_one_row() {
        let mut document = DocumentGraph::new(DocumentSize::new(8, 8));
        let ids: Vec<LayerId> = document.layers().iter().map(|l| l.id).collect();
        for id in ids {
            document.remove_layer(id);
        }
        assert_eq!(
            layer_rows(&document, &[]).len(),
            1,
            "the last layer cannot be removed, so one row always survives"
        );
    }

    #[test]
    fn selection_and_active_are_independent() {
        let mut document = document_with(&["a", "b", "c"]);
        let (a, b, c) = (
            layer_id_at(&document, 0),
            layer_id_at(&document, 1),
            layer_id_at(&document, 2),
        );
        document.set_active(b);
        let rows = layer_rows(&document, &[a, c]);
        let by_name = |n: &str| rows.iter().find(|r| r.name == n).expect("row").clone();

        assert!(by_name("b").active, "b is active");
        assert!(!by_name("b").selected, "but not in the selection set");
        assert!(by_name("a").selected && !by_name("a").active);
        assert!(by_name("c").selected && !by_name("c").active);
    }

    #[test]
    fn exactly_one_row_is_active() {
        let document = document_with(&["a", "b", "c"]);
        let rows = layer_rows(&document, &[]);
        assert_eq!(rows.iter().filter(|r| r.active).count(), 1);
    }

    /// The three mask states have to survive as three: a layer with a disabled
    /// mask is not a layer without one, and the panel draws them differently.
    #[test]
    fn mask_flag_distinguishes_absent_enabled_and_disabled() {
        let mut document = document_with(&["none", "on", "off"]);
        document.layers_mut()[1].mask = Some(crate::layer::LayerMask::default());
        document.layers_mut()[2].mask = Some(crate::layer::LayerMask {
            enabled: false,
            ..Default::default()
        });
        let rows = layer_rows(&document, &[]);
        let flag = |n: &str| rows.iter().find(|r| r.name == n).expect("row").mask_flag;
        assert_eq!(flag("none"), 0);
        assert_eq!(flag("on"), 1);
        assert_eq!(flag("off"), 2);
    }

    #[test]
    fn rows_carry_visibility_and_clipping_per_layer() {
        let mut document = document_with(&["a", "b"]);
        document.layers_mut()[0].visible = false;
        document.layers_mut()[1].clips_to_below = true;
        let rows = layer_rows(&document, &[]);
        let by_name = |n: &str| rows.iter().find(|r| r.name == n).expect("row").clone();
        assert!(!by_name("a").visible, "a was hidden");
        assert!(by_name("b").visible, "b was not");
        assert!(by_name("b").clips_to_below);
        assert!(!by_name("a").clips_to_below);
    }

    /// Every row describes one layer, so the count cannot drift from the
    /// graph's — which is exactly what six independently built strings could
    /// do without anything noticing.
    #[test]
    fn one_row_per_layer_always() {
        for n in 1..8usize {
            let names: Vec<String> = (0..n).map(|i| format!("l{i}")).collect();
            let refs: Vec<&str> = names.iter().map(String::as_str).collect();
            let document = document_with(&refs);
            assert_eq!(layer_rows(&document, &[]).len(), n);
        }
    }

    #[test]
    fn only_the_default_kind_goes_unmarked() {
        // The badge means "this is not an ordinary raster layer". A second
        // empty badge would make two kinds indistinguishable in the panel, and
        // a badge on raster would mark every row, which marks nothing.
        let blank: Vec<&str> = LayerKind::ALL
            .into_iter()
            .filter(|k| k.badge().is_empty())
            .map(LayerKind::as_str)
            .collect();
        assert_eq!(blank, vec!["raster"]);
    }

    #[test]
    fn every_kind_has_its_own_badge_and_name() {
        for (i, kind) in LayerKind::ALL.iter().enumerate() {
            assert!(!kind.label().is_empty(), "{} has no label", kind.as_str());
            for other in &LayerKind::ALL[i + 1..] {
                assert_ne!(kind.label(), other.label());
                if !kind.badge().is_empty() {
                    assert_ne!(
                        kind.badge(),
                        other.badge(),
                        "{} and {} share a badge",
                        kind.as_str(),
                        other.as_str()
                    );
                }
            }
        }
    }

    /// The panel's indent comes from the graph, not from a row's neighbours.
    ///
    /// Rows are flat, so a delegate that tried to infer nesting from the row
    /// above it would indent whatever happened to follow a group — including
    /// the layer that sits *after* the group at the same level.
    #[test]
    fn a_grouped_layer_reports_the_depth_of_its_group() {
        let mut graph = DocumentGraph::new(DocumentSize::new(64, 64));
        let outside = graph
            .add_layer_top(Some("outside".to_owned()))
            .expect("layer");
        let group = graph
            .add_group_top(Some("group".to_owned()))
            .expect("group");
        let inside = graph
            .add_layer_top(Some("inside".to_owned()))
            .expect("layer");
        graph.get_mut(inside).expect("layer").parent = Some(group);

        let rows = layer_rows(&graph, &[]);
        let depth = |n: &str| rows.iter().find(|r| r.name == n).expect("row").depth;
        assert_eq!(depth("group"), 0, "the group itself sits at the root");
        assert_eq!(depth("inside"), 1, "its child is indented one level");
        assert_eq!(
            depth("outside"),
            0,
            "a layer next to the group is not inside it"
        );
        let _ = outside;
    }

    #[test]
    fn nested_groups_indent_cumulatively() {
        let mut graph = DocumentGraph::new(DocumentSize::new(64, 64));
        let outer = graph
            .add_group_top(Some("outer".to_owned()))
            .expect("group");
        let inner = graph
            .add_group_top(Some("inner".to_owned()))
            .expect("group");
        let leaf = graph.add_layer_top(Some("leaf".to_owned())).expect("layer");
        graph.get_mut(inner).expect("group").parent = Some(outer);
        graph.get_mut(leaf).expect("layer").parent = Some(inner);

        let rows = layer_rows(&graph, &[]);
        let depth = |n: &str| rows.iter().find(|r| r.name == n).expect("row").depth;
        assert_eq!(depth("outer"), 0);
        assert_eq!(depth("inner"), 1);
        assert_eq!(depth("leaf"), 2);
    }

    #[test]
    fn a_row_carries_the_badge_its_kind_declares() {
        // The panel reads these as model roles, so a row that computed them
        // some other way would put the panel back to holding its own copy of
        // the vocabulary.
        let mut graph = DocumentGraph::new(DocumentSize::new(64, 64));
        let group = graph.add_group_top(None).expect("group");
        let rows = layer_rows(&graph, &[]);
        let row = rows
            .iter()
            .find(|r| r.stack_index == graph.index_of(group).expect("index") as i32)
            .expect("the group has a row");
        assert_eq!(row.kind, "group");
        assert_eq!(row.kind_badge, LayerKind::Group.badge());
        assert_eq!(row.kind_label, LayerKind::Group.label());
        assert!(
            rows.iter()
                .any(|r| r.kind == "raster" && r.kind_badge.is_empty()),
            "a raster row must carry no badge"
        );
    }
}
