use super::*;

use crate::file::EditFile;
use crate::model::player::{PlayerEntry, PlayerMotion};
use crate::schema::fields::PlayerField;
use crate::settings_toml::{MotionSettings, PlayerSettings};
use crate::test_support::{FIXTURES, count, open, record, test_salt};

/// A patch at `version` over `teams`.
fn patch_at(version: PesVersion, teams: Vec<PatchTeam>) -> AestheticsPatch {
    AestheticsPatch {
        pes_version: version,
        tool_version: "0.1.0".to_string(),
        allocation_scheme_version: ALLOCATION_SCHEME_VERSION,
        cpks: vec!["4cc_69.cpk".to_string()],
        compiled: "2026-09-12T14:03:00Z".parse().expect("a datetime"),
        teams,
    }
}

/// An entry holding nothing: every key `None`, the save's player kept whole.
fn bare() -> PatchPlayer {
    PatchPlayer {
        name: None,
        shirt_name: None,
        boots_id: None,
        gloves_id: None,
        base_copy_id: None,
        appearance: None,
    }
}

/// A player with `name` alone.
fn named(name: &str) -> PatchPlayer {
    PatchPlayer {
        name: Some(name.to_string()),
        ..bare()
    }
}

/// A player with `appearance` alone.
fn look(appearance: AppearanceSettings) -> PatchPlayer {
    PatchPlayer {
        appearance: Some(appearance),
        ..bare()
    }
}

/// A `[[teams]]` block; `name` is a reader aid `apply` never uses.
fn team(id: u32, players: BTreeMap<u8, PatchPlayer>) -> PatchTeam {
    PatchTeam {
        id,
        name: String::new(),
        players,
    }
}

mod apply;
mod format;
