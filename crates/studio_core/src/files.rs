//! File writes that must not leave a half-written file behind.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Writes `path` through a sibling `<file name>.tmp`: `write` fills the temporary file, which
/// is then renamed onto `path`, so a failure part-way never leaves a half-written file under
/// the real name. On any error the temporary file is removed and the error returned.
pub fn replace_file(path: &Path, write: impl FnOnce(&Path) -> io::Result<()>) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let write_and_swap = || -> io::Result<()> {
        write(&tmp)?;
        fs::rename(&tmp, path)
    };
    if let Err(error) = write_and_swap() {
        // Best effort: the original error is the one to report.
        if let Err(removal) = fs::remove_file(&tmp) {
            log::debug!("{}: kept: {removal}", tmp.display());
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A fresh scratch folder for `test` under the system temp.
    fn scratch(test: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("studio_core_files_{test}_{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The write lands under `path`'s name, the temporary sibling renamed
    /// away, not left beside it.
    #[test]
    fn a_write_lands_at_the_path_and_the_temporary_is_gone() {
        let dir = scratch("write_lands");
        let path = dir.join("f.txt");
        replace_file(&path, |tmp| fs::write(tmp, "new bytes")).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new bytes");
        assert!(!dir.join("f.txt.tmp").exists());
    }

    /// A `write` that fails before it writes anything: the error is
    /// returned, `path`'s bytes are as they were, and no temporary stays.
    #[test]
    fn a_failed_write_leaves_the_old_file_and_no_temporary() {
        let dir = scratch("write_fails");
        let path = dir.join("f.txt");
        fs::write(&path, "old bytes").unwrap();
        let error = replace_file(&path, |_| Err(io::Error::other("the write's own failure")))
            .expect_err("the write's error is returned");
        assert_eq!(error.to_string(), "the write's own failure");
        assert_eq!(fs::read_to_string(&path).unwrap(), "old bytes");
        assert!(!dir.join("f.txt.tmp").exists());
    }

    /// A `write` that fills half the temporary then fails: the temporary's
    /// partial bytes never reach `path`, and the temporary is removed.
    #[test]
    fn a_half_written_temporary_never_replaces_the_file() {
        let dir = scratch("half_write");
        let path = dir.join("f.txt");
        fs::write(&path, "old bytes").unwrap();
        replace_file(&path, |tmp| {
            fs::write(tmp, "half")?;
            Err(io::Error::other("full"))
        })
        .expect_err("the write's error is returned");
        assert_eq!(fs::read_to_string(&path).unwrap(), "old bytes");
        assert!(!dir.join("f.txt.tmp").exists());
    }

    /// The rename's failure (`path` a directory, which a file never
    /// replaces): the error is returned and the written temporary removed.
    #[test]
    fn a_failed_rename_removes_the_temporary() {
        let dir = scratch("rename_fails");
        let target = dir.join("f.txt");
        fs::create_dir(&target).unwrap();
        replace_file(&target, |tmp| fs::write(tmp, "bytes"))
            .expect_err("a file is not renamed over a directory");
        assert!(!dir.join("f.txt.tmp").exists());
        assert!(target.is_dir());
    }
}
