//! The Team compiler's section of the settings file
//! (`spec/team_compiler/settings.md` "Team compiler settings"). A key enters with the code that
//! reads it, so the file never carries a setting nothing honors yet.

use std::fmt;
use std::path::PathBuf;

use pes_version::{Engine, PesVersion};
use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The Team compiler's settings, the keys this version reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct TeamCompilerSettings {
    /// The single output CPK's name, without `.cpk`.
    pub(crate) cpk_name: String,
    /// Where compiled CPKs go; a relative path resolves beside the executable.
    pub(crate) output_folder_path: PathBuf,
    /// The local savefile the compile's aesthetics patch is applied to.
    pub(crate) savefile_path: SavefilePath,
    /// Cup DLC mode: team content split into size-capped parts plus a bins CPK.
    pub(crate) multicpk_mode: bool,
    /// The stem of the teams part slots in multi-CPK mode: every official list entry
    /// `{prefix}_{NN}_{stem}.cpk` is a slot (`4cc_41_teams`), filled in the order of `NN`.
    pub(crate) teams_cpk_name: String,
    /// The most bytes a teams part may hold, its table of contents included; a single CPK
    /// past it is only warned about. A byte count, not text like `3 GB`: it needs no unit
    /// parser and leaves no doubt between GB and GiB.
    pub(crate) cpk_part_max_size: u64,
    /// The CPK, without `.cpk`, that holds the bins and the overrides in multi-CPK mode.
    pub(crate) bins_cpk_name: String,
    /// The CPK, without `.cpk`, that holds the referees of a normal compile whose exports
    /// include a refs export, in single and multi-CPK mode alike.
    pub(crate) refs_cpk_name: String,
    /// Whether every DDS a PES 15-17 run emits is WESYS-zlibbed (`compresses_dds`).
    pub(crate) dds_compression: DdsCompression,
    /// Disallowed file types are errors when on, info notes when off.
    pub(crate) strict_file_type_check: bool,
    /// Whether folders with errors are kept instead of discarded.
    pub(crate) pass_through: bool,
    /// The working teams list; a relative path resolves in the data directory.
    pub(crate) teams_list_path: PathBuf,
}

impl Default for TeamCompilerSettings {
    fn default() -> Self {
        TeamCompilerSettings {
            cpk_name: "4cc_99_test".to_owned(),
            output_folder_path: PathBuf::from("output"),
            savefile_path: SavefilePath::Auto,
            multicpk_mode: false,
            teams_cpk_name: "teams".to_owned(),
            // 3 GiB: comfortably under the 4 GiB a Git for Windows object can hold, the limit
            // the cup DLC's repository has.
            cpk_part_max_size: 3_221_225_472,
            bins_cpk_name: "4cc_08_bins".to_owned(),
            refs_cpk_name: "4cc_18_referees".to_owned(),
            dds_compression: DdsCompression::Auto,
            strict_file_type_check: true,
            pass_through: false,
            teams_list_path: PathBuf::from("teams_list.txt"),
        }
    }
}

/// `dds_compression`: whether every DDS a PES 15-17 run emits is
/// WESYS-zlibbed. `Auto` follows `multicpk_mode`: the cup DLC is what the size matters for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DdsCompression {
    Auto,
    On,
    Off,
}

impl DdsCompression {
    /// Whether DDS are compressed, given the run's `multicpk_mode`.
    pub(crate) fn resolved(self, multicpk_mode: bool) -> bool {
        match self {
            DdsCompression::Auto => multicpk_mode,
            DdsCompression::On => true,
            DdsCompression::Off => false,
        }
    }
}

// Read through a visitor, not an untagged helper enum of a bool and a string: a value of the
// wrong type then reads "invalid type: integer `1`, expected true, false or "auto"", where
// serde's untagged error names the helper's Rust type to the member.
impl<'de> Deserialize<'de> for DdsCompression {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(DdsCompressionVisitor)
    }
}

/// Reads `dds_compression` as the settings file spells it: `true`, `false` or `"auto"`.
struct DdsCompressionVisitor;

impl Visitor<'_> for DdsCompressionVisitor {
    type Value = DdsCompression;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("true, false or \"auto\"")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<DdsCompression, E> {
        Ok(if value {
            DdsCompression::On
        } else {
            DdsCompression::Off
        })
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<DdsCompression, E> {
        if value == "auto" {
            return Ok(DdsCompression::Auto);
        }
        Err(E::invalid_value(Unexpected::Str(value), &self))
    }
}

impl Serialize for DdsCompression {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            DdsCompression::Auto => serializer.serialize_str("auto"),
            DdsCompression::On => serializer.serialize_bool(true),
            DdsCompression::Off => serializer.serialize_bool(false),
        }
    }
}

/// `savefile_path`: the local savefile the compile's aesthetics patch is applied to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SavefilePath {
    /// `pes_savefile`'s discovery under Documents\KONAMI for the target version.
    Auto,
    /// A configured file; a relative path resolves beside the executable.
    Path(PathBuf),
}

// A visitor like `DdsCompression`'s: `"auto"` or a path string, never an untagged enum whose
// error would name the helper's Rust type.
impl<'de> Deserialize<'de> for SavefilePath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(SavefilePathVisitor)
    }
}

/// Reads `savefile_path` as the settings file spells it: `"auto"` or the path as a string.
struct SavefilePathVisitor;

impl Visitor<'_> for SavefilePathVisitor {
    type Value = SavefilePath;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("\"auto\" or a path")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<SavefilePath, E> {
        match value {
            "auto" => Ok(SavefilePath::Auto),
            path => Ok(SavefilePath::Path(PathBuf::from(path))),
        }
    }
}

impl Serialize for SavefilePath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            SavefilePath::Auto => serializer.serialize_str("auto"),
            SavefilePath::Path(path) => serializer.serialize_str(&path.to_string_lossy()),
        }
    }
}

/// The defaults as a settings table, merged into the file's section for every key it lacks.
pub(crate) fn default_table() -> toml::Table {
    toml::Table::try_from(TeamCompilerSettings::default()).expect(
        "the defaults are strings, ASCII paths, booleans and a count far below i64::MAX, \
             all of which TOML holds",
    )
}

impl TeamCompilerSettings {
    /// The CPK below which the installed CPKs are walked for the working bins and looked in for
    /// textures: the run's first CPK in the list's order, which is the bins CPK in multi-CPK
    /// mode, else `cpk_name`.
    pub(crate) fn boundary_cpk_name(&self) -> &str {
        if self.multicpk_mode {
            &self.bins_cpk_name
        } else {
            &self.cpk_name
        }
    }

    /// Whether a run for `version` WESYS-zlibs every DDS it emits: on PES 15-17 as
    /// `dds_compression` resolves against `multicpk_mode`, in every output mode; never on Fox,
    /// whose textures are FTEX.
    pub(crate) fn compresses_dds(&self, version: PesVersion) -> bool {
        match version.engine() {
            Engine::PreFox => self.dds_compression.resolved(self.multicpk_mode),
            Engine::Fox => false,
        }
    }
}

/// Reads the tool's settings table. A missing key loads as its default; a key this version does
/// not know is ignored, so a newer version's settings file does not break an older binary; a key
/// of the wrong type is an error naming it.
pub(crate) fn from_table(table: &toml::Table) -> Result<TeamCompilerSettings, toml::de::Error> {
    table.clone().try_into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip_through_the_table() {
        let table = default_table();
        assert_eq!(table["cpk_name"].as_str(), Some("4cc_99_test"));
        assert_eq!(table["strict_file_type_check"].as_bool(), Some(true));
        assert_eq!(table["teams_cpk_name"].as_str(), Some("teams"));
        assert_eq!(table["cpk_part_max_size"].as_integer(), Some(3_221_225_472));
        assert_eq!(table["bins_cpk_name"].as_str(), Some("4cc_08_bins"));
        assert_eq!(table["refs_cpk_name"].as_str(), Some("4cc_18_referees"));
        assert_eq!(from_table(&table).unwrap(), TeamCompilerSettings::default());
    }

    #[test]
    fn the_boundary_is_the_bins_cpk_in_multi_cpk_mode_and_cpk_name_otherwise() {
        let mut settings = TeamCompilerSettings {
            cpk_name: "4cc_61_midcup".to_owned(),
            ..TeamCompilerSettings::default()
        };
        assert_eq!(settings.boundary_cpk_name(), "4cc_61_midcup");
        settings.multicpk_mode = true;
        assert_eq!(settings.boundary_cpk_name(), "4cc_08_bins");
    }

    #[test]
    fn a_missing_key_takes_its_default_and_an_unknown_key_is_ignored() {
        let table: toml::Table =
            toml::from_str("pass_through = true\nsetting_from_a_newer_version = 3").unwrap();
        let settings = from_table(&table).unwrap();
        assert!(settings.pass_through);
        assert_eq!(settings.cpk_name, "4cc_99_test");
    }

    #[test]
    fn dds_compression_reads_true_false_and_auto_and_defaults_to_auto() {
        for (text, expected) in [
            ("dds_compression = true", DdsCompression::On),
            ("dds_compression = false", DdsCompression::Off),
            ("dds_compression = \"auto\"", DdsCompression::Auto),
        ] {
            let table: toml::Table = toml::from_str(text).unwrap();
            assert_eq!(
                from_table(&table).unwrap().dds_compression,
                expected,
                "{text}"
            );
        }
        assert_eq!(
            TeamCompilerSettings::default().dds_compression,
            DdsCompression::Auto
        );
        assert_eq!(default_table()["dds_compression"].as_str(), Some("auto"));
        for (value, written) in [
            (DdsCompression::On, toml::Value::Boolean(true)),
            (DdsCompression::Off, toml::Value::Boolean(false)),
        ] {
            let settings = TeamCompilerSettings {
                dds_compression: value,
                ..TeamCompilerSettings::default()
            };
            let table = toml::Table::try_from(&settings).unwrap();
            assert_eq!(table["dds_compression"], written);
            assert_eq!(from_table(&table).unwrap(), settings);
        }
    }

    #[test]
    fn a_dds_compression_other_than_true_false_or_auto_is_an_error_naming_it() {
        for text in [
            "dds_compression = 1",
            "dds_compression = \"yes\"",
            "dds_compression = \"Auto\"",
        ] {
            let table: toml::Table = toml::from_str(text).unwrap();
            let error = from_table(&table).unwrap_err().to_string();
            assert!(error.contains("dds_compression"), "{error}");
            assert!(
                error.contains("expected true, false or \"auto\""),
                "{error}"
            );
        }
    }

    #[test]
    fn auto_follows_multicpk_mode_and_true_and_false_ignore_it() {
        for multicpk_mode in [false, true] {
            assert_eq!(DdsCompression::Auto.resolved(multicpk_mode), multicpk_mode);
            assert!(DdsCompression::On.resolved(multicpk_mode));
            assert!(!DdsCompression::Off.resolved(multicpk_mode));
        }
    }

    #[test]
    fn only_pre_fox_runs_compress_dds() {
        let settings = |dds_compression, multicpk_mode| TeamCompilerSettings {
            dds_compression,
            multicpk_mode,
            ..TeamCompilerSettings::default()
        };
        assert!(settings(DdsCompression::On, false).compresses_dds(PesVersion::Pes17));
        assert!(settings(DdsCompression::Auto, true).compresses_dds(PesVersion::Pes15));
        assert!(!settings(DdsCompression::Auto, false).compresses_dds(PesVersion::Pes16));
        assert!(!settings(DdsCompression::Off, true).compresses_dds(PesVersion::Pes17));
        for version in [PesVersion::Pes18, PesVersion::Pes21] {
            assert!(
                !settings(DdsCompression::On, true).compresses_dds(version),
                "{version}"
            );
        }
    }

    #[test]
    fn savefile_path_reads_auto_and_a_path_and_round_trips() {
        for (text, expected) in [
            ("savefile_path = \"auto\"", SavefilePath::Auto),
            (
                "savefile_path = \"save/EDIT00000000\"",
                SavefilePath::Path(PathBuf::from("save/EDIT00000000")),
            ),
        ] {
            let table: toml::Table = toml::from_str(text).unwrap();
            assert_eq!(
                from_table(&table).unwrap().savefile_path,
                expected,
                "{text}"
            );
        }
        assert_eq!(
            TeamCompilerSettings::default().savefile_path,
            SavefilePath::Auto
        );
        assert_eq!(default_table()["savefile_path"].as_str(), Some("auto"));
        for value in [
            SavefilePath::Auto,
            SavefilePath::Path(PathBuf::from("save/EDIT00000000")),
        ] {
            let settings = TeamCompilerSettings {
                savefile_path: value,
                ..TeamCompilerSettings::default()
            };
            let table = toml::Table::try_from(&settings).unwrap();
            assert_eq!(from_table(&table).unwrap(), settings);
        }
    }

    #[test]
    fn a_key_of_the_wrong_type_is_an_error_naming_it() {
        let table: toml::Table = toml::from_str("strict_file_type_check = \"yes\"").unwrap();
        let error = from_table(&table).unwrap_err();
        assert!(
            error.to_string().contains("strict_file_type_check"),
            "{error}"
        );
    }

    #[test]
    fn a_savefile_path_that_is_not_a_string_says_what_it_takes() {
        let table: toml::Table = toml::from_str("savefile_path = 3").unwrap();
        let error = from_table(&table).unwrap_err();
        assert!(
            error.to_string().contains("expected \"auto\" or a path"),
            "{error}"
        );
    }
}
