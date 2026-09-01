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
    /// Tool-shelf slot. Tools sharing a slot occupy one button with a flyout.
    ///
    /// Coarser than [`Self::id`] and finer than [`Self::group`]: `group` draws
    /// the separators between families, `slot` decides which tools stack. Both
    /// are needed — the six selection tools are one family drawn as three
    /// slots, the way Photoshop stacks them.
    pub slot: String,
    /// Default accelerator, following the conventional raster-editor letters.
    ///
    /// Here rather than on the action registry because the registry's tool
    /// entries are *generated* from these descriptors: title, icon and key are
    /// three facts about one tool, and keeping them together is what stops the
    /// registry becoming a second list of the tool vocabulary.
    pub shortcut: String,
}

/// Built-in panel set shipped with the desktop shell.
///
/// Every entry here must be a panel the shell actually draws — a test holds the
/// two in agreement. `panel.paths` and `panel.character` used to be declared
/// here and rendered nowhere, so the Window menu offered two toggles that
/// changed the persisted workspace and put nothing on screen. Their content
/// lives as the `inspector.path` and `inspector.text` disclosure groups in
/// Properties; promoting either to a dock of its own is a separate piece of
/// work, and this list is where it would start.
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
    ]
}

/// Built-in tool strip set (`icon_key` = Phosphor stem under `assets/icons/phosphor/`).
pub fn default_tools() -> Vec<ToolDescriptor> {
    use crate::tool_id;
    let tool =
        |id: &str, title: &str, icon: &str, key: &str, group: &str, slot: &str| ToolDescriptor {
            id: id.into(),
            title: title.into(),
            icon_key: icon.into(),
            shortcut: key.into(),
            group: group.into(),
            slot: slot.into(),
        };
    // `group` is the *separator band*, not the tool family: Photoshop rules a
    // line after Move, after the selection tools, after crop/transform/
    // eyedropper, after the whole painting block, and after the vector block.
    // Splitting painting from retouching would draw a line through the middle
    // of that block, where Photoshop draws none.
    //
    // Rail order is Photoshop's, slot for slot, because a tool shelf is
    // muscle memory: someone who reaches for the lasso three buttons down
    // should find the lasso there. The two orders had drifted in one place
    // that mattered — the polygonal lasso sat two slots below the freehand
    // one, with the wand between them, so the pair a user thinks of as one
    // tool were not neighbours and could not share a slot.
    let mut tools = vec![
        tool(
            tool_id::MOVE,
            "Move",
            "arrows-out-cardinal",
            "V",
            "move",
            "move",
        ),
        tool(
            tool_id::SELECT_RECT,
            "Rectangular Marquee",
            "selection",
            "M",
            "select",
            "marquee",
        ),
        tool(
            tool_id::SELECT_ELLIPSE,
            "Elliptical Marquee",
            "circle-dashed",
            "Shift+M",
            "select",
            "marquee",
        ),
        tool(
            tool_id::SELECT_LASSO,
            "Lasso",
            "lasso",
            "L",
            "select",
            "lasso",
        ),
        tool(
            tool_id::SELECT_POLYGON,
            "Polygonal Lasso",
            "polygon",
            "Shift+L",
            "select",
            "lasso",
        ),
        tool(
            tool_id::SELECT_WAND,
            "Magic Wand",
            "magic-wand",
            "W",
            "select",
            "wand",
        ),
        tool(
            tool_id::SELECT_COLOR_RANGE,
            "Color Range",
            "selection-foreground",
            "Shift+W",
            "select",
            "wand",
        ),
        tool(tool_id::CROP, "Crop", "crop", "C", "measure", "crop"),
        tool(
            tool_id::TRANSFORM,
            "Free Transform",
            "arrows-out",
            "Ctrl+T",
            "measure",
            "transform",
        ),
        tool(
            tool_id::EYEDROPPER,
            "Eyedropper",
            "eyedropper",
            "I",
            "measure",
            "eyedropper",
        ),
        tool(
            tool_id::BRUSH,
            "Brush",
            "paint-brush",
            "B",
            "paint",
            "brush",
        ),
    ];
    // The retouch tools are generated from `DabMode` — modes and tools are one
    // list seen from two sides, so a mode cannot arrive with no way to pick it
    // — but they belong at three different points on the rail, next to the
    // painting they rework. Each slot is placed where Photoshop places it.
    tools.extend(retouch_tools("clone"));
    tools.push(tool(
        tool_id::ERASER,
        "Eraser",
        "eraser",
        "E",
        "paint",
        "eraser",
    ));
    tools.push(tool(
        tool_id::GRADIENT,
        "Gradient",
        "gradient",
        "Shift+G",
        "paint",
        "fill",
    ));
    tools.push(tool(
        tool_id::FILL,
        "Paint Bucket",
        "paint-bucket",
        "G",
        "paint",
        "fill",
    ));
    tools.extend(retouch_tools("focus"));
    tools.extend(retouch_tools("tone"));
    tools.extend([
        tool(
            tool_id::PATH_EDIT,
            "Path Edit",
            "pen-nib",
            "A",
            "vector",
            "pen",
        ),
        tool(tool_id::TEXT, "Text", "text-t", "T", "vector", "type"),
        tool(tool_id::SHAPE, "Shape", "shapes", "U", "vector", "shape"),
        tool(tool_id::PAN, "Hand", "hand", "H", "navigate", "hand"),
        tool(
            tool_id::ZOOM,
            "Zoom",
            "magnifying-glass",
            "Z",
            "navigate",
            "zoom",
        ),
    ]);
    tools
}

/// Rail entries for every retouch mode belonging to `slot`, in mode order.
fn retouch_tools(slot: &'static str) -> impl Iterator<Item = ToolDescriptor> {
    crate::DabMode::retouch_modes()
        .filter(move |mode| mode.slot() == slot)
        .map(|mode| ToolDescriptor {
            id: mode.tool_id().into(),
            title: mode.tool_title().into(),
            icon_key: mode.icon_key().into(),
            shortcut: mode.shortcut().into(),
            group: "paint".into(),
            slot: mode.slot().into(),
        })
}

/// The shelf as slots, in rail order, each holding its tools in rail order.
///
/// One button per slot with a flyout for the rest, the way Photoshop stacks a
/// shelf. Twenty-five buttons in a column need about a thousand pixels and ran
/// off the bottom of a 1080p window into an overflow menu — which is a worse
/// place for a tool than a flyout, because nothing about "…" says which tools
/// are behind it.
#[must_use]
pub fn tool_slots() -> Vec<Vec<ToolDescriptor>> {
    let mut slots: Vec<Vec<ToolDescriptor>> = Vec::new();
    for tool in default_tools() {
        match slots.last_mut() {
            Some(last) if last[0].slot == tool.slot => last.push(tool),
            _ => slots.push(vec![tool]),
        }
    }
    slots
}

/// Shelf slots as `[{slot, group, tools:[{id,title,icon,shortcut}]}]` for QML.
#[must_use]
pub fn tool_slots_json() -> String {
    let rows: Vec<serde_json::Value> = tool_slots()
        .into_iter()
        .map(|tools| {
            serde_json::json!({
                "slot": tools[0].slot,
                "group": tools[0].group,
                "tools": tools
                    .iter()
                    .map(|t| serde_json::json!({
                        "id": t.id,
                        "title": t.title,
                        "icon": t.icon_key,
                        "shortcut": t.shortcut,
                    }))
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
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
        group("inspector.styles", "Layer Styles", 3, false),
        // Level 3 and closed: Blend If is powerful and rarely the first thing
        // anyone reaches for, and its eight handles would dominate the panel
        // if they were open by default.
        group("inspector.blend-if", "Blend If", 3, false),
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
    /// Every icon a descriptor names must be packaged into the QML resource.
    ///
    /// The qrc carries a hand-written list of icon stems, which CMake cannot
    /// derive from these descriptors — so a tool or action naming an icon
    /// nobody added to that list ships a blank button, silently. This reads
    /// the list from the other side.
    #[test]
    fn every_icon_key_is_packaged_into_the_qrc() {
        let cmake = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../phototux/qml-aot/CMakeLists.txt"
        ))
        .expect("qml-aot/CMakeLists.txt is readable from the engine crate");
        let packaged: Vec<&str> = cmake
            .split("set(ICON_NAMES")
            .nth(1)
            .expect("ICON_NAMES list")
            .split(')')
            .next()
            .expect("ICON_NAMES terminator")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();
        assert!(
            packaged.len() > 20,
            "the ICON_NAMES parse found {} entries — the list moved, rather than shrank",
            packaged.len()
        );

        for tool in default_tools() {
            assert!(
                packaged.contains(&tool.icon_key.as_str()),
                "{} names icon {:?}, which the qrc does not carry — the button ships blank",
                tool.id,
                tool.icon_key
            );
        }
        for kind in crate::GradientKind::ALL {
            assert!(
                packaged.contains(&kind.icon_key()),
                "gradient {:?} names icon {:?}, which the qrc does not carry",
                kind,
                kind.icon_key()
            );
        }
        for action in crate::default_actions() {
            let Some(icon) = action.icon_key.as_deref() else {
                continue;
            };
            assert!(
                packaged.contains(&icon),
                "{} names icon {icon:?}, which the qrc does not carry",
                action.id
            );
        }

        // Icons the shell names directly, which no descriptor mentions. The
        // sweep above only covers stems the *engine* declares, so a glyph
        // written into QML by hand — a panel placeholder, a dialog badge —
        // shipped blank with nothing to say so.
        for (file, stems) in qml_icon_literals() {
            for stem in stems {
                assert!(
                    packaged.contains(&stem.as_str()),
                    "{file} names icon {stem:?}, which the qrc does not carry"
                );
            }
        }
    }

    /// Icon stems written as literals in `qml/`, by file.
    ///
    /// Matches `iconKey: "stem"` and `iconUrl("stem")` — the two forms the
    /// shell uses to name a glyph that comes from nowhere else. Interpolated
    /// stems (`root.toolIconStem(...)`) resolve from descriptors and are
    /// already covered above.
    fn qml_icon_literals() -> Vec<(String, Vec<String>)> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../qml");
        let mut out = Vec::new();
        let entries = std::fs::read_dir(dir).expect("qml/ is readable from the engine crate");
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "qml") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("qml file is readable");
            let mut stems = Vec::new();
            for (marker, closer) in [("iconKey: \"", '"'), ("iconUrl(\"", '"')] {
                for chunk in text.split(marker).skip(1) {
                    if let Some(stem) = chunk.split(closer).next()
                        && !stem.is_empty()
                        && stem
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit())
                    {
                        stems.push(stem.to_owned());
                    }
                }
            }
            if !stems.is_empty() {
                stems.sort_unstable();
                stems.dedup();
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                out.push((name, stems));
            }
        }
        assert!(
            !out.is_empty(),
            "no icon literals found in qml/ — the scan broke rather than the shell"
        );
        out
    }

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

    /// The terminal key of a chord: `Ctrl+Shift+R` → `R`.
    fn terminal_key(shortcut: &str) -> &str {
        shortcut.rsplit('+').next().unwrap_or(shortcut)
    }

    #[test]
    fn tools_sharing_a_slot_share_an_accelerator_letter() {
        // The two say the same thing from different sides — Photoshop stacks
        // the tools that share a letter — so they are the kind of pair that
        // drifts silently. A slot grouping tools with different letters means
        // one of the two is wrong, and neither is obviously the wrong one.
        for slot in tool_slots() {
            let keys: Vec<&str> = slot
                .iter()
                .map(|t| terminal_key(t.shortcut.as_str()))
                .collect();
            assert!(
                keys.windows(2).all(|w| w[0] == w[1]),
                "slot {} stacks tools with different accelerators: {keys:?}",
                slot[0].slot
            );
        }
    }

    #[test]
    fn every_slot_is_contiguous_and_named() {
        // `tool_slots` folds *adjacent* tools, so a slot whose members are not
        // neighbours would silently split into two buttons showing the same
        // name — which is what the polygonal lasso did before the rail was
        // reordered.
        let mut seen: Vec<String> = Vec::new();
        for slot in tool_slots() {
            let key = slot[0].slot.clone();
            assert!(!key.is_empty(), "a tool has no slot");
            assert!(
                !seen.contains(&key),
                "slot {key} appears twice — its tools are not neighbours"
            );
            assert!(slot.iter().all(|t| t.slot == key));
            seen.push(key);
        }
        assert!(seen.len() < default_tools().len(), "no tool stacks at all");
    }

    #[test]
    fn every_retouch_mode_reaches_the_shelf_exactly_once() {
        // The rail places the retouch slots at three separate points now, so
        // "generated from the vocabulary" is no longer one splice — a mode
        // whose slot matched nothing would vanish from the shelf entirely.
        let ids: Vec<&str> = default_tools()
            .iter()
            .map(|t| t.id.clone())
            .collect::<Vec<_>>()
            .leak()
            .iter()
            .map(|s| s.as_str())
            .collect();
        for mode in crate::DabMode::retouch_modes() {
            let hits = ids.iter().filter(|id| **id == mode.tool_id()).count();
            assert_eq!(
                hits,
                1,
                "{} appears {hits} times on the shelf",
                mode.tool_id()
            );
        }
    }

    #[test]
    fn the_shelf_fits_a_maximized_1080p_window() {
        // The reason slots exist at all. Budget is what a maximized 1080p
        // window leaves beside the canvas once the chrome bands are taken off:
        // title, menu, toolbar, tool options, document tabs and status bar.
        // Flat, twenty-five tools needed a thousand pixels of column and the
        // tail fell into an overflow menu — a worse home for a tool than a
        // flyout, since nothing about "…" says which tools are behind it.
        const HIT: usize = 40; // Theme.toolHit at density 1.0
        const CHROME: usize = 30 + 26 + 40 + 40 + 28 + 28;
        let budget = 1080 - CHROME;
        let slots = tool_slots().len();
        let needed = slots * HIT;
        assert!(
            needed <= budget,
            "{slots} slots need {needed}px but the shelf has {budget}px"
        );
        // And it must actually be doing something: a shelf where every tool
        // got its own slot would pass the line above only by luck.
        assert!(
            slots < default_tools().len() - 4,
            "{slots} slots for {} tools — barely anything stacks",
            default_tools().len()
        );
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
