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
//!
//! The checks themselves are private on purpose. [`ImagePayload`] and
//! [`CoveragePayload`] are the only way to obtain a payload, so the question
//! cannot be skipped or answered differently by one caller than another —
//! which is exactly what happened while every site held a bare
//! `(u32, u32, Vec<u8>)` and remembered its own rules.

/// Largest payload the clipboard will carry, in bytes.
///
/// Shared with the engine's snapshot ceiling rather than restated: both answer
/// "how much pixel data may we hold for the user at once", and letting them
/// drift would mean a payload the clipboard accepts but the publisher refuses.
const MAX_CLIPBOARD_BYTES: usize = phototux_engine::MAX_SNAPSHOT_BYTES;

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
fn accept_rgba(bytes: usize) -> Result<(), ClipboardRefusal> {
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
fn accept_coverage(coverage_len: usize, width: u32, height: u32) -> Result<(), ClipboardRefusal> {
    let pixels = (width as usize).saturating_mul(height as usize);
    if pixels == 0 {
        return Err(ClipboardRefusal::EmptyDocument);
    }
    if coverage_len != pixels {
        return Err(ClipboardRefusal::CoverageSizeMismatch);
    }
    accept_rgba(coverage_len)
}

/// Bytes an RGBA image of this size occupies, or `None` on overflow.
#[must_use]
fn rgba_byte_len(width: u32, height: u32) -> Option<usize> {
    (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)
}

/// A validated RGBA image on the clipboard.
///
/// The three channels used to travel as bare `(u32, u32, Vec<u8>)` tuples,
/// which carry no promise that the buffer matches the dimensions beside it.
/// Every site had to remember to check, and they did not agree on what to
/// check: the layer-mask copy validated only the size ceiling where its
/// selection-mask sibling validated the shape too, and the paste path
/// discarded the dimensions outright. The mismatch was caught eventually — by
/// the GPU, after the paste command had already committed a new layer — so the
/// user saw an empty layer and a texture error rather than a refusal.
///
/// Constructing the payload is the one place the question is asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePayload {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl ImagePayload {
    /// Validate `rgba` as a `width`×`height` image.
    ///
    /// # Errors
    /// [`ClipboardRefusal`] when the document is empty, the buffer is not
    /// exactly four bytes per pixel, or it is over the ceiling.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, ClipboardRefusal> {
        let expected = rgba_byte_len(width, height).ok_or(ClipboardRefusal::TooLarge)?;
        if expected == 0 {
            return Err(ClipboardRefusal::EmptyDocument);
        }
        if rgba.len() != expected {
            return Err(ClipboardRefusal::CoverageSizeMismatch);
        }
        accept_rgba(rgba.len())?;
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Whether this payload describes a document of exactly this size.
    #[must_use]
    pub fn fits(&self, width: u32, height: u32) -> bool {
        self.width == width && self.height == height
    }
}

/// A validated single-channel coverage buffer on the clipboard.
///
/// One byte per pixel, exactly document-sized — see [`accept_coverage`] for why
/// a buffer that disagrees is refused rather than truncated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoveragePayload {
    width: u32,
    height: u32,
    coverage: Vec<u8>,
}

impl CoveragePayload {
    /// Validate `coverage` as a `width`×`height` coverage buffer.
    ///
    /// # Errors
    /// [`ClipboardRefusal`] when the document is empty, the length disagrees,
    /// or the payload is over the ceiling.
    pub fn new(width: u32, height: u32, coverage: Vec<u8>) -> Result<Self, ClipboardRefusal> {
        accept_coverage(coverage.len(), width, height)?;
        Ok(Self {
            width,
            height,
            coverage,
        })
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn coverage(&self) -> &[u8] {
        &self.coverage
    }

    /// Whether this payload describes a document of exactly this size.
    #[must_use]
    pub fn fits(&self, width: u32, height: u32) -> bool {
        self.width == width && self.height == height
    }

    /// Greyscale RGBA preview of this coverage, for the OS clipboard.
    #[must_use]
    pub fn to_gray_rgba(&self) -> Vec<u8> {
        coverage_to_gray_rgba(&self.coverage)
    }
}

/// Expand single-channel coverage into an opaque greyscale RGBA image.
///
/// Coverage has no colour of its own, so previewing or handing it to the OS
/// clipboard means choosing one; grey at full alpha is the conventional reading
/// and matches how the mask is drawn on canvas.
#[must_use]
fn coverage_to_gray_rgba(coverage: &[u8]) -> Vec<u8> {
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
    fn an_image_payload_must_be_four_bytes_per_pixel() {
        assert!(ImagePayload::new(4, 4, vec![0; 64]).is_ok());
        assert_eq!(
            ImagePayload::new(4, 4, vec![0; 63]),
            Err(ClipboardRefusal::CoverageSizeMismatch)
        );
        assert_eq!(
            ImagePayload::new(4, 4, vec![0; 16]),
            Err(ClipboardRefusal::CoverageSizeMismatch),
            "one byte per pixel is a coverage buffer, not an image"
        );
    }

    #[test]
    fn an_image_payload_of_no_size_is_refused() {
        assert_eq!(
            ImagePayload::new(0, 0, Vec::new()),
            Err(ClipboardRefusal::EmptyDocument)
        );
    }

    /// Dimensions whose byte count overflows `usize` must refuse rather than
    /// wrap into a small number that some buffer happens to match.
    #[test]
    fn an_absurd_image_size_cannot_wrap_into_acceptance() {
        assert!(ImagePayload::new(u32::MAX, u32::MAX, vec![0; 16]).is_err());
    }

    #[test]
    fn a_coverage_payload_must_be_one_byte_per_pixel() {
        assert!(CoveragePayload::new(8, 4, vec![0; 32]).is_ok());
        assert_eq!(
            CoveragePayload::new(8, 4, vec![0; 31]),
            Err(ClipboardRefusal::CoverageSizeMismatch)
        );
    }

    /// The check the layer-mask copy was missing. It validated only the size
    /// ceiling, so a mask buffer that did not describe the document reached
    /// the clipboard and failed later at the GPU instead of here.
    #[test]
    fn a_mask_sized_for_another_document_is_refused_at_copy_time() {
        let other_document = vec![0; 32 * 32];
        assert_eq!(
            CoveragePayload::new(64, 64, other_document),
            Err(ClipboardRefusal::CoverageSizeMismatch)
        );
    }

    #[test]
    fn payloads_know_which_document_they_fit() {
        let image = ImagePayload::new(4, 4, vec![0; 64]).expect("image");
        assert!(image.fits(4, 4));
        assert!(!image.fits(4, 5));

        let coverage = CoveragePayload::new(4, 4, vec![0; 16]).expect("coverage");
        assert!(coverage.fits(4, 4));
        assert!(!coverage.fits(5, 4));
    }

    #[test]
    fn a_coverage_payload_previews_as_opaque_grey() {
        let coverage = CoveragePayload::new(3, 1, vec![0, 128, 255]).expect("coverage");
        assert_eq!(
            coverage.to_gray_rgba(),
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
