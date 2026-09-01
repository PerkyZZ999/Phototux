//! Turning bytes into something a QML `Image` can load.
//!
//! Qt loads an image from a `data:` URL, which is the only route from Rust-held
//! pixels into a QML `Image` here: the canvas is a native wgpu item, and
//! registering a `QQuickImageProvider` would mean hand-written C++, which
//! `AGENTS.md` keeps out of every crate but `phototux_canvas`.
//!
//! Base64 is written out rather than taken as a dependency. It is twenty lines
//! and a fixed standard; a crate for it would be a supply-chain entry for
//! something with no room to be wrong, and the test below holds it to the RFC
//! 4648 vectors.

/// Standard base64 alphabet (RFC 4648 §4), the one `data:` URLs use.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode `bytes` as base64 with `=` padding.
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        // Pack up to three bytes into a 24-bit group, missing bytes as zero.
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).copied().map_or(0, u32::from);
        let b2 = chunk.get(2).copied().map_or(0, u32::from);
        let group = (b0 << 16) | (b1 << 8) | b2;
        for shift in [18, 12, 6, 0] {
            out.push(char::from(ALPHABET[((group >> shift) & 0x3F) as usize]));
        }
        // Each missing input byte costs one output character, replaced by pad.
        let padding = 3 - chunk.len();
        out.truncate(out.len() - padding);
        for _ in 0..padding {
            out.push('=');
        }
    }
    out
}

/// A `data:` URL for a PNG, ready to hand to a QML `Image`.
#[must_use]
pub fn png_data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", base64(png))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The RFC 4648 §10 test vectors, which is what "correct" means here.
    #[test]
    fn the_rfc_vectors_encode_exactly() {
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), expected, "input {input:?}");
        }
    }

    #[test]
    fn every_byte_value_survives_a_round_trip() {
        // Decoded here rather than trusting the encoder against itself: a
        // table-driven encoder with a wrong alphabet entry passes any test that
        // only compares its own output to its own output.
        let bytes: Vec<u8> = (0..=255).collect();
        let encoded = base64(&bytes);
        let mut decoded = Vec::new();
        let mut buffer = 0_u32;
        let mut bits = 0_u32;
        for ch in encoded.bytes().filter(|c| *c != b'=') {
            let value = ALPHABET
                .iter()
                .position(|a| *a == ch)
                .expect("only alphabet characters are emitted");
            buffer = (buffer << 6) | value as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                decoded.push((buffer >> bits) as u8);
            }
        }
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn the_output_length_is_always_a_multiple_of_four() {
        for len in 0..40 {
            let encoded = base64(&vec![0xAB; len]);
            assert_eq!(encoded.len() % 4, 0, "length {len} gave {encoded:?}");
        }
    }

    #[test]
    fn a_data_url_is_shaped_the_way_qml_expects() {
        let url = png_data_url(b"\x89PNG");
        assert!(url.starts_with("data:image/png;base64,"));
        assert!(url.ends_with(&base64(b"\x89PNG")));
    }
}
