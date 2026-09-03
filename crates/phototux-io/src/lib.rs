//! Document I/O boundaries: rasters, native `.ptx`, recovery, PSD subset (DR-013 / DR-026).

mod atomic;
mod psd;
mod ptx;
mod recovery;

pub use psd::{
    CompatibilityIssue, PsdError, PsdExport, PsdImport, export_psd, export_psd_path, format_report,
    import_psd_bytes, import_psd_path,
};
pub use ptx::{
    PTX_FORMAT_VERSION, PtxDocument, PtxError, PtxManifest, PtxParts, decode_ptx, encode_ptx,
    load_ptx, load_ptx_with_diagnostics, ptx_integrity_report, save_ptx_atomic,
};
pub use recovery::{
    RecoveryEntry, RecoveryError, discard_recovery, list_recoverable, load_recovery, recovery_dir,
    write_autosave, write_stroke_journal,
};

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Seek, Write};
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{
    DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageFormat, ImageReader, Limits,
};
use phototux_engine::validate_icc_profile;
use thiserror::Error;

/// Largest accepted width or height. Pixel allocation limits reject oversized area separately.
pub const MAX_DIMENSION: u32 = 32_768;
/// Largest normalized RGBA8 buffer accepted by the file boundary.
pub const MAX_RASTER_BYTES: u64 = 512 * 1024 * 1024;
/// Default visually lossless JPEG quality for explicit export.
pub const JPEG_QUALITY: u8 = 92;

/// Raster formats supported at the file boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterFormat {
    Png,
    Jpeg,
    Webp,
    Tiff,
    Bmp,
    Gif,
}

impl RasterFormat {
    /// Infer an export format from a destination extension.
    ///
    /// # Errors
    ///
    /// Returns [`RasterIoError::UnsupportedExtension`] for missing or unsupported extensions.
    pub fn from_path(path: &Path) -> Result<Self, RasterIoError> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .ok_or(RasterIoError::UnsupportedExtension)?;
        match extension.as_str() {
            "png" => Ok(Self::Png),
            "jpg" | "jpeg" => Ok(Self::Jpeg),
            "webp" => Ok(Self::Webp),
            "tif" | "tiff" => Ok(Self::Tiff),
            "bmp" => Ok(Self::Bmp),
            "gif" => Ok(Self::Gif),
            _ => Err(RasterIoError::UnsupportedExtension),
        }
    }

    fn from_image_format(format: ImageFormat) -> Result<Self, RasterIoError> {
        match format {
            ImageFormat::Png => Ok(Self::Png),
            ImageFormat::Jpeg => Ok(Self::Jpeg),
            ImageFormat::WebP => Ok(Self::Webp),
            ImageFormat::Tiff => Ok(Self::Tiff),
            ImageFormat::Bmp => Ok(Self::Bmp),
            ImageFormat::Gif => Ok(Self::Gif),
            _ => Err(RasterIoError::UnsupportedFormat),
        }
    }
}

/// Normalized, tightly packed RGBA8 raster owned by Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    width: u32,
    height: u32,
    pixels: Box<[u8]>,
}

impl Raster {
    /// Construct a checked RGBA8 raster.
    ///
    /// # Errors
    ///
    /// Returns a dimension or buffer-length error when the input violates file-boundary limits.
    pub fn new(width: u32, height: u32, pixels: Box<[u8]>) -> Result<Self, RasterIoError> {
        let expected = checked_rgba_len(width, height)?;
        if pixels.len() != expected {
            return Err(RasterIoError::InvalidPixelLength {
                expected,
                actual: pixels.len(),
            });
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn into_pixels(self) -> Box<[u8]> {
        self.pixels
    }
}

/// Recoverable raster file-boundary failures.
#[derive(Debug, Error)]
pub enum RasterIoError {
    #[error("unsupported raster image format")]
    UnsupportedFormat,
    #[error("destination must end in a supported raster extension")]
    UnsupportedExtension,
    #[error("image dimensions exceed the 32,768 pixel limit")]
    DimensionsTooLarge,
    /// The other end of the range, which `DimensionsTooLarge` used to answer
    /// for: a file is free to declare an edge of 0, and telling its author the
    /// image is too large is the opposite of what happened.
    #[error("image has a zero-width or zero-height edge")]
    DimensionsEmpty,
    #[error("image requires more than 512 MiB of RGBA memory")]
    RasterTooLarge,
    #[error("invalid RGBA buffer length: expected {expected} bytes, got {actual}")]
    InvalidPixelLength { expected: usize, actual: usize },
    #[error("image codec failed: {0}")]
    Codec(#[from] image::ImageError),
    #[error("image I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Decode raster content, apply orientation, and normalize it to RGBA8.
///
/// # Errors
///
/// Returns [`RasterIoError`] for unsupported formats, decode failures, or exceeded limits.
pub fn decode<R>(reader: R) -> Result<Raster, RasterIoError>
where
    R: BufRead + Seek,
{
    let mut reader = ImageReader::new(reader).with_guessed_format()?;
    reader.limits(decode_limits());
    let image_format = reader.format().ok_or(RasterIoError::UnsupportedFormat)?;
    RasterFormat::from_image_format(image_format)?;

    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation()?;
    let (source_width, source_height) = decoder.dimensions();
    checked_rgba_len(source_width, source_height)?;

    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    let rgba = image.into_rgba8();
    let (width, height) = rgba.dimensions();
    Raster::new(width, height, rgba.into_raw().into_boxed_slice())
}

/// Decode a raster file from disk.
///
/// # Errors
///
/// Returns [`RasterIoError`] when the path cannot be opened or decode fails.
pub fn decode_path(path: &Path) -> Result<Raster, RasterIoError> {
    let file = File::open(path)?;
    decode(BufReader::new(file))
}

/// Encode a normalized raster.
///
/// PNG/WebP/TIFF/BMP/GIF preserve RGBA where the codec allows. JPEG composites alpha over white.
///
/// # Errors
///
/// Returns [`RasterIoError`] when encoding or writing fails.
pub fn encode<W>(writer: W, raster: &Raster, format: RasterFormat) -> Result<(), RasterIoError>
where
    W: Write,
{
    encode_with_icc(writer, raster, format, None)
}

/// Encode a raster, optionally embedding a validated ICC profile (PNG only).
///
/// JPEG and other formats ignore `icc` (no error). Callers should prefer PNG when
/// profile embedding is required.
///
/// # Errors
///
/// Returns [`RasterIoError`] when ICC validation or encoding fails.
pub fn encode_with_icc<W>(
    mut writer: W,
    raster: &Raster,
    format: RasterFormat,
    icc: Option<&[u8]>,
) -> Result<(), RasterIoError>
where
    W: Write,
{
    if let Some(bytes) = icc {
        validate_icc_profile(bytes).map_err(|reason| {
            RasterIoError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                reason,
            ))
        })?;
    }
    match format {
        RasterFormat::Png => {
            let mut encoder = PngEncoder::new(&mut writer);
            if let Some(bytes) = icc {
                encoder
                    .set_icc_profile(bytes.to_vec())
                    .map_err(|error| RasterIoError::Codec(image::ImageError::Unsupported(error)))?;
            }
            encoder.write_image(
                raster.pixels(),
                raster.width(),
                raster.height(),
                ExtendedColorType::Rgba8,
            )?;
        }
        RasterFormat::Jpeg => {
            let rgb = flatten_rgba_over_white(raster);
            JpegEncoder::new_with_quality(writer, JPEG_QUALITY).write_image(
                &rgb,
                raster.width(),
                raster.height(),
                ExtendedColorType::Rgb8,
            )?;
        }
        RasterFormat::Webp => write_dynamic_format(&mut writer, raster, ImageFormat::WebP)?,
        RasterFormat::Tiff => write_dynamic_format(&mut writer, raster, ImageFormat::Tiff)?,
        RasterFormat::Bmp => write_dynamic_format(&mut writer, raster, ImageFormat::Bmp)?,
        RasterFormat::Gif => write_dynamic_format(&mut writer, raster, ImageFormat::Gif)?,
    }
    Ok(())
}

fn write_dynamic_format<W: Write>(
    writer: &mut W,
    raster: &Raster,
    format: ImageFormat,
) -> Result<(), RasterIoError> {
    let image =
        image::RgbaImage::from_raw(raster.width(), raster.height(), raster.pixels().to_vec())
            .ok_or(RasterIoError::InvalidPixelLength {
                expected: raster.pixels().len(),
                actual: raster.pixels().len(),
            })?;
    let dynamic = DynamicImage::ImageRgba8(image);
    let mut cursor = std::io::Cursor::new(Vec::new());
    dynamic.write_to(&mut cursor, format)?;
    writer.write_all(cursor.get_ref())?;
    Ok(())
}

/// Encode to a temporary sibling and atomically replace the destination.
///
/// # Errors
///
/// Returns [`RasterIoError`] when temporary creation, encode, sync, or rename fails.
pub fn encode_path_atomic(
    path: &Path,
    raster: &Raster,
    format: RasterFormat,
) -> Result<(), RasterIoError> {
    encode_path_atomic_with_icc(path, raster, format, None)
}

/// Atomic path encode with optional ICC embed (PNG).
///
/// # Errors
///
/// Returns [`RasterIoError`] when temporary creation, encode, sync, or rename fails.
pub fn encode_path_atomic_with_icc(
    path: &Path,
    raster: &Raster,
    format: RasterFormat,
    icc: Option<&[u8]>,
) -> Result<(), RasterIoError> {
    if path.file_name().is_none() {
        return Err(RasterIoError::UnsupportedExtension);
    }
    crate::atomic::write_atomic(path, |file| {
        let mut writer = BufWriter::new(&*file);
        encode_with_icc(&mut writer, raster, format, icc)?;
        writer.flush()?;
        Ok(())
    })
}

fn decode_limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_RASTER_BYTES);
    limits
}

fn checked_rgba_len(width: u32, height: u32) -> Result<usize, RasterIoError> {
    // Four conditions used to share one error, and its message says "exceed
    // the limit" — so a corrupt file declaring 0x0 was reported to its reader
    // as an image too large to open.
    if width == 0 || height == 0 {
        return Err(RasterIoError::DimensionsEmpty);
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(RasterIoError::DimensionsTooLarge);
    }
    let bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(RasterIoError::RasterTooLarge)?;
    if bytes > MAX_RASTER_BYTES {
        return Err(RasterIoError::RasterTooLarge);
    }
    usize::try_from(bytes).map_err(|_| RasterIoError::RasterTooLarge)
}

fn flatten_rgba_over_white(raster: &Raster) -> Vec<u8> {
    let pixel_count = raster.pixels().len() / 4;
    let mut rgb = Vec::with_capacity(pixel_count * 3);
    for rgba in raster.pixels().chunks_exact(4) {
        let alpha = u16::from(rgba[3]);
        let inverse = 255 - alpha;
        for channel in &rgba[..3] {
            let blended = (u16::from(*channel) * alpha + 255 * inverse + 127) / 255;
            rgb.push(blended as u8);
        }
    }
    rgb
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use std::io::Cursor;

    #[test]
    fn png_embeds_icc_profile() {
        let raster = Raster::new(
            2,
            1,
            vec![10, 20, 30, 255, 200, 150, 100, 255].into_boxed_slice(),
        )
        .expect("valid raster");
        let icc = phototux_engine::minimal_icc_fixture();
        let mut encoded = Vec::new();
        encode_with_icc(&mut encoded, &raster, RasterFormat::Png, Some(&icc))
            .expect("encode PNG+ICC");
        // iCCP chunk marker appears after PNG signature / IHDR.
        assert!(
            encoded.windows(4).any(|w| w == b"iCCP"),
            "expected iCCP chunk in PNG"
        );
    }

    #[test]
    fn png_round_trip_preserves_rgba() {
        let raster = Raster::new(
            2,
            1,
            vec![10, 20, 30, 40, 200, 150, 100, 255].into_boxed_slice(),
        )
        .expect("valid raster");
        let mut encoded = Vec::new();
        encode(&mut encoded, &raster, RasterFormat::Png).expect("encode PNG");

        let decoded = decode(Cursor::new(encoded)).expect("decode PNG");
        assert_eq!(decoded, raster);
    }

    #[test]
    fn jpeg_round_trip_is_opaque_and_keeps_dimensions() {
        let raster = Raster::new(1, 1, vec![0, 0, 0, 0].into_boxed_slice()).expect("valid raster");
        let mut encoded = Vec::new();
        encode(&mut encoded, &raster, RasterFormat::Jpeg).expect("encode JPEG");

        let decoded = decode(Cursor::new(encoded)).expect("decode JPEG");
        assert_eq!((decoded.width(), decoded.height()), (1, 1));
        assert_eq!(decoded.pixels()[3], 255);
        assert!(decoded.pixels()[..3].iter().all(|channel| *channel >= 250));
    }

    #[test]
    fn rejects_rgba_allocation_over_limit() {
        let error = checked_rgba_len(MAX_DIMENSION, MAX_DIMENSION).expect_err("must exceed limit");
        assert!(matches!(error, RasterIoError::RasterTooLarge));
    }

    #[test]
    fn rejects_unknown_content() {
        let error = decode(Cursor::new(b"not an image".to_vec())).expect_err("must reject");
        assert!(matches!(error, RasterIoError::UnsupportedFormat));
    }

    #[test]
    fn atomic_path_export_round_trips() {
        let directory = std::env::temp_dir().join(format!(
            "phototux-io-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir(&directory).expect("create test directory");
        let path = directory.join("output.png");
        std::fs::write(&path, b"previous content").expect("seed destination");
        let raster = Raster::new(1, 1, vec![1, 2, 3, 4].into_boxed_slice()).expect("valid raster");

        encode_path_atomic(&path, &raster, RasterFormat::Png).expect("atomic export");
        let decoded = decode_path(&path).expect("decode exported image");
        assert_eq!(decoded, raster);

        std::fs::remove_dir_all(directory).expect("remove test directory");
    }
    /// A refusal must name the end of the range the value actually broke.
    ///
    /// `checked_rgba_len` folded four conditions into `DimensionsTooLarge`,
    /// two of which are the opposite — a file is free to declare a zero edge,
    /// and its reader was then told the image was too large to open.
    #[test]
    fn a_zero_edge_is_not_reported_as_too_large() {
        let empty = |w, h| {
            Raster::new(w, h, Vec::new().into_boxed_slice())
                .expect_err("a zero edge cannot hold pixels")
                .to_string()
        };
        for message in [empty(0, 0), empty(0, 4), empty(4, 0)] {
            assert!(
                message.contains("zero"),
                "a zero edge must say so, not {message:?}"
            );
            assert!(!message.contains("exceed"), "{message}");
        }
        let too_big = Raster::new(MAX_DIMENSION + 1, 1, Vec::new().into_boxed_slice())
            .expect_err("past the limit")
            .to_string();
        assert!(too_big.contains("exceed"), "{too_big}");
    }

    /// Writing to a place that will not take it fails cleanly, every way.
    #[test]
    fn a_write_that_cannot_land_reports_why() {
        let base = std::env::temp_dir().join(format!("phototux-qa-fs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).expect("base");
        let raster = Raster::new(2, 2, vec![9u8; 16].into_boxed_slice()).expect("raster");

        let missing = base.join("no-such-dir").join("out.png");
        assert!(
            encode_path_atomic(&missing, &raster, RasterFormat::Png).is_err(),
            "a missing parent directory must refuse"
        );

        let readonly = base.join("readonly");
        fs::create_dir_all(&readonly).expect("dir");
        let mut perms = fs::metadata(&readonly).expect("meta").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o500);
        fs::set_permissions(&readonly, perms).expect("chmod");
        assert!(
            encode_path_atomic(&readonly.join("out.png"), &raster, RasterFormat::Png).is_err(),
            "a read-only directory must refuse"
        );

        let gone = base.join("gone.png");
        encode_path_atomic(&gone, &raster, RasterFormat::Png).expect("write");
        fs::remove_file(&gone).expect("remove");
        assert!(
            decode_path(&gone).is_err(),
            "reading a file deleted underneath must refuse"
        );

        let mut perms = fs::metadata(&readonly).expect("meta").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o700);
        let _ = fs::set_permissions(&readonly, perms);
        let _ = fs::remove_dir_all(&base);
    }
}
