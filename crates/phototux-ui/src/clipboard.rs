//! Clipboard payload policy (handbook 21).
//!
//! The clipboard carries three channels — composited RGBA, selection coverage
//! and layer-mask coverage — and the rules about what may travel on them were
//! spread across seven `#[qslot]` bodies: the 64 MiB refusal was written as a
//! function-local `const` in five of them, and the check that a coverage buffer
//! matches its document size was open-coded beside each one. That made the size
//! cap five numbers that happened to agree, next to a sixth in the engine.
//!
//! Nothing here touches Qt or the OS clipboard. It is the decidable part —
//! is this payload allowed, and what shape must it be — which is why it can be
//! tested without a session.

/// Largest payload the clipboard will carry, in bytes.
///
/// Shared with the engine's snapshot ceiling rather than restated: both answer
/// "how much pixel data may we hold for the user at once", and letting them
/// drift would mean a payload the clipboard accepts but the publisher refuses.
pub const MAX_CLIPBOARD_BYTES: usize = phototux_engine::MAX_SNAPSHOT_BYTES;

/// Why a clipboard payload was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardRefusal {
    /// Larger than [`MAX_CLIPBOARD_BYTES`].
    TooLarge,
    /// Coverage length does not match the document's pixel count.
    CoverageSizeMismatch,
    /// Document has zero area, so no payload is meaningful.
    EmptyDocument,
}

impl ClipboardRefusal {
    /// Status-bar wording for this refusal.
    #[must_use]
    pub fn message(self, verb: &str) -> String {
        match self {
            Self::TooLarge => format!("{verb} refused: clipboard size limit"),
            Self::CoverageSizeMismatch => format!("{verb} refused: coverage size mismatch"),
            Self::EmptyDocument => format!("{verb} refused: empty document"),
        }
    }
}

/// Check an RGBA payload against the size ceiling.
///
/// # Errors
/// [`ClipboardRefusal::TooLarge`] when over [`MAX_CLIPBOARD_BYTES`].
pub fn accept_rgba(bytes: usize) -> Result<(), ClipboardRefusal> {
    if bytes > MAX_CLIPBOARD_BYTES {
        return Err(ClipboardRefusal::TooLarge);
    }
    Ok(())
}

/// Check a coverage buffer against the document it claims to describe.
///
/// Coverage is one byte per pixel, so a buffer that does not match the document
/// exactly is describing a different image — worth refusing rather than
/// truncating, because the result would be a silently misaligned paste.
///
/// # Errors
/// [`ClipboardRefusal`] when the document is empty, the length disagrees, or
/// the payload is over the ceiling.
pub fn accept_coverage(
    coverage_len: usize,
    width: u32,
    height: u32,
) -> Result<(), ClipboardRefusal> {
    let pixels = (width as usize).saturating_mul(height as usize);
    if pixels == 0 {
        return Err(ClipboardRefusal::EmptyDocument);
    }
    if coverage_len != pixels {
        return Err(ClipboardRefusal::CoverageSizeMismatch);
    }
    accept_rgba(coverage_len)
}

/// Expand single-channel coverage into an opaque greyscale RGBA image.
///
/// Coverage has no colour of its own, so previewing or handing it to the OS
/// clipboard means choosing one; grey at full alpha is the conventional reading
/// and matches how the mask is drawn on canvas.
#[must_use]
pub fn coverage_to_gray_rgba(coverage: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(coverage.len().saturating_mul(4));
    for &value in coverage {
        rgba.extend_from_slice(&[value, value, value, 255]);
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cap_is_the_engines_cap_not_a_second_number() {
        assert_eq!(MAX_CLIPBOARD_BYTES, phototux_engine::MAX_SNAPSHOT_BYTES);
    }

    #[test]
    fn payloads_at_the_ceiling_are_accepted_and_beyond_it_refused() {
        assert!(accept_rgba(MAX_CLIPBOARD_BYTES).is_ok());
        assert_eq!(
            accept_rgba(MAX_CLIPBOARD_BYTES + 1),
            Err(ClipboardRefusal::TooLarge)
        );
    }

    #[test]
    fn coverage_must_match_the_document_exactly() {
        assert!(accept_coverage(64 * 32, 64, 32).is_ok());
        assert_eq!(
            accept_coverage(64 * 32 - 1, 64, 32),
            Err(ClipboardRefusal::CoverageSizeMismatch)
        );
        assert_eq!(
            accept_coverage(64 * 32 + 1, 64, 32),
            Err(ClipboardRefusal::CoverageSizeMismatch)
        );
    }

    #[test]
    fn an_empty_document_carries_no_coverage() {
        assert_eq!(
            accept_coverage(0, 0, 0),
            Err(ClipboardRefusal::EmptyDocument)
        );
        assert_eq!(
            accept_coverage(0, 64, 0),
            Err(ClipboardRefusal::EmptyDocument)
        );
    }

    /// A pixel count that overflows `usize` must refuse rather than wrap into a
    /// small number that happens to match the buffer.
    #[test]
    fn an_absurd_document_size_cannot_wrap_into_acceptance() {
        assert!(accept_coverage(4, u32::MAX, u32::MAX).is_err());
    }

    #[test]
    fn coverage_expands_to_opaque_grey() {
        assert_eq!(
            coverage_to_gray_rgba(&[0, 128, 255]),
            vec![0, 0, 0, 255, 128, 128, 128, 255, 255, 255, 255, 255]
        );
    }

    #[test]
    fn refusal_messages_name_the_action() {
        assert!(
            ClipboardRefusal::TooLarge
                .message("Copy")
                .starts_with("Copy")
        );
        assert!(
            ClipboardRefusal::TooLarge
                .message("Paste")
                .starts_with("Paste")
        );
    }
}
