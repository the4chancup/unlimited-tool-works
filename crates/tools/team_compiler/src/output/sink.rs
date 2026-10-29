//! Where the writer's entries go (`spec/team_compiler/pipeline.md` "5. Writer", step 5): one CPK, or
//! a folder of loose files laid out as the CPK's entries would be.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use cpk::CpkWriter;
use studio_core::TOOL_VERSION;

/// Where the writer puts each entry, by its path in the output: the sink knows nothing of game
/// paths or modes.
pub(crate) enum OutputSink {
    /// One CPK file at `path`, created with its first entry.
    Cpk {
        path: PathBuf,
        cpk: Option<CpkWriter<File>>,
    },
    /// A folder of loose files, each entry at its path below `root`, created with the first.
    Loose { root: PathBuf },
}

impl OutputSink {
    /// A CPK to be written at `path`; nothing is created yet.
    pub(crate) fn cpk(path: PathBuf) -> OutputSink {
        OutputSink::Cpk { path, cpk: None }
    }

    /// A folder of loose files at `root`; nothing is created yet.
    pub(crate) fn loose(root: PathBuf) -> OutputSink {
        OutputSink::Loose { root }
    }

    /// Writes `bytes` as the entry `path` (`/`-separated, as in a CPK). A path added twice is
    /// an error naming it, whichever the sink: the writer's duplicate invariant.
    pub(crate) fn add(&mut self, path: &str, bytes: &[u8]) -> anyhow::Result<()> {
        match self {
            OutputSink::Cpk {
                path: cpk_path,
                cpk,
            } => {
                let writer = match cpk {
                    Some(writer) => writer,
                    None => cpk.insert(create_cpk(cpk_path)?),
                };
                writer
                    .add(path, bytes, None)
                    .with_context(|| format!("cannot add {path} to {}", cpk_path.display()))
            }
            OutputSink::Loose { root } => {
                let cannot_write = || format!("cannot write {path} in {}", root.display());
                let file = root.join(path);
                if let Some(folder) = file.parent() {
                    fs::create_dir_all(folder).with_context(cannot_write)?;
                }
                // `create_new`, so an entry that arrives twice is refused as a CPK refuses it,
                // instead of silently replacing the first.
                File::create_new(&file)
                    .and_then(|mut written| written.write_all(bytes))
                    .with_context(cannot_write)
            }
        }
    }

    /// Completes the output: the CPK's table of contents written and the file closed; a folder
    /// needs nothing more, and neither does a CPK that was never created.
    pub(crate) fn finish(self) -> anyhow::Result<()> {
        match self {
            OutputSink::Cpk {
                path,
                cpk: Some(cpk),
            } => {
                cpk.finish()
                    .with_context(|| format!("cannot write the CPK {}", path.display()))?;
                Ok(())
            }
            OutputSink::Cpk { cpk: None, .. } | OutputSink::Loose { .. } => Ok(()),
        }
    }
}

/// Creates the CPK file at `path`, and its folder.
pub(crate) fn create_cpk(path: &Path) -> anyhow::Result<CpkWriter<File>> {
    let cannot_create = || format!("cannot create the CPK {}", path.display());
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).with_context(cannot_create)?;
    }
    let file = File::create(path).with_context(cannot_create)?;
    CpkWriter::new(file, TOOL_VERSION).with_context(cannot_create)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cpk::CpkArchive;

    use super::*;
    use crate::testing::scratch;

    #[test]
    fn the_loose_sink_writes_each_entry_at_its_path_with_its_bytes() {
        let temp = scratch("sink_loose");
        let root = temp.path().join("livecpk");
        let mut sink = OutputSink::loose(root.clone());
        assert!(!root.exists(), "nothing is created before the first entry");

        sink.add("common/etc/TeamColor.bin", b"team colors")
            .unwrap();
        sink.add("common/etc/pesdb/Player.bin", b"players").unwrap();
        sink.add("top.bin", b"top").unwrap();
        sink.finish().unwrap();

        assert_eq!(
            fs::read(root.join("common/etc/TeamColor.bin")).unwrap(),
            b"team colors"
        );
        assert_eq!(
            fs::read(root.join("common/etc/pesdb/Player.bin")).unwrap(),
            b"players"
        );
        assert_eq!(fs::read(root.join("top.bin")).unwrap(), b"top");
    }

    #[test]
    fn a_path_added_twice_to_the_loose_sink_is_an_error_naming_it_and_the_first_is_kept() {
        let temp = scratch("sink_loose_twice");
        let root = temp.path().join("livecpk");
        let mut sink = OutputSink::loose(root.clone());
        sink.add("common/etc/TeamColor.bin", b"first").unwrap();

        let error = sink.add("common/etc/TeamColor.bin", b"second").unwrap_err();

        assert!(
            error.to_string().starts_with(&format!(
                "cannot write common/etc/TeamColor.bin in {}",
                root.display()
            )),
            "{error}"
        );
        assert_eq!(
            fs::read(root.join("common/etc/TeamColor.bin")).unwrap(),
            b"first"
        );
    }

    #[test]
    fn the_cpk_sink_s_entries_read_back_as_written_and_a_path_twice_is_refused() {
        let temp = scratch("sink_cpk");
        let path = temp.path().join("run/cup.cpk");
        let mut sink = OutputSink::cpk(path.clone());
        assert!(!path.exists(), "nothing is created before the first entry");

        sink.add("common/etc/TeamColor.bin", b"team colors")
            .unwrap();
        sink.add("a/b.bin", b"b").unwrap();
        let error = sink.add("a/b.bin", b"again").unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("cannot add a/b.bin to {}", path.display())
        );
        sink.finish().unwrap();

        let mut archive = CpkArchive::open(File::open(&path).unwrap()).unwrap();
        let entries = archive.entries().to_vec();
        let read: BTreeMap<String, Vec<u8>> = entries
            .iter()
            .map(|entry| (entry.path.clone(), archive.read(entry).unwrap()))
            .collect();
        assert_eq!(
            read,
            BTreeMap::from([
                (
                    "common/etc/TeamColor.bin".to_owned(),
                    b"team colors".to_vec()
                ),
                ("a/b.bin".to_owned(), b"b".to_vec()),
            ])
        );
    }
}
