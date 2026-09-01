//! What the Properties panel is looking at (handbook 01, 28).
//!
//! Photoshop's Properties panel is contextual: it shows the settings for
//! whatever is selected, and nothing else. PhotoTux's showed everything at
//! once — a brush size beside a text frame beside a soft-proof profile —
//! because each section carried its own visibility condition written in QML,
//! and several of those conditions compared [`LayerKind::as_str`] against a
//! string literal. That is the layer vocabulary written a second time in a
//! language with no way to check it: renaming a kind, or adding one, changes
//! nothing in QML except which sections silently stop appearing.
//!
//! A *subject* is the answer to "what is the panel describing" — a document,
//! or a layer of some kind. Every section declares the subjects it belongs to,
//! here, once. The panel resolves visibility by asking this table instead of
//! naming kinds, and `properties_panel_draws_every_declared_section` holds the
//! two in agreement by reading the QML as text.
//!
//! Deliberately *not* here: the tool. Tool settings live in the options bar,
//! which is where Photoshop keeps them and where PhotoTux's already are; a
//! section that appears because the brush is selected is not describing the
//! selection and does not belong in this table.

use crate::layer::LayerKind;

/// The subject the Properties panel is describing.
///
/// One more variant than [`LayerKind`] has: the document itself, which is what
/// the panel falls back to when there is no layer worth describing, and what
/// the user gets by asking for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InspectorSubject {
    /// The document: canvas, guides, colour management, diagnostics.
    Document,
    Raster,
    Group,
    Text,
    Adjustment,
    Shape,
    Fill,
    SmartObject,
}

impl InspectorSubject {
    /// Every subject, so a consumer can be checked for covering all of them.
    pub const ALL: [Self; 8] = [
        Self::Document,
        Self::Raster,
        Self::Group,
        Self::Text,
        Self::Adjustment,
        Self::Shape,
        Self::Fill,
        Self::SmartObject,
    ];

    /// Every subject that is a layer — the ones a layer-scoped section shares.
    ///
    /// Named rather than spelled out at each call site so that adding a
    /// [`LayerKind`] cannot leave a section quietly applying to six of seven
    /// kinds; [`Self::layer_subjects_cover_every_kind`] holds it to that.
    pub const LAYERS: &'static [Self] = &[
        Self::Raster,
        Self::Group,
        Self::Text,
        Self::Adjustment,
        Self::Shape,
        Self::Fill,
        Self::SmartObject,
    ];

    /// The subject a layer of `kind` presents.
    #[must_use]
    pub fn from_kind(kind: LayerKind) -> Self {
        match kind {
            LayerKind::Raster => Self::Raster,
            LayerKind::Group => Self::Group,
            LayerKind::Text => Self::Text,
            LayerKind::Adjustment => Self::Adjustment,
            LayerKind::Shape => Self::Shape,
            LayerKind::Fill => Self::Fill,
            LayerKind::SmartObject => Self::SmartObject,
        }
    }

    /// The layer kind this subject describes, or `None` for the document.
    #[must_use]
    pub fn kind(self) -> Option<LayerKind> {
        match self {
            Self::Document => None,
            Self::Raster => Some(LayerKind::Raster),
            Self::Group => Some(LayerKind::Group),
            Self::Text => Some(LayerKind::Text),
            Self::Adjustment => Some(LayerKind::Adjustment),
            Self::Shape => Some(LayerKind::Shape),
            Self::Fill => Some(LayerKind::Fill),
            Self::SmartObject => Some(LayerKind::SmartObject),
        }
    }

    /// Toolkit-neutral id, matching [`LayerKind::as_str`] for layer subjects.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self.kind() {
            Some(kind) => kind.as_str(),
            None => "document",
        }
    }

    /// Parse an id from [`Self::as_str`]. No fallback: an unknown id is a
    /// spelling mistake in the shell, and defaulting it to a real subject
    /// would show the wrong panel rather than say so.
    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.as_str() == id)
    }

    /// Display name for the panel header.
    ///
    /// Delegates to [`LayerKind::label`] rather than carrying its own words:
    /// the layers panel and the inspector header naming the same layer two
    /// different things is the drift this module exists to stop.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self.kind() {
            Some(kind) => kind.label(),
            None => "Document",
        }
    }

    /// Phosphor stem for the header, from [`LayerKind::icon_key`].
    #[must_use]
    pub fn icon_key(self) -> &'static str {
        match self.kind() {
            Some(kind) => kind.icon_key(),
            None => "frame-corners",
        }
    }
}

/// Every subject as `[{"id", "title", "icon"}]`.
///
/// The panel resolves the subject it is *showing* through this, which is not
/// always the subject the selection reports: a user reading the document scope
/// with a raster layer active must see the document's name and glyph, not the
/// layer's. Publishing the live subject's title and icon as separate values
/// could not express that, and the document tab wore the raster layer's icon.
#[must_use]
pub fn subjects_json() -> String {
    let rows: Vec<_> = InspectorSubject::ALL
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "id": s.as_str(),
                "title": s.title(),
                "icon": s.icon_key(),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_layer_kind_has_a_subject_and_comes_back_unchanged() {
        for kind in LayerKind::ALL {
            let subject = InspectorSubject::from_kind(kind);
            assert_eq!(subject.kind(), Some(kind), "{kind:?} round trip");
            assert_eq!(subject.as_str(), kind.as_str(), "{kind:?} id");
        }
    }

    #[test]
    fn ids_are_unique_and_parse_back() {
        let mut seen = Vec::new();
        for subject in InspectorSubject::ALL {
            let id = subject.as_str();
            assert!(!seen.contains(&id), "{id} is declared twice");
            seen.push(id);
            assert_eq!(InspectorSubject::parse(id), Some(subject));
        }
        assert_eq!(InspectorSubject::parse("linked-file"), None);
    }

    /// The layer set must be every subject except the document, or a section
    /// declared for `LAYERS` would quietly not apply to some layer a user can
    /// make.
    #[test]
    fn layer_subjects_cover_every_kind() {
        assert_eq!(InspectorSubject::LAYERS.len(), LayerKind::ALL.len());
        for kind in LayerKind::ALL {
            assert!(
                InspectorSubject::LAYERS.contains(&InspectorSubject::from_kind(kind)),
                "{kind:?} is not in LAYERS"
            );
        }
        assert!(!InspectorSubject::LAYERS.contains(&InspectorSubject::Document));
    }

    #[test]
    fn the_table_carries_every_subject() {
        let json = subjects_json();
        for subject in InspectorSubject::ALL {
            assert!(
                json.contains(&format!("\"id\":\"{}\"", subject.as_str())),
                "{subject:?} is missing from {json}"
            );
        }
    }

    #[test]
    fn every_subject_names_itself_for_a_reader() {
        for subject in InspectorSubject::ALL {
            assert!(!subject.title().is_empty(), "{subject:?} has no title");
            assert!(!subject.icon_key().is_empty(), "{subject:?} has no icon");
        }
    }
}
