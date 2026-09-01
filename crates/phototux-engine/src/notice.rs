//! Transient messages to the user — the toast channel.
//!
//! The status bar used to carry both the document summary ("1920×1080 · zoom
//! 53% · Layer 1 …") and every message, so a message was overwritten by the
//! next summary refresh and a user who looked away missed it entirely. They are
//! different things: the summary is *state*, always true and always there;
//! a notice is an *event*, true once. Only one of them belongs in permanent
//! chrome.
//!
//! Severity is a vocabulary rather than something the presenter infers from the
//! text. Deciding whether a message is an error by searching it for the word
//! "failed" is the same mistake as classifying a typed error by grepping its
//! `Display` — it works until a perfectly ordinary message contains the word.

use serde::{Deserialize, Serialize};

/// How much a notice wants the user to care.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NoticeLevel {
    /// Something happened that the user asked for. Fades.
    #[default]
    Info,
    /// Something the user asked for could not be done, and they can fix it.
    /// Fades — nothing has been lost, and the command can be retried.
    Warning,
    /// Something failed. Does **not** fade: a save that did not happen must not
    /// scroll past while the user is looking at the canvas.
    Error,
}

impl NoticeLevel {
    pub const ALL: [Self; 3] = [Self::Info, Self::Warning, Self::Error];

    /// Wire key, shared with QML.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    /// Parse a wire key, `None` when it names no level.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.as_str() == key)
    }

    /// Phosphor icon stem for the toast's glyph.
    #[must_use]
    pub fn icon_key(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "x-circle",
        }
    }

    /// Whether the toast dismisses itself after a moment.
    ///
    /// Errors do not. A message that fades is one the user may never have seen,
    /// which is acceptable for "Workspace reset" and not for "Save failed".
    #[must_use]
    pub fn auto_dismisses(self) -> bool {
        !matches!(self, Self::Error)
    }

    /// Prefix for assistive technology, so the severity is spoken and not left
    /// to the colour of the toast.
    #[must_use]
    pub fn spoken_prefix(self) -> &'static str {
        match self {
            Self::Info => "",
            Self::Warning => "Warning: ",
            Self::Error => "Error: ",
        }
    }
}

/// One message on screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    /// Identity for dismissal. Monotonic within a session.
    pub id: u64,
    pub level: NoticeLevel,
    pub text: String,
    /// How many times this same message arrived in a row.
    ///
    /// Clicking a refused command three times should not stack three identical
    /// toasts; the newest one counts up instead, which also keeps the queue
    /// from being flooded by one repeated rejection.
    pub repeats: u32,
}

impl Notice {
    /// The line assistive technology reads.
    #[must_use]
    pub fn spoken(&self) -> String {
        let mut out = format!("{}{}", self.level.spoken_prefix(), self.text);
        if self.repeats > 1 {
            out.push_str(&format!(" ({} times)", self.repeats));
        }
        out
    }
}

/// The notices currently on screen, oldest first.
///
/// Bounded, because a loop that posts on every frame would otherwise grow this
/// without limit and cover the canvas it is trying to report on.
#[derive(Debug, Default, Clone)]
pub struct NoticeQueue {
    notices: Vec<Notice>,
    next_id: u64,
}

impl NoticeQueue {
    /// Most toasts that may be on screen at once.
    ///
    /// Four is about what fits above the status bar without becoming a wall,
    /// and past four the oldest is almost certainly stale anyway.
    pub const MAX_VISIBLE: usize = 4;

    /// Post a message, or count up the newest if it repeats it.
    ///
    /// Returns the notice's id. Repeats keep the *original* id so a toast that
    /// is counting up does not restart as a new element — its dismiss timer is
    /// refreshed by the count changing, which is the behaviour a user expects
    /// when the same thing happens again.
    pub fn post(&mut self, level: NoticeLevel, text: impl Into<String>) -> u64 {
        let text = text.into();
        if let Some(last) = self.notices.last_mut()
            && last.level == level
            && last.text == text
        {
            last.repeats += 1;
            return last.id;
        }
        self.next_id += 1;
        let id = self.next_id;
        self.notices.push(Notice {
            id,
            level,
            text,
            repeats: 1,
        });
        // Drop from the front: the oldest message is the one the user has had
        // the most chance to read.
        while self.notices.len() > Self::MAX_VISIBLE {
            self.notices.remove(0);
        }
        id
    }

    /// Remove one notice; `true` when it was there.
    pub fn dismiss(&mut self, id: u64) -> bool {
        let before = self.notices.len();
        self.notices.retain(|n| n.id != id);
        self.notices.len() != before
    }

    /// Remove every notice, for a deliberate "clear" gesture.
    pub fn clear(&mut self) {
        self.notices.clear();
    }

    #[must_use]
    pub fn notices(&self) -> &[Notice] {
        &self.notices
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.notices.is_empty()
    }

    /// `[{id, level, text, repeats, icon, autoDismiss, spoken}]` for QML.
    #[must_use]
    pub fn to_json(&self) -> String {
        let rows: Vec<serde_json::Value> = self
            .notices
            .iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "level": n.level.as_str(),
                    "text": n.text,
                    "repeats": n.repeats,
                    "icon": n.level.icon_key(),
                    "autoDismiss": n.level.auto_dismisses(),
                    "spoken": n.spoken(),
                })
            })
            .collect();
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_level_round_trips_and_has_its_own_icon() {
        for level in NoticeLevel::ALL {
            assert_eq!(NoticeLevel::parse(level.as_str()), Some(level));
            assert!(!level.icon_key().is_empty());
            for other in NoticeLevel::ALL {
                if other != level {
                    assert_ne!(level.icon_key(), other.icon_key());
                }
            }
        }
        assert_eq!(NoticeLevel::parse("fatal"), None);
    }

    #[test]
    fn only_an_error_stays_on_screen() {
        // The one asymmetry in the whole feature, and the reason it exists: a
        // save that did not happen must not scroll past while the user is
        // looking at the canvas.
        let sticky: Vec<&str> = NoticeLevel::ALL
            .into_iter()
            .filter(|l| !l.auto_dismisses())
            .map(NoticeLevel::as_str)
            .collect();
        assert_eq!(sticky, vec!["error"]);
    }

    #[test]
    fn a_repeated_message_counts_up_instead_of_stacking() {
        let mut q = NoticeQueue::default();
        let first = q.post(NoticeLevel::Warning, "Select a layer first.");
        let again = q.post(NoticeLevel::Warning, "Select a layer first.");
        assert_eq!(first, again, "a repeat must not become a new toast");
        assert_eq!(q.notices().len(), 1);
        assert_eq!(q.notices()[0].repeats, 2);
    }

    #[test]
    fn the_same_text_at_a_different_level_is_a_different_message() {
        let mut q = NoticeQueue::default();
        q.post(NoticeLevel::Info, "Saved.");
        q.post(NoticeLevel::Error, "Saved.");
        assert_eq!(q.notices().len(), 2);
    }

    #[test]
    fn a_repeat_only_folds_into_the_message_directly_above_it() {
        // Otherwise an alternating pair would collapse into two counters and
        // lose the order they actually happened in.
        let mut q = NoticeQueue::default();
        q.post(NoticeLevel::Info, "A");
        q.post(NoticeLevel::Info, "B");
        q.post(NoticeLevel::Info, "A");
        let texts: Vec<&str> = q.notices().iter().map(|n| n.text.as_str()).collect();
        assert_eq!(texts, vec!["A", "B", "A"]);
    }

    #[test]
    fn the_queue_is_bounded_and_drops_the_oldest() {
        let mut q = NoticeQueue::default();
        for i in 0..12 {
            q.post(NoticeLevel::Info, format!("message {i}"));
        }
        assert_eq!(q.notices().len(), NoticeQueue::MAX_VISIBLE);
        assert_eq!(
            q.notices()[0].text,
            format!("message {}", 12 - NoticeQueue::MAX_VISIBLE),
            "the surviving notices must be the newest ones"
        );
    }

    #[test]
    fn dismissing_reports_whether_it_found_anything() {
        let mut q = NoticeQueue::default();
        let id = q.post(NoticeLevel::Info, "hello");
        assert!(q.dismiss(id));
        assert!(!q.dismiss(id), "a second dismiss must not claim a hit");
        assert!(q.is_empty());
    }

    #[test]
    fn ids_are_not_reused_after_a_dismissal() {
        // QML keys its toasts by id. Reusing one would let a new message
        // inherit the dying animation of the one it replaced.
        let mut q = NoticeQueue::default();
        let first = q.post(NoticeLevel::Info, "a");
        q.dismiss(first);
        let second = q.post(NoticeLevel::Info, "b");
        assert_ne!(first, second);
    }

    #[test]
    fn assistive_technology_hears_the_severity_and_the_count() {
        let mut q = NoticeQueue::default();
        q.post(NoticeLevel::Error, "Save failed.");
        assert_eq!(q.notices()[0].spoken(), "Error: Save failed.");
        q.post(NoticeLevel::Warning, "Select a layer first.");
        q.post(NoticeLevel::Warning, "Select a layer first.");
        assert_eq!(
            q.notices()[1].spoken(),
            "Warning: Select a layer first. (2 times)"
        );
        // Info carries no prefix: reading "Info:" before every ordinary
        // confirmation is noise.
        q.post(NoticeLevel::Info, "Workspace reset.");
        assert_eq!(q.notices()[2].spoken(), "Workspace reset.");
    }

    #[test]
    fn the_json_carries_what_the_toast_needs_to_draw_itself() {
        let mut q = NoticeQueue::default();
        q.post(NoticeLevel::Error, "Save failed.");
        let json = q.to_json();
        for key in [
            "\"id\"",
            "\"level\"",
            "\"text\"",
            "\"repeats\"",
            "\"icon\"",
            "\"autoDismiss\"",
            "\"spoken\"",
        ] {
            assert!(json.contains(key), "{json} is missing {key}");
        }
        assert!(json.contains("\"autoDismiss\":false"));
        assert_eq!(NoticeQueue::default().to_json(), "[]");
    }
}
