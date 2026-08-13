//! Requests the engine hands to the host shell (handbook 08 / DR-003).
//!
//! Some actions cannot complete in the engine or in `AppSession`: opening a
//! file chooser, raising the About dialog, quitting. They need the toolkit.
//! The vocabulary for asking lived as ten `"host:…"` string literals written
//! into the status-bar text and prefix-matched in QML — a contract duplicated
//! across two languages with nothing checking the halves agreed, where a typo
//! was a silently dead menu item.
//!
//! Naming them here makes the vocabulary one list, and the mapping from command
//! op to request a pure function the engine test suite can cover.

/// A shell capability the host must provide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostRequest {
    /// Begin the new-document flow (may prompt about unsaved work first).
    NewDocument,
    /// Open the file chooser.
    OpenDocument,
    /// Open the save-as chooser.
    SaveDocumentAs,
    /// Open the export chooser.
    ExportDocument,
    /// Close the current document (may prompt about unsaved work first).
    CloseDocument,
    /// Quit the application (may prompt about unsaved work first).
    Quit,
    /// Show the About dialog.
    ShowAbout,
    /// Open the ICC-profile chooser.
    EmbedIccProfile,
    /// Raise the command palette.
    OpenCommandPalette,
}

impl HostRequest {
    /// Stable wire name, shared with the QML side.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NewDocument => "document.new",
            Self::OpenDocument => "document.open",
            Self::SaveDocumentAs => "document.save_as",
            Self::ExportDocument => "document.export",
            Self::CloseDocument => "document.close",
            Self::Quit => "app.quit",
            Self::ShowAbout => "help.about",
            Self::EmbedIccProfile => "document.embed_icc",
            Self::OpenCommandPalette => "palette.open",
        }
    }

    /// Parse a wire name back into a request.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.as_str() == name)
    }

    /// Every request, so the host can be checked for full coverage.
    pub const ALL: [HostRequest; 9] = [
        Self::NewDocument,
        Self::OpenDocument,
        Self::SaveDocumentAs,
        Self::ExportDocument,
        Self::CloseDocument,
        Self::Quit,
        Self::ShowAbout,
        Self::EmbedIccProfile,
        Self::OpenCommandPalette,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_request_round_trips_through_its_wire_name() {
        for request in HostRequest::ALL {
            assert_eq!(
                HostRequest::parse(request.as_str()),
                Some(request),
                "{request:?} did not round-trip"
            );
        }
    }

    #[test]
    fn wire_names_are_distinct() {
        let mut names: Vec<&str> = HostRequest::ALL.iter().map(|r| r.as_str()).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "two requests share a wire name");
    }

    #[test]
    fn unknown_names_are_rejected() {
        assert_eq!(HostRequest::parse("document.nope"), None);
        assert_eq!(HostRequest::parse(""), None);
        // The old channel prefixed everything with `host:`; that is not a name.
        assert_eq!(HostRequest::parse("host:document.new"), None);
    }
}
