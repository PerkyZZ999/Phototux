//! Durable file replacement (DR-014).
//!
//! Every format this crate writes needs the same sequence: create a uniquely
//! named sibling in the destination directory, write into it, flush it to the
//! device, rename it over the target, and remove the temporary if any step
//! failed. The sibling has to live in the destination directory because `rename`
//! is only atomic within a filesystem.
//!
//! That sequence was written out twice — once for raster export, once for
//! `.ptx` — character-for-character identical apart from the error type and one
//! message, backed by two independent sequence counters. Autosave's index file
//! did not use it at all. One implementation means the durability policy has one
//! statement, and a change to it (say, syncing the parent directory after the
//! rename, which none of the copies did) lands everywhere at once.

use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Distinguishes concurrent temporaries within one process.
///
/// One counter, not one per format: two counters starting at zero with the same
/// pid can hand out the same suffix, and only the differing file names kept the
/// two previous copies from colliding.
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// How many names to try before giving up.
const NAME_ATTEMPTS: u32 = 16;

/// Create a uniquely named temporary beside `parent/file_name`.
///
/// `create_new` makes the check-and-create one step, so two writers racing for
/// the same name cannot both believe they won it.
fn create_temporary_sibling(parent: &Path, file_name: &OsStr) -> io::Result<(PathBuf, File)> {
    for _ in 0..NAME_ATTEMPTS {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = OsString::from(".");
        temporary_name.push(file_name);
        temporary_name.push(format!(".phototux-{}-{sequence}.tmp", std::process::id()));
        let path = parent.join(temporary_name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique temporary file",
    ))
}

/// Replace `path` with whatever `write` produces, atomically.
///
/// `write` receives the opened temporary. It may fail with any error type; on
/// failure the temporary is removed and the error returned unchanged, so the
/// destination is left exactly as it was. The file is synced before the rename,
/// so a crash cannot leave the destination name pointing at unflushed content.
///
/// # Errors
/// Returns `E` from `write`, or an I/O error from creating, syncing or renaming
/// the temporary — both converted through `From<io::Error>`.
pub fn write_atomic<E, F>(path: &Path, write: F) -> Result<(), E>
where
    E: From<io::Error>,
    F: FnOnce(&mut File) -> Result<(), E>,
{
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        E::from(io::Error::new(
            io::ErrorKind::InvalidInput,
            "destination path has no file name",
        ))
    })?;
    let (temporary_path, mut file) = create_temporary_sibling(parent, file_name)?;
    let result = (|| {
        write(&mut file)?;
        file.sync_all()?;
        std::fs::rename(&temporary_path, path)?;
        Ok(())
    })();
    if result.is_err() {
        // Best effort: the destination is already untouched, so a leftover
        // temporary is untidy rather than harmful, and reporting the cleanup
        // error instead of the real one would hide the cause.
        let _ = std::fs::remove_file(&temporary_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "phototux-atomic-{}-{tag}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn a_successful_write_replaces_the_destination() {
        let dir = temp_dir("ok");
        let path = dir.join("out.bin");
        write_atomic::<io::Error, _>(&path, |f| f.write_all(b"hello")).expect("write");
        assert_eq!(std::fs::read(&path).expect("read"), b"hello");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_write_leaves_the_previous_contents_intact() {
        let dir = temp_dir("fail");
        let path = dir.join("out.bin");
        std::fs::write(&path, b"original").expect("seed");

        let outcome = write_atomic::<io::Error, _>(&path, |f| {
            f.write_all(b"partial")?;
            Err(io::Error::other("encoder gave up"))
        });

        assert!(outcome.is_err());
        assert_eq!(
            std::fs::read(&path).expect("read"),
            b"original",
            "a failed write must not damage the destination"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_write_leaves_no_temporary_behind() {
        let dir = temp_dir("cleanup");
        let path = dir.join("out.bin");
        let _ = write_atomic::<io::Error, _>(&path, |_| Err(io::Error::other("nope")));

        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".phototux-"))
            .collect();
        assert!(leftovers.is_empty(), "temporary was not cleaned up");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_writers_to_one_directory_get_distinct_temporaries() {
        let dir = temp_dir("unique");
        let mut names = Vec::new();
        for i in 0..8 {
            let (path, _file) =
                create_temporary_sibling(&dir, OsStr::new("same.bin")).expect("temporary");
            assert!(!names.contains(&path), "temporary {i} reused a name");
            names.push(path);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_path_without_a_file_name_is_rejected() {
        let outcome = write_atomic::<io::Error, _>(Path::new("/"), |f| f.write_all(b"x"));
        assert!(outcome.is_err());
    }
}
