//! The supported PES versions and the engine each one uses.
//!
//! One closed set shared by everything that is version-aware (settings, savefile schemas,
//! skeleton data, the compiler), so no crate keeps its own copy. Version-specific *facts*
//! (offsets, keys, paths) do not live here; they are data in the crate that owns the format.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A PES release the suite supports, `PES 2015` through `PES 2021`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub enum PesVersion {
    /// PES 2015 (pre-Fox engine).
    Pes15,
    /// PES 2016 (pre-Fox engine).
    Pes16,
    /// PES 2017 (pre-Fox engine).
    Pes17,
    /// PES 2018 (Fox engine).
    Pes18,
    /// PES 2019 (Fox engine).
    Pes19,
    /// PES 2020 (Fox engine).
    Pes20,
    /// PES 2021 (Fox engine).
    Pes21,
}

/// The two model/texture pipelines: `.model` + `.mtl` before PES 2018, FMDL/FPK/FTEX from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Engine {
    /// PES 2015 to 2017.
    PreFox,
    /// PES 2018 to 2021.
    Fox,
}

impl PesVersion {
    /// Every supported version, oldest first.
    pub const ALL: [PesVersion; 7] = [
        PesVersion::Pes15,
        PesVersion::Pes16,
        PesVersion::Pes17,
        PesVersion::Pes18,
        PesVersion::Pes19,
        PesVersion::Pes20,
        PesVersion::Pes21,
    ];

    /// The short number the community uses (`16` for PES 2016).
    pub fn number(self) -> u16 {
        match self {
            PesVersion::Pes15 => 15,
            PesVersion::Pes16 => 16,
            PesVersion::Pes17 => 17,
            PesVersion::Pes18 => 18,
            PesVersion::Pes19 => 19,
            PesVersion::Pes20 => 20,
            PesVersion::Pes21 => 21,
        }
    }

    /// The release year (`2016` for PES 2016).
    pub fn year(self) -> u16 {
        2000 + self.number()
    }

    /// Which engine the version's models and textures use.
    pub fn engine(self) -> Engine {
        match self {
            PesVersion::Pes15 | PesVersion::Pes16 | PesVersion::Pes17 => Engine::PreFox,
            PesVersion::Pes18 | PesVersion::Pes19 | PesVersion::Pes20 | PesVersion::Pes21 => {
                Engine::Fox
            }
        }
    }

    /// Parses a version from its short number (`16`) or its year (`2016`).
    pub fn from_number(number: u16) -> Option<PesVersion> {
        let short = if number >= 2000 {
            number - 2000
        } else {
            number
        };
        PesVersion::ALL
            .into_iter()
            .find(|version| version.number() == short)
    }
}

/// The number or text the caller gave to `TryFrom<u16>` or `FromStr` was not a supported version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownVersion(pub String);

impl fmt::Display for UnknownVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsupported PES version `{}` (expected 15 to 21)",
            self.0
        )
    }
}

impl std::error::Error for UnknownVersion {}

impl TryFrom<u16> for PesVersion {
    type Error = UnknownVersion;

    fn try_from(number: u16) -> Result<Self, Self::Error> {
        PesVersion::from_number(number).ok_or_else(|| UnknownVersion(number.to_string()))
    }
}

impl From<PesVersion> for u16 {
    fn from(version: PesVersion) -> u16 {
        version.number()
    }
}

impl FromStr for PesVersion {
    type Err = UnknownVersion;

    /// Accepts `21`, `2021`, `pes21`, `pes2021` or `PES 2021`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let digits = text
            .trim()
            .trim_start_matches(['p', 'P', 'e', 'E', 's', 'S'])
            .trim();
        digits
            .parse::<u16>()
            .ok()
            .and_then(PesVersion::from_number)
            .ok_or_else(|| UnknownVersion(text.to_string()))
    }
}

impl fmt::Display for PesVersion {
    /// `PES 2021`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PES {}", self.year())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_of_every_version() {
        let expected = [
            Engine::PreFox,
            Engine::PreFox,
            Engine::PreFox,
            Engine::Fox,
            Engine::Fox,
            Engine::Fox,
            Engine::Fox,
        ];
        for (version, engine) in PesVersion::ALL.into_iter().zip(expected) {
            assert_eq!(version.engine(), engine, "{version}");
        }
    }

    #[test]
    fn parses_numbers_years_and_prefixes() {
        for text in ["16", "2016", "pes16", "PES 2016", " pes2016 "] {
            assert_eq!(text.parse::<PesVersion>(), Ok(PesVersion::Pes16), "{text}");
        }
        assert!("14".parse::<PesVersion>().is_err());
        assert!("22".parse::<PesVersion>().is_err());
        assert!("".parse::<PesVersion>().is_err());
    }

    #[test]
    fn an_unknown_version_names_what_was_given() {
        let err = "pes 2014"
            .parse::<PesVersion>()
            .expect_err("14 is not supported");
        assert_eq!(
            err.to_string(),
            "unsupported PES version `pes 2014` (expected 15 to 21)"
        );
        let err = PesVersion::try_from(22).expect_err("22 is not supported");
        assert_eq!(
            err.to_string(),
            "unsupported PES version `22` (expected 15 to 21)"
        );
    }

    #[test]
    fn year_and_display_name_the_release_year() {
        for ((version, number), year) in PesVersion::ALL.into_iter().zip(15..=21).zip(2015..=2021) {
            assert_eq!(version.number(), number, "{version:?}");
            assert_eq!(version.year(), year, "{version:?}");
        }
        assert_eq!(PesVersion::Pes21.to_string(), "PES 2021");
        for version in PesVersion::ALL {
            assert_eq!(version.to_string().parse::<PesVersion>(), Ok(version));
        }
    }

    #[derive(Serialize, Deserialize)]
    struct Holder {
        version: PesVersion,
    }

    #[test]
    fn serde_uses_the_short_number() {
        let text = toml::to_string(&Holder {
            version: PesVersion::Pes19,
        })
        .unwrap();
        assert_eq!(text.trim(), "version = 19");
        let back: Holder = toml::from_str("version = 2015").unwrap();
        assert_eq!(back.version, PesVersion::Pes15);
        assert!(toml::from_str::<Holder>("version = 14").is_err());
    }
}
