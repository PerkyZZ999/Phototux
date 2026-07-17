//! Effective preference precedence (handbook §24 spine for P9).
//!
//! Layers: document > workspace > user > builtin. Blind merges are forbidden;
//! callers pass only layers that the key declares legal.

use serde::{Deserialize, Serialize};

/// Source that won the resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrefSource {
    Builtin,
    User,
    Workspace,
    Document,
}

impl PrefSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::User => "user",
            Self::Workspace => "workspace",
            Self::Document => "document",
        }
    }
}

/// Resolve with document > workspace > user > builtin precedence.
pub fn resolve_layered<T: Clone>(
    builtin: T,
    user: Option<T>,
    workspace: Option<T>,
    document: Option<T>,
) -> (T, PrefSource) {
    if let Some(v) = document {
        return (v, PrefSource::Document);
    }
    if let Some(v) = workspace {
        return (v, PrefSource::Workspace);
    }
    if let Some(v) = user {
        return (v, PrefSource::User);
    }
    (builtin, PrefSource::Builtin)
}

/// Whether selected layers disagree on a comparable field (mixed inspector).
pub fn values_are_mixed<T: PartialEq>(values: &[T]) -> bool {
    match values {
        [] | [_] => false,
        [first, rest @ ..] => rest.iter().any(|v| v != first),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_beats_user_and_builtin() {
        let (v, src) = resolve_layered(10, Some(20), None, Some(40));
        assert_eq!(v, 40);
        assert_eq!(src, PrefSource::Document);
    }

    #[test]
    fn workspace_beats_user() {
        let (v, src) = resolve_layered(false, Some(true), Some(false), None);
        assert!(!v);
        assert_eq!(src, PrefSource::Workspace);
    }

    #[test]
    fn mixed_detects_disagreement() {
        assert!(!values_are_mixed(&[1.0_f32]));
        assert!(!values_are_mixed(&[0.5_f32, 0.5]));
        assert!(values_are_mixed(&[0.5_f32, 1.0]));
    }
}
