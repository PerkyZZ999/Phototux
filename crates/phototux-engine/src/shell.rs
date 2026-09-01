//! Shell descriptors for panels and tools (handbook Phase 3 — Qt presents these).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Dock / panel contribution descriptor (toolkit-neutral).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelDescriptor {
    pub id: String,
    pub title: String,
    /// Default region: `right`, `left`, `bottom`.
    pub default_region: String,
    pub visible_by_default: bool,
}

/// Inspector disclosure group descriptor (handbook 01 / 28 progressive disclosure).
///
/// Context decides whether a group is *present*; disclosure decides how much of
/// it is *shown*. The two are independent: a brush group hidden because the
/// eraser is active is not the same as a brush group the user collapsed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisclosureGroupDescriptor {
    pub id: String,
    pub title: String,
    /// Handbook disclosure level: 2 nearby, 3 on demand, 4 specialized.
    ///
    /// Level 1 (immediate) content is never collapsible and therefore never
    /// appears here.
    pub level: u8,
    pub open_by_default: bool,
}

/// Tool strip entry descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub id: String,
    pub title: String,
    pub icon_key: String,
    pub group: String,
}

/// Built-in panel set shipped with the desktop shell.
pub fn default_panels() -> Vec<PanelDescriptor> {
    vec![
        PanelDescriptor {
            id: "panel.properties".into(),
            title: "Properties".into(),
            default_region: "right".into(),
            visible_by_default: true,
        },
        PanelDescriptor {
            id: "panel.navigator".into(),
            title: "Navigator".into(),
            default_region: "right".into(),
            visible_by_default: true,
        },
        PanelDescriptor {
            id: "panel.swatches".into(),
            title: "Swatches".into(),
            default_region: "right".into(),
            visible_by_default: true,
        },
        PanelDescriptor {
            id: "panel.layers".into(),
            title: "Layers".into(),
            default_region: "right".into(),
            visible_by_default: true,
        },
        PanelDescriptor {
            id: "panel.history".into(),
            title: "History".into(),
            default_region: "right".into(),
            visible_by_default: true,
        },
        PanelDescriptor {
            id: "panel.paths".into(),
            title: "Paths".into(),
            default_region: "right".into(),
            visible_by_default: false,
        },
        PanelDescriptor {
            id: "panel.character".into(),
            title: "Character".into(),
            default_region: "right".into(),
            visible_by_default: false,
        },
    ]
}

/// Built-in tool strip set (`icon_key` = Phosphor stem under `assets/icons/phosphor/`).
pub fn default_tools() -> Vec<ToolDescriptor> {
    use crate::tool_id;
    vec![
        ToolDescriptor {
            id: tool_id::MOVE.into(),
            title: "Move".into(),
            icon_key: "arrows-out-cardinal".into(),
            group: "move".into(),
        },
        ToolDescriptor {
            id: tool_id::SELECT_RECT.into(),
            title: "Rectangular Marquee".into(),
            icon_key: "selection".into(),
            group: "select".into(),
        },
        ToolDescriptor {
            id: tool_id::SELECT_ELLIPSE.into(),
            title: "Elliptical Marquee".into(),
            icon_key: "circle-dashed".into(),
            group: "select".into(),
        },
        ToolDescriptor {
            id: tool_id::SELECT_LASSO.into(),
            title: "Lasso".into(),
            icon_key: "lasso".into(),
            group: "select".into(),
        },
        ToolDescriptor {
            id: tool_id::SELECT_POLYGON.into(),
            title: "Polygonal Lasso".into(),
            icon_key: "polygon".into(),
            group: "select".into(),
        },
        ToolDescriptor {
            id: tool_id::CROP.into(),
            title: "Crop".into(),
            icon_key: "crop".into(),
            group: "transform".into(),
        },
        ToolDescriptor {
            id: tool_id::TRANSFORM.into(),
            title: "Free Transform".into(),
            icon_key: "arrows-out".into(),
            group: "transform".into(),
        },
        ToolDescriptor {
            id: tool_id::EYEDROPPER.into(),
            title: "Eyedropper".into(),
            icon_key: "eyedropper".into(),
            group: "sample".into(),
        },
        ToolDescriptor {
            id: tool_id::BRUSH.into(),
            title: "Brush".into(),
            icon_key: "paint-brush".into(),
            group: "paint".into(),
        },
        ToolDescriptor {
            id: tool_id::ERASER.into(),
            title: "Eraser".into(),
            icon_key: "eraser".into(),
            group: "paint".into(),
        },
        ToolDescriptor {
            id: tool_id::GRADIENT.into(),
            title: "Gradient".into(),
            icon_key: "gradient".into(),
            group: "paint".into(),
        },
        ToolDescriptor {
            id: tool_id::FILL.into(),
            title: "Paint Bucket".into(),
            icon_key: "paint-bucket".into(),
            group: "paint".into(),
        },
        ToolDescriptor {
            id: tool_id::TEXT.into(),
            title: "Text".into(),
            icon_key: "text-t".into(),
            group: "type".into(),
        },
        ToolDescriptor {
            id: tool_id::PATH_EDIT.into(),
            title: "Path Edit".into(),
            icon_key: "pen-nib".into(),
            group: "vector".into(),
        },
        ToolDescriptor {
            id: tool_id::SHAPE.into(),
            title: "Shape".into(),
            icon_key: "shapes".into(),
            group: "vector".into(),
        },
        ToolDescriptor {
            id: tool_id::PAN.into(),
            title: "Hand".into(),
            icon_key: "hand".into(),
            group: "navigate".into(),
        },
        ToolDescriptor {
            id: tool_id::ZOOM.into(),
            title: "Zoom".into(),
            icon_key: "magnifying-glass".into(),
            group: "navigate".into(),
        },
    ]
}

/// Built-in inspector disclosure groups, ordered as they appear in Properties.
///
/// Titles name a coherent concept rather than "More" or "Advanced" so the
/// collapsed header still carries information scent (handbook 01/28).
pub fn default_disclosure_groups() -> Vec<DisclosureGroupDescriptor> {
    let group =
        |id: &str, title: &str, level: u8, open_by_default: bool| DisclosureGroupDescriptor {
            id: id.into(),
            title: title.into(),
            level,
            open_by_default,
        };
    vec![
        group("inspector.selection", "Selection", 2, true),
        group("inspector.brush", "Brush", 2, true),
        group("inspector.fill", "Fill", 2, true),
        group("inspector.text", "Character", 2, true),
        group("inspector.path", "Path", 2, true),
        group("inspector.transform", "Transform and Crop", 2, true),
        group("inspector.adjustment", "Adjustment", 2, true),
        group("inspector.effects", "Effects", 3, false),
        group("inspector.color", "Color Management", 3, false),
        group("inspector.diagnostics", "Diagnostics", 4, false),
    ]
}

pub fn disclosure_groups_json() -> String {
    serde_json::to_string(&default_disclosure_groups()).unwrap_or_else(|_| "[]".into())
}

/// Editable bounds of one inspector parameter, in `AdjustmentParams` units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdjustmentParamRange {
    /// Position in the kind's slot projection.
    pub index: usize,
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
}

/// Inspector editing ranges per adjustment kind.
///
/// These are the bounds the Properties editor can *represent*. They are
/// deliberately narrower than [`crate::AdjustmentParams::clamped`], which
/// defines what the engine accepts: a document may legally carry a gamma of 6.0
/// that the gamma slider cannot reach. Sliders and the out-of-range badge read
/// the same table so the two cannot disagree about what is showable.
pub fn adjustment_editor_ranges() -> Vec<(&'static str, Vec<AdjustmentParamRange>)> {
    crate::AdjustmentParams::ALL_KINDS
        .iter()
        .map(|params| {
            let slots = params
                .editor_slots()
                .iter()
                .enumerate()
                .map(|(index, &(label, min, max))| AdjustmentParamRange {
                    index,
                    label,
                    min,
                    max,
                })
                .collect();
            (params.kind_key(), slots)
        })
        .collect()
}

/// `{kind: [{slot, label, min, max}]}` for the QML adjustment editor.
///
/// An ordered list rather than a slot→bounds map, because the chrome builds
/// the editor from this: it needs the label and the order as much as the
/// bounds. The panel used to hand-write a slider pair per kind, which is why
/// four adjustment kinds had no editor at all.
#[must_use]
pub fn adjustment_editor_ranges_json() -> String {
    let map: BTreeMap<&str, Vec<serde_json::Value>> = adjustment_editor_ranges()
        .into_iter()
        .map(|(kind, params)| {
            let slots = params
                .into_iter()
                .map(|p| {
                    serde_json::json!({
                        "label": p.label,
                        "min": p.min,
                        "max": p.max,
                    })
                })
                .collect();
            (kind, slots)
        })
        .collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".into())
}

/// Display label per adjustment kind, for the editor heading.
#[must_use]
pub fn adjustment_labels_json() -> String {
    let map: BTreeMap<&str, &str> = crate::AdjustmentParams::ALL_KINDS
        .iter()
        .map(|p| (p.kind_key(), p.label()))
        .collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".into())
}

/// A warning or invalid value a collapsed inspector group would otherwise hide.
///
/// Handbook 28 requires these to reach the group header without the body
/// existing, so they are derived from host state rather than from widgets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisclosureBadge {
    pub text: String,
    /// `warning` | `error`.
    pub severity: &'static str,
}

/// Host state the badge rules read.
///
/// Plain data rather than a session borrow, so the rules stay pure and each one
/// is testable on its own.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InspectorState<'a> {
    pub adjustment_kind: &'a str,
    /// Slot values of the active adjustment, index-aligned with the kind's
    /// [`crate::AdjustmentParams::editor_slots`].
    pub adjustment_slots: &'a [f32],
    pub selection_active: bool,
    /// Outline bounds as `(x, y, width, height)` in document space.
    pub selection_bounds: Option<(i32, i32, u32, u32)>,
    pub document_size: (u32, u32),
    pub text_layer_active: bool,
    pub text_font_family: &'a str,
    /// `None` until fontconfig discovery has run. A family cannot be called
    /// missing from a list that is not yet authoritative, so the rule stays
    /// silent rather than guessing.
    pub known_font_families: Option<&'a [String]>,
    pub gpu_lost: bool,
}

/// Inspector group id → badge, for every group currently hiding something.
///
/// Groups absent from the map have nothing to surface. Strings are English;
/// presentations translate them like other descriptor labels.
pub fn inspector_badges(state: &InspectorState<'_>) -> BTreeMap<String, DisclosureBadge> {
    let mut badges = BTreeMap::new();
    if let Some(text) = adjustment_out_of_range(state) {
        badges.insert(
            "inspector.adjustment".to_owned(),
            DisclosureBadge {
                text,
                severity: "warning",
            },
        );
    }
    if selection_misses_canvas(state) {
        badges.insert(
            "inspector.selection".to_owned(),
            DisclosureBadge {
                text: "Outside canvas".to_owned(),
                severity: "warning",
            },
        );
    }
    if font_family_missing(state) {
        badges.insert(
            "inspector.text".to_owned(),
            DisclosureBadge {
                text: "Font not installed".to_owned(),
                severity: "warning",
            },
        );
    }
    if state.gpu_lost {
        badges.insert(
            "inspector.diagnostics".to_owned(),
            DisclosureBadge {
                text: "GPU lost".to_owned(),
                severity: "error",
            },
        );
    }
    badges
}

pub fn inspector_badges_json(state: &InspectorState<'_>) -> String {
    serde_json::to_string(&inspector_badges(state)).unwrap_or_else(|_| "{}".into())
}

/// Name the first parameter the editor cannot represent, if any.
///
/// The tolerance is a thousandth of each parameter's own span. It has to
/// exceed the separation `clamped` inserts between Levels black and white,
/// or dragging Black to its maximum would flag White as out of range.
fn adjustment_out_of_range(state: &InspectorState<'_>) -> Option<String> {
    let params = adjustment_editor_ranges()
        .into_iter()
        .find(|(kind, _)| *kind == state.adjustment_kind)?
        .1;
    params
        .into_iter()
        .find(|p| {
            let Some(&value) = state.adjustment_slots.get(p.index) else {
                return false;
            };
            let slack = (p.max - p.min) * 1e-3;
            value < p.min - slack || value > p.max + slack
        })
        .map(|p| format!("{} out of range", p.label))
}

/// An active selection whose outline shares no pixel with the canvas edits
/// nothing, and says so nowhere else in the shell.
fn selection_misses_canvas(state: &InspectorState<'_>) -> bool {
    if !state.selection_active {
        return false;
    }
    let (doc_w, doc_h) = state.document_size;
    if doc_w == 0 || doc_h == 0 {
        return false;
    }
    let Some((x, y, w, h)) = state.selection_bounds else {
        return true;
    };
    if w == 0 || h == 0 {
        return true;
    }
    let x1 = i64::from(x) + i64::from(w);
    let y1 = i64::from(y) + i64::from(h);
    x1 <= 0 || y1 <= 0 || i64::from(x) >= i64::from(doc_w) || i64::from(y) >= i64::from(doc_h)
}

/// A text layer asking for a family fontconfig does not know renders in a
/// substitute, which the collapsed Character group would hide entirely.
fn font_family_missing(state: &InspectorState<'_>) -> bool {
    if !state.text_layer_active || state.text_font_family.is_empty() {
        return false;
    }
    let Some(known) = state.known_font_families else {
        return false;
    };
    !known.iter().any(|f| f == state.text_font_family)
}

/// Essentials workspace panel visibility map (panel id → visible).
pub fn essentials_panel_visibility() -> Vec<(String, bool)> {
    default_panels()
        .into_iter()
        .map(|p| (p.id, p.visible_by_default))
        .collect()
}

/// JSON for QML consumption.
pub fn panels_json() -> String {
    serde_json::to_string(&default_panels()).unwrap_or_else(|_| "[]".into())
}

pub fn tools_json() -> String {
    serde_json::to_string(&default_tools()).unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_serialize() {
        let panels = panels_json();
        assert!(panels.contains("panel.layers"));
        let tools = tools_json();
        assert!(tools.contains("tool.brush"));
        let groups = disclosure_groups_json();
        assert!(groups.contains("inspector.brush"));
    }

    /// The shelf draws a separator wherever the group changes between
    /// neighbours, so a family split across the list produces a separator in
    /// the middle of it. Contiguity is what makes the bands read as families.
    #[test]
    fn tool_groups_are_contiguous() {
        let tools = default_tools();
        let mut seen: Vec<&str> = Vec::new();
        let mut previous = "";
        for tool in &tools {
            if tool.group != previous {
                assert!(
                    !seen.contains(&tool.group.as_str()),
                    "group {} resumes after {} — the shelf would separate it mid-family",
                    tool.group,
                    previous
                );
                seen.push(tool.group.as_str());
                previous = tool.group.as_str();
            }
        }
        assert!(tools.iter().all(|t| !t.group.is_empty()));
    }

    /// Pointer and selection lead, navigation trails. Users reach for these
    /// positions before they read the icons.
    #[test]
    fn tool_shelf_opens_with_pointer_and_ends_with_navigation() {
        let tools = default_tools();
        let ids: Vec<&str> = tools.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids.first().copied(), Some(crate::tool_id::MOVE));
        assert_eq!(ids.get(1).copied(), Some(crate::tool_id::SELECT_RECT));
        assert_eq!(ids.last().copied(), Some(crate::tool_id::ZOOM));
        let paint = ids.iter().position(|id| *id == crate::tool_id::BRUSH);
        let select = ids.iter().position(|id| *id == crate::tool_id::SELECT_RECT);
        assert!(select < paint, "selection must precede the paint family");
    }

    /// The rail and the vocabulary are separate lists that must describe the
    /// same set of tools. Each direction fails differently and neither is
    /// visible from the other file, so both are asserted here.
    #[test]
    fn the_tool_rail_and_the_tool_vocabulary_describe_the_same_tools() {
        let tools = default_tools();
        let ids: Vec<&str> = tools.iter().map(|t| t.id.as_str()).collect();
        for id in crate::tool_id::ALL {
            assert!(
                ids.contains(id),
                "{id} is a known tool with no rail entry — nothing can select it"
            );
        }
        for id in &ids {
            assert!(
                crate::tool_id::is_known(id),
                "the rail offers {id}, which the host does not recognise and would replace with the brush"
            );
        }
    }

    #[test]
    fn disclosure_group_ids_are_unique_and_levelled() {
        let groups = default_disclosure_groups();
        let mut ids: Vec<&str> = groups.iter().map(|g| g.id.as_str()).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "disclosure group ids must be unique");
        // Level 1 is immediate content and is never collapsible.
        assert!(groups.iter().all(|g| (2..=4).contains(&g.level)));
    }

    #[test]
    fn specialized_groups_start_collapsed() {
        for group in default_disclosure_groups() {
            if group.level >= 3 {
                assert!(
                    !group.open_by_default,
                    "{} is level {} and must start collapsed",
                    group.id, group.level
                );
            }
        }
    }

    /// Every badge must name a registered group, or it can never be shown.
    #[test]
    fn badge_ids_are_registered_groups() {
        let registered: Vec<String> = default_disclosure_groups()
            .into_iter()
            .map(|g| g.id)
            .collect();
        let state = InspectorState {
            adjustment_kind: "levels",
            adjustment_slots: &[0.0, 1.0, 6.0],
            selection_active: true,
            selection_bounds: Some((-500, 0, 100, 100)),
            document_size: (256, 256),
            text_layer_active: true,
            text_font_family: "Nonexistent Sans",
            known_font_families: Some(&[]),
            gpu_lost: true,
            // No `..default()`: naming every field is what makes adding one
            // without a rule — or a rule without a fixture — a compile error
            // here rather than a badge nobody notices is missing.
        };
        let badges = inspector_badges(&state);
        assert_eq!(badges.len(), 4, "expected every rule to fire: {badges:?}");
        for id in badges.keys() {
            assert!(registered.contains(id), "{id} is not a registered group");
        }
    }

    /// Dragging any slider to either extreme must never raise a badge: the
    /// badge means "the document holds a value this editor cannot reach", so
    /// the editor's own output has to survive the engine's clamping unflagged.
    #[test]
    fn slider_extremes_never_raise_a_badge() {
        for (kind, params) in adjustment_editor_ranges() {
            for p in &params {
                assert!(p.min < p.max, "{kind} slot {} has an empty range", p.index);
                for value in [p.min, p.max] {
                    let state = project(kind, build_adjustment(kind, p.index, value).clamped());
                    assert_eq!(
                        adjustment_out_of_range(&state),
                        None,
                        "{kind} slot {} at {value} raises a badge after clamping",
                        p.index
                    );
                }
            }
        }
    }

    /// An adjustment holding `value` in `slot`, other slots at their defaults.
    ///
    /// Built through the vocabulary rather than restated: this helper used to
    /// carry its own `match` with a fallback arm, so sweeping the sliders of a
    /// kind it did not name silently swept Brightness/Contrast instead and
    /// flagged the wrong badge.
    fn build_adjustment(kind: &str, index: usize, value: f32) -> crate::AdjustmentParams {
        let base = crate::AdjustmentParams::default_for_kind(kind)
            .unwrap_or_else(|| panic!("{kind} is not a known adjustment kind"));
        let mut slots = base.slots();
        slots[index] = value;
        base.with_slots(slots)
    }

    /// Mirror the host's `p0..p2` projection of an adjustment.
    fn project(kind: &'static str, params: crate::AdjustmentParams) -> InspectorState<'static> {
        // Leaked so the borrow outlives the helper; the suite is short-lived
        // and this keeps `InspectorState` borrowing rather than owning.
        let slots: &'static [f32] = Box::leak(Box::new(params.slots()));
        InspectorState {
            adjustment_kind: kind,
            adjustment_slots: slots,
            ..InspectorState::default()
        }
    }

    #[test]
    fn in_range_adjustment_raises_no_badge() {
        let state = InspectorState {
            adjustment_kind: "levels",
            adjustment_slots: &[0.0, 1.0, 3.0],
            ..InspectorState::default()
        };
        assert!(adjustment_out_of_range(&state).is_none());
    }

    #[test]
    fn out_of_range_badge_names_the_parameter() {
        let state = InspectorState {
            adjustment_kind: "levels",
            adjustment_slots: &[0.0, 1.0, 6.0],
            ..InspectorState::default()
        };
        assert_eq!(
            adjustment_out_of_range(&state).as_deref(),
            Some("Gamma out of range")
        );
    }

    #[test]
    fn selection_overlapping_the_canvas_is_not_flagged() {
        let inside = InspectorState {
            selection_active: true,
            selection_bounds: Some((-10, -10, 40, 40)),
            document_size: (256, 256),
            ..InspectorState::default()
        };
        assert!(!selection_misses_canvas(&inside));

        let outside = InspectorState {
            selection_bounds: Some((256, 0, 40, 40)),
            ..inside.clone()
        };
        assert!(selection_misses_canvas(&outside));

        let inactive = InspectorState {
            selection_active: false,
            ..outside
        };
        assert!(!selection_misses_canvas(&inactive));
    }

    #[test]
    fn font_badge_stays_silent_until_the_list_is_authoritative() {
        let undiscovered = InspectorState {
            text_layer_active: true,
            text_font_family: "Nonexistent Sans",
            known_font_families: None,
            ..InspectorState::default()
        };
        assert!(!font_family_missing(&undiscovered));

        let known = ["Noto Sans".to_owned()];
        let discovered = InspectorState {
            known_font_families: Some(&known),
            ..undiscovered
        };
        assert!(font_family_missing(&discovered));

        let installed = InspectorState {
            text_font_family: "Noto Sans",
            ..discovered
        };
        assert!(!font_family_missing(&installed));
    }
}
