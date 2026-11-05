//! The CPK path a player's portrait DDS sits at, shared by the Team compiler's
//! writer and the Save editor's reader.

use pes_version::PesVersion;

/// The CPK path of one player's portrait, a DDS, by player id: the same folder on every
/// version, the file named `player_{player id}.dds` up to PES 2018 and `{player id}.dds` from
/// PES 2019.
pub fn portrait_path(version: PesVersion, player_id: u32) -> String {
    let prefix = match version {
        PesVersion::Pes15 | PesVersion::Pes16 | PesVersion::Pes17 | PesVersion::Pes18 => "player_",
        PesVersion::Pes19 | PesVersion::Pes20 | PesVersion::Pes21 => "",
    };
    format!("common/render/symbol/player/{prefix}{player_id}.dds")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_portrait_is_prefixed_up_to_pes_2018_and_bare_from_pes_2019() {
        for version in [
            PesVersion::Pes15,
            PesVersion::Pes16,
            PesVersion::Pes17,
            PesVersion::Pes18,
        ] {
            assert_eq!(
                portrait_path(version, 71405),
                "common/render/symbol/player/player_71405.dds",
                "{version}"
            );
        }
        for version in [PesVersion::Pes19, PesVersion::Pes20, PesVersion::Pes21] {
            assert_eq!(
                portrait_path(version, 71405),
                "common/render/symbol/player/71405.dds",
                "{version}"
            );
        }
    }
}
