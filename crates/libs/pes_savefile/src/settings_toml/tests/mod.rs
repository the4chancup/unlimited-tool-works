//! The `PlayerSettings` tests: the completeness invariant, `from_player`,
//! `apply` and the key table itself (`model`), the `PlayerAppearance.bin`
//! row (`appearance_row`), the per-version narrowing (`narrow`), the marker
//! preset and the strip-style check inputs (`fpc`), and the TOML text half
//! (`document`).

mod appearance_row;
mod document;
mod fpc;
mod model;
mod narrow;

use std::collections::HashSet;

use pes_version::PesVersion;
use toml_edit::DocumentMut;

use super::keys::Source;
use super::{
    AppearanceSettings, IngameFaceSettings, KeyTable, Kind, NameField, NameSetting, NarrowedValue,
    Ownership, PlayerSettings, RefusedValue, SettingKey, SettingsError, appearance_from,
    face_ownership, get_appearance, held_on, ownership,
};
use crate::codec::{CodecError, read_player, runs, text_max, write_player};
use crate::model::ingame_face::IngameFace;
use crate::model::ingame_face::in_layout;
use crate::model::player::PlayerAppearance;
use crate::schema::fields::{PlayerField, PlayerText};
use crate::schema::ingame_face::{
    FaceLayout, INGAME_FACE_FIELDS, IngameFaceField, UNUSED_FACE_BITS,
};
use crate::schema::schema_for;
use crate::test_support::{count, find_player, open, payload, record};

/// The `schema.players` index of the record whose decoded `id` is `id`.
fn player_record_index(payload: &[u8], schema: &crate::schema::VersionSchema, id: u32) -> usize {
    (0..count(payload, &schema.players))
        .find(|&i| {
            read_player(
                record(payload, &schema.players, schema.player.size, i),
                schema.player,
            )
            .expect("decode")
            .id == id
        })
        .expect("the player's record")
}
