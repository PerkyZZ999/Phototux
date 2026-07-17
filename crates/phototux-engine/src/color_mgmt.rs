//! Document color profile metadata (handbook 16 / DR-012).
//!
//! Assign changes interpretation only. Convert rewrites pixels (separate command).
//! Optional embedded ICC bytes are validated and persisted with the document.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Hard cap on embedded ICC profile size (4 MiB).
pub const MAX_ICC_BYTES: usize = 4 * 1024 * 1024;

/// Working / document profile identity plus optional embedded ICC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentColorState {
    /// Profile tag shown in UI (e.g. `sRGB`, `Display-P3`).
    pub assigned_profile: String,
    /// True when pixels were last written assuming `assigned_profile`.
    pub pixels_match_assigned: bool,
    /// Soft-proof display simulation (proof profile tag; empty = off).
    #[serde(default)]
    pub soft_proof_profile: String,
    /// Soft-proof intent: `perceptual` | `relative` | `saturation` | `absolute`.
    #[serde(default = "default_proof_intent")]
    pub soft_proof_intent: String,
    /// Validated ICC profile bytes embedded in the document (`None` = tag-only).
    #[serde(default, with = "icc_hex_opt")]
    pub embedded_icc: Option<Vec<u8>>,
}

fn default_proof_intent() -> String {
    "relative".into()
}

impl Default for DocumentColorState {
    fn default() -> Self {
        Self {
            assigned_profile: "sRGB".into(),
            pixels_match_assigned: true,
            soft_proof_profile: String::new(),
            soft_proof_intent: default_proof_intent(),
            embedded_icc: None,
        }
    }
}

impl DocumentColorState {
    /// Assign a profile without rewriting pixels (DR-012).
    pub fn assign_profile(&mut self, profile: impl Into<String>) {
        let next = profile.into();
        if next != self.assigned_profile {
            self.assigned_profile = next;
            self.pixels_match_assigned = false;
        }
    }

    /// Record that a convert operation rewrote pixels to match the assigned profile.
    pub fn mark_converted(&mut self) {
        self.pixels_match_assigned = true;
    }

    /// Enable or disable soft-proof (empty profile disables).
    pub fn set_soft_proof(&mut self, profile: impl Into<String>, intent: impl Into<String>) {
        self.soft_proof_profile = profile.into();
        let intent = intent.into();
        self.soft_proof_intent = match intent.as_str() {
            "perceptual" | "saturation" | "absolute" => intent,
            _ => "relative".into(),
        };
    }

    pub fn soft_proof_active(&self) -> bool {
        !self.soft_proof_profile.is_empty()
    }

    pub fn has_embedded_icc(&self) -> bool {
        self.embedded_icc.as_ref().is_some_and(|b| !b.is_empty())
    }

    /// Set or clear embedded ICC after validation.
    ///
    /// # Errors
    /// Returns a static reason when bytes fail validation.
    pub fn set_embedded_icc(&mut self, bytes: Option<Vec<u8>>) -> Result<(), &'static str> {
        match bytes {
            None => {
                self.embedded_icc = None;
                Ok(())
            }
            Some(raw) => {
                validate_icc_profile(&raw)?;
                self.embedded_icc = Some(raw);
                Ok(())
            }
        }
    }

    /// Prepare convert: assign target profile and return whether a pixel rewrite is needed.
    pub fn begin_convert(&mut self, target: impl Into<String>) -> ConvertPlan {
        let target = target.into();
        let from = self.assigned_profile.clone();
        if from == target && self.pixels_match_assigned {
            return ConvertPlan {
                from,
                to: target,
                rewrite_pixels: false,
            };
        }
        let rewrite_pixels = from != target;
        self.assigned_profile = target.clone();
        ConvertPlan {
            from,
            to: target,
            rewrite_pixels,
        }
    }
}

/// Result of planning a profile convert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertPlan {
    pub from: String,
    pub to: String,
    pub rewrite_pixels: bool,
}

/// Validate ICC profile bytes (size + `acsp` signature at offset 36).
///
/// # Errors
/// Returns a static reason when the buffer is not a plausible ICC profile.
pub fn validate_icc_profile(bytes: &[u8]) -> Result<(), &'static str> {
    if bytes.is_empty() {
        return Err("empty ICC profile");
    }
    if bytes.len() > MAX_ICC_BYTES {
        return Err("ICC profile exceeds 4 MiB");
    }
    if bytes.len() < 128 {
        return Err("ICC profile too short");
    }
    if &bytes[36..40] != b"acsp" {
        return Err("ICC missing acsp signature");
    }
    Ok(())
}

/// Approximate sRGB ↔ Display-P3 linear matrix convert on straight RGBA8.
///
/// Unknown profile pairs leave pixels unchanged (caller still marks converted).
pub fn convert_rgba8_profile(pixels: &mut [u8], from: &str, to: &str) {
    if from == to || pixels.len() < 4 {
        return;
    }
    let matrix = match (from, to) {
        ("sRGB", "Display-P3") => SRGB_TO_P3,
        ("Display-P3", "sRGB") => P3_TO_SRGB,
        _ => return,
    };
    for px in pixels.chunks_exact_mut(4) {
        let r = srgb_eotf(px[0] as f32 / 255.0);
        let g = srgb_eotf(px[1] as f32 / 255.0);
        let b = srgb_eotf(px[2] as f32 / 255.0);
        let nr = matrix[0][0] * r + matrix[0][1] * g + matrix[0][2] * b;
        let ng = matrix[1][0] * r + matrix[1][1] * g + matrix[1][2] * b;
        let nb = matrix[2][0] * r + matrix[2][1] * g + matrix[2][2] * b;
        px[0] = (srgb_oetf(nr.clamp(0.0, 1.0)) * 255.0).round() as u8;
        px[1] = (srgb_oetf(ng.clamp(0.0, 1.0)) * 255.0).round() as u8;
        px[2] = (srgb_oetf(nb.clamp(0.0, 1.0)) * 255.0).round() as u8;
    }
}

// Approx Bradford-adapted matrices (linear light).
const SRGB_TO_P3: [[f32; 3]; 3] = [
    [0.8225, 0.1774, 0.0000],
    [0.0332, 0.9669, 0.0000],
    [0.0171, 0.0724, 0.9108],
];
const P3_TO_SRGB: [[f32; 3]; 3] = [
    [1.2249, -0.2247, 0.0000],
    [-0.0420, 1.0419, 0.0000],
    [-0.0197, -0.0786, 1.0979],
];

fn srgb_eotf(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb_oetf(c: f32) -> f32 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

mod icc_hex_opt {
    use super::*;

    pub fn serialize<S>(value: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            None => serializer.serialize_none(),
            Some(bytes) => serializer.serialize_some(&hex_encode(bytes)),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        match opt {
            None => Ok(None),
            Some(s) if s.is_empty() => Ok(None),
            Some(s) => hex_decode(&s).map(Some).map_err(serde::de::Error::custom),
        }
    }

    fn hex_encode(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0xf) as usize] as char);
        }
        out
    }

    fn hex_decode(s: &str) -> Result<Vec<u8>, &'static str> {
        if !s.len().is_multiple_of(2) {
            return Err("odd hex length");
        }
        let mut out = Vec::with_capacity(s.len() / 2);
        let bytes = s.as_bytes();
        let mut i = 0;
        while i + 1 < bytes.len() {
            let hi = hex_nibble(bytes[i])?;
            let lo = hex_nibble(bytes[i + 1])?;
            out.push((hi << 4) | lo);
            i += 2;
        }
        Ok(out)
    }

    fn hex_nibble(c: u8) -> Result<u8, &'static str> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err("invalid hex digit"),
        }
    }
}

/// Minimal 128-byte ICC header with `acsp` signature (tests / fixtures).
pub fn minimal_icc_fixture() -> Vec<u8> {
    let mut v = vec![0_u8; 128];
    v[0..4].copy_from_slice(&128_u32.to_be_bytes());
    v[36..40].copy_from_slice(b"acsp");
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assign_marks_mismatch() {
        let mut c = DocumentColorState::default();
        assert!(c.pixels_match_assigned);
        c.assign_profile("Display-P3");
        assert_eq!(c.assigned_profile, "Display-P3");
        assert!(!c.pixels_match_assigned);
        c.mark_converted();
        assert!(c.pixels_match_assigned);
    }

    #[test]
    fn convert_srgb_to_p3_changes_red() {
        let mut px = [255_u8, 0, 0, 255];
        convert_rgba8_profile(&mut px, "sRGB", "Display-P3");
        assert!(px[0] < 255 || px[1] > 0);
    }

    #[test]
    fn icc_validation_accepts_acsp() {
        assert!(validate_icc_profile(&minimal_icc_fixture()).is_ok());
        assert!(validate_icc_profile(&[]).is_err());
        assert!(validate_icc_profile(&[0_u8; 64]).is_err());
        let mut bad = minimal_icc_fixture();
        bad[36..40].copy_from_slice(b"xxxx");
        assert!(validate_icc_profile(&bad).is_err());
    }

    #[test]
    fn icc_serde_defaults_and_roundtrips() {
        let json = r#"{"assigned_profile":"sRGB","pixels_match_assigned":true}"#;
        let c: DocumentColorState = serde_json::from_str(json).expect("de");
        assert!(c.embedded_icc.is_none());

        let mut c = DocumentColorState::default();
        c.set_embedded_icc(Some(minimal_icc_fixture()))
            .expect("set");
        let ser = serde_json::to_string(&c).expect("ser");
        let back: DocumentColorState = serde_json::from_str(&ser).expect("de2");
        assert_eq!(back.embedded_icc, c.embedded_icc);
        c.set_embedded_icc(None).expect("clear");
        assert!(!c.has_embedded_icc());
    }
}
