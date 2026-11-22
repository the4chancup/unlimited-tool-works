//! Maps the `libs/fpc` presets onto a player and derives the interference
//! inputs: `apply` writes one preset's appearance, `strip_style` reads the
//! strip settings `fpc::check` reasons over, `is_fpc_player` is the
//! hidden-player test under the settings' hide IDs.

use pes_version::PesVersion;

use crate::codec::CodecError;
use crate::model::player::PlayerEntry;
use crate::schema::ingame_face::IngameFaceField;
use crate::schema::schema_for;

/// An FPC preset could not be applied or read back.
#[derive(Debug, thiserror::Error)]
pub enum FpcError {
    /// The version has no Custom skin colour.
    #[error("{version:?} has no Custom skin color; the partial-hide preset cannot apply")]
    CustomSkinUnavailable {
        /// The player's game version.
        version: PesVersion,
    },
    /// A codec read or write failed (a player never read from a record has no
    /// ingame-face run to read or write through).
    #[error(transparent)]
    Codec(#[from] CodecError),
}

/// Writes one preset's appearance (sleeves, tuck, socks, boots/gloves IDs,
/// skin), all or nothing: every check (custom skin available, the face run
/// present where the version maps one — PES 2014's records carry none, so
/// the presets' skin part has nothing to write there) runs before the
/// first write. The caller substitutes a custom
/// model's own boots/gloves IDs into `appearance` first (the compiler's
/// precedence table). `SkinColor::Custom` writes skin 7 on versions that have
/// a custom skin (`fpc::custom_skin_available`) and is
/// `FpcError::CustomSkinUnavailable` elsewhere; `SkinColor::Preset` resets a
/// skin of 7 to 1 (light) and leaves any other skin alone.
pub fn apply(
    player: &mut PlayerEntry,
    appearance: &fpc::Appearance,
    version: PesVersion,
) -> Result<(), FpcError> {
    if appearance.skin_color == fpc::SkinColor::Custom && !fpc::custom_skin_available(version) {
        return Err(FpcError::CustomSkinUnavailable { version });
    }
    // A probe, not a read: on a version whose records carry the face run it
    // must hold the skin field before the first write, so a player without
    // it errors with nothing changed. PES 2014's records map no run: the
    // strip fields and ids still land on the appearance record's own.
    let face_run = schema_for(version).has_ingame_face();
    if face_run {
        player
            .appearance
            .ingame_face
            .get(IngameFaceField::SkinColor)?;
    }
    player.appearance.sleeves = sleeves_value(appearance.sleeves);
    player.appearance.untucked = untucked_value(appearance.tuck);
    player.appearance.socks = socks_value(appearance.socks);
    player.appearance.boots_id = appearance.boots_id;
    player.appearance.gloves_id = appearance.gloves_id;
    match appearance.skin_color {
        fpc::SkinColor::Custom => player
            .appearance
            .ingame_face
            .set(IngameFaceField::SkinColor, 7)?,
        fpc::SkinColor::Preset => {
            if face_run
                && player
                    .appearance
                    .ingame_face
                    .get(IngameFaceField::SkinColor)?
                    == 7
            {
                player
                    .appearance
                    .ingame_face
                    .set(IngameFaceField::SkinColor, 1)?;
            }
        }
    }
    Ok(())
}

/// A preset's sleeve length as `PlayerField::Sleeves`'s stored value, the
/// `settings.toml` label list's index ("seasonal", "short", "long").
/// `settings_toml` shares these mappings.
pub(crate) fn sleeves_value(sleeves: fpc::Sleeves) -> u8 {
    match sleeves {
        fpc::Sleeves::Short => 1,
        fpc::Sleeves::Long => 2,
    }
}

/// A preset's tuck as `PlayerField::Untucked`'s stored value.
pub(crate) fn untucked_value(tuck: fpc::Tuck) -> bool {
    match tuck {
        fpc::Tuck::Tucked => false,
        fpc::Tuck::Untucked => true,
    }
}

/// A preset's sock length as `PlayerField::Socks`'s stored value, the
/// `settings.toml` label list's index ("standard", "long", "short").
pub(crate) fn socks_value(socks: fpc::Socks) -> u8 {
    match socks {
        fpc::Socks::Standard => 0,
        fpc::Socks::Long => 1,
        fpc::Socks::Short => 2,
    }
}

/// The gloves value a record the game creates on an in-game PES 2014
/// edit stores, all 18 bits of the field set, which the game draws as
/// bare hands like a stored 0.
const NO_GLOVES_MODEL: u32 = 262_143;

/// The gloves model a stored gloves ID means: `NO_GLOVES_MODEL` reads
/// as 0, the bare hands the game draws for it, `stored` itself
/// otherwise. One rule on every version: PES 15 to 21 store the field
/// in 14 bits, too narrow ever to hold the all-bits value.
pub fn gloves_model(stored: u32) -> u32 {
    if stored == NO_GLOVES_MODEL { 0 } else { stored }
}

/// The player's strip settings as the interference inputs. On a version
/// whose records carry the in-game face run the gloves checkbox and the
/// skin are read through it; on one mapping none (PES 2014) the checkbox
/// comes off the appearance record's own bit and the skin counts as
/// preset, `Result` standing for the run reads alone.
pub fn strip_style(player: &PlayerEntry, version: PesVersion) -> Result<fpc::StripStyle, FpcError> {
    let a = &player.appearance;
    let gloves = if schema_for(version).has_ingame_face() {
        a.ingame_face.get(IngameFaceField::PlayerGloves)?
    } else {
        u8::from(a.player_gloves)
    };
    Ok(fpc::StripStyle {
        inners_worn: a.inners != 0,
        undershorts_worn: a.undershorts != 0,
        wrist_taping: a.wrist_taping != 0,
        ankle_taping: a.ankle_taping,
        gloves_checkbox: gloves != 0,
        gloves_id: gloves_model(a.gloves_id),
        skin_color: if schema_for(version).has_ingame_face()
            && a.ingame_face.get(IngameFaceField::SkinColor)? == 7
        {
            fpc::SkinColor::Custom
        } else {
            fpc::SkinColor::Preset
        },
    })
}

/// What the save shows, not what the team intends: a boots ID of `ids`'s
/// (the hide ID the settings give), or — where `version`'s records carry
/// the in-game face run — a custom skin; on a version mapping none (PES
/// 2014) the boots ID alone decides. A hide preset applied with a
/// substituted (real) boots ID is indistinguishable from a dressed
/// player by construction, so this is the save editor's read-only
/// classifier; the compiler, which knows the folder's marker, passes its
/// own `is_fpc_player` to `fpc::check`. `Result` for the skin read where
/// the run exists.
pub fn is_fpc_player(
    player: &PlayerEntry,
    version: PesVersion,
    ids: fpc::HideIds,
) -> Result<bool, FpcError> {
    Ok(player.appearance.boots_id == u32::from(ids.boots())
        || (schema_for(version).has_ingame_face()
            && player
                .appearance
                .ingame_face
                .get(IngameFaceField::SkinColor)?
                == 7))
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;
    use crate::codec::{read_player, runs, write_player};
    use crate::schema::fields::PlayerField;
    use crate::schema::ingame_face::INGAME_FACE_FIELDS;
    use crate::schema::schema_for;
    use crate::test_support::{count, find_player, payload, record};

    fn player_70101(version: PesVersion) -> PlayerEntry {
        find_player(&payload(version), schema_for(version), 70101)
    }

    /// `player` dressed in the other preset's strip with skin 3, so every
    /// field a preset writes starts at another value, and the skin is one
    /// a preset leaves alone.
    fn dressed(player: &mut PlayerEntry, preset: fpc::Preset) {
        let other = fpc::preset(preset, fpc::HideIds::CONVENTION);
        player.appearance.sleeves = sleeves_value(other.sleeves);
        player.appearance.untucked = untucked_value(other.tuck);
        player.appearance.socks = socks_value(other.socks);
        player.appearance.boots_id = other.boots_id;
        player.appearance.gloves_id = other.gloves_id;
        player
            .appearance
            .ingame_face
            .set(IngameFaceField::SkinColor, 3)
            .expect("the skin field is in the run");
    }

    /// The skin a preset left: 3, as `dressed` set it.
    fn skin(player: &PlayerEntry) -> u8 {
        player
            .appearance
            .ingame_face
            .get(IngameFaceField::SkinColor)
            .expect("read run")
    }

    #[test]
    fn the_hide_preset_writes_long_sleeves_and_the_hide_ids() {
        let mut player = player_70101(PesVersion::Pes19);
        dressed(&mut player, fpc::Preset::Unhide);
        assert_ne!(player.appearance.sleeves, 2);
        assert!(player.appearance.untucked);
        assert_ne!(player.appearance.socks, 2);
        assert_ne!(player.appearance.boots_id, 55);
        assert_ne!(player.appearance.gloves_id, 11);
        apply(
            &mut player,
            &fpc::preset(fpc::Preset::Hide, fpc::HideIds::CONVENTION),
            PesVersion::Pes19,
        )
        .expect("apply");
        assert_eq!(player.appearance.sleeves, 2);
        assert!(!player.appearance.untucked);
        assert_eq!(player.appearance.socks, 2);
        assert_eq!(player.appearance.boots_id, 55);
        assert_eq!(player.appearance.gloves_id, 11);
        assert_eq!(skin(&player), 3, "a preset skin keeps a non-custom skin");
        assert!(
            is_fpc_player(&player, PesVersion::Pes19, fpc::HideIds::CONVENTION)
                .expect("is_fpc_player")
        );
    }

    // On a version whose records map no in-game face run a player is an
    // FPC player by his boots id alone: the hide id reads hidden, another
    // does not, and the run is never consulted (PES 2014 has no skin to
    // read).
    #[test]
    fn is_fpc_player_on_pes14_reads_the_boots_id_alone() {
        let mut player = find_player(
            &payload(PesVersion::Pes14),
            schema_for(PesVersion::Pes14),
            42810,
        );
        player.appearance.boots_id = 55;
        assert!(
            is_fpc_player(&player, PesVersion::Pes14, fpc::HideIds::CONVENTION)
                .expect("is_fpc_player")
        );
        player.appearance.boots_id = 54;
        assert!(
            !is_fpc_player(&player, PesVersion::Pes14, fpc::HideIds::CONVENTION)
                .expect("is_fpc_player")
        );
        // And a gloves id alone is no hide either: the pair's boots half
        // names him, not the gloves half.
        player.appearance.boots_id = 0;
        player.appearance.gloves_id = 11;
        assert!(
            !is_fpc_player(&player, PesVersion::Pes14, fpc::HideIds::CONVENTION)
                .expect("is_fpc_player")
        );
    }

    // The classifier reads the hide IDs it is given, not the convention:
    // under IDs (60, 12) boots 60 is hidden and the convention's 55 is not.
    #[test]
    fn is_fpc_player_reads_the_given_hide_ids() {
        let ids = fpc::HideIds::new(60, 12).expect("in band");
        let mut player = player_70101(PesVersion::Pes19);
        player.appearance.boots_id = 60;
        assert!(is_fpc_player(&player, PesVersion::Pes19, ids).expect("is_fpc_player"));
        player.appearance.boots_id = 55;
        assert!(!is_fpc_player(&player, PesVersion::Pes19, ids).expect("is_fpc_player"));
    }

    #[test]
    fn the_unhide_preset_restores_the_stock_strip() {
        let mut player = player_70101(PesVersion::Pes19);
        dressed(&mut player, fpc::Preset::Hide);
        assert_ne!(player.appearance.sleeves, 1);
        assert!(!player.appearance.untucked);
        assert_ne!(player.appearance.socks, 0);
        assert_ne!(player.appearance.boots_id, 0);
        assert_ne!(player.appearance.gloves_id, 0);
        apply(
            &mut player,
            &fpc::preset(fpc::Preset::Unhide, fpc::HideIds::CONVENTION),
            PesVersion::Pes19,
        )
        .expect("apply");
        assert_eq!(player.appearance.sleeves, 1);
        assert!(player.appearance.untucked);
        assert_eq!(player.appearance.socks, 0);
        assert_eq!(player.appearance.boots_id, 0);
        assert_eq!(player.appearance.gloves_id, 0);
        assert_eq!(skin(&player), 3, "a preset skin keeps a non-custom skin");
        assert!(
            !is_fpc_player(&player, PesVersion::Pes19, fpc::HideIds::CONVENTION)
                .expect("is_fpc_player")
        );
    }

    // On PES 2014 the presets write the ids as on the other versions (the
    // hide preset's 55/11 pair, `the_hide_preset_writes_long_sleeves_and_the_hide_ids`
    // and `the_unhide_preset_restores_the_stock_strip` on PES 19): the
    // version's records carry no in-game face run, so the presets' skin
    // part has nothing to read or write, and the strip fields and ids all
    // land on the appearance record's own runs.
    #[test]
    fn the_presets_write_the_ids_on_pes14() {
        let mut player = find_player(
            &payload(PesVersion::Pes14),
            schema_for(PesVersion::Pes14),
            42810,
        );
        // A strip other than either preset's, so every write shows.
        player.appearance.sleeves = 0;
        player.appearance.untucked = true;
        player.appearance.socks = 1;
        player.appearance.boots_id = 0;
        player.appearance.gloves_id = 0;
        apply(
            &mut player,
            &fpc::preset(fpc::Preset::Hide, fpc::HideIds::CONVENTION),
            PesVersion::Pes14,
        )
        .expect("apply");
        assert_eq!(player.appearance.sleeves, 2);
        assert!(!player.appearance.untucked);
        assert_eq!(player.appearance.socks, 2);
        assert_eq!(player.appearance.boots_id, 55);
        assert_eq!(player.appearance.gloves_id, 11);
        apply(
            &mut player,
            &fpc::preset(fpc::Preset::Unhide, fpc::HideIds::CONVENTION),
            PesVersion::Pes14,
        )
        .expect("apply");
        assert_eq!(player.appearance.sleeves, 1);
        assert!(player.appearance.untucked);
        assert_eq!(player.appearance.socks, 0);
        assert_eq!(player.appearance.boots_id, 0);
        assert_eq!(player.appearance.gloves_id, 0);
    }

    #[test]
    fn partial_hide_needs_a_version_with_custom_skin() {
        let mut player = player_70101(PesVersion::Pes19);
        let before = player.clone();
        match apply(
            &mut player,
            &fpc::preset(fpc::Preset::PartialHide, fpc::HideIds::CONVENTION),
            PesVersion::Pes19,
        ) {
            Err(FpcError::CustomSkinUnavailable { version }) => {
                assert_eq!(version, PesVersion::Pes19)
            }
            other => panic!("expected CustomSkinUnavailable, got {other:?}"),
        }
        assert_eq!(player, before, "the refused preset wrote nothing");

        let mut player = player_70101(PesVersion::Pes16);
        apply(
            &mut player,
            &fpc::preset(fpc::Preset::PartialHide, fpc::HideIds::CONVENTION),
            PesVersion::Pes16,
        )
        .expect("apply");
        assert_eq!(
            player
                .appearance
                .ingame_face
                .get(IngameFaceField::SkinColor)
                .expect("read run"),
            7
        );
        assert!(
            is_fpc_player(&player, PesVersion::Pes16, fpc::HideIds::CONVENTION)
                .expect("is_fpc_player")
        );
    }

    #[test]
    fn a_preset_skin_resets_a_custom_skin_to_light() {
        let mut player = player_70101(PesVersion::Pes16);
        player
            .appearance
            .ingame_face
            .set(IngameFaceField::SkinColor, 7)
            .expect("set");
        apply(
            &mut player,
            &fpc::preset(fpc::Preset::Unhide, fpc::HideIds::CONVENTION),
            PesVersion::Pes16,
        )
        .expect("apply");
        assert_eq!(
            player
                .appearance
                .ingame_face
                .get(IngameFaceField::SkinColor)
                .expect("read run"),
            1
        );
    }

    // On PES 2014 `strip_style` reads the gloves checkbox off the
    // appearance record's own bit — the version maps no face run to take
    // it from — and takes the skin for a preset one, none being stored.
    #[test]
    fn strip_style_on_pes14_reads_the_record_bit_and_a_preset_skin() {
        let mut player = find_player(
            &payload(PesVersion::Pes14),
            schema_for(PesVersion::Pes14),
            42810,
        );
        player.appearance.player_gloves = true;
        let style = strip_style(&player, PesVersion::Pes14).expect("strip_style");
        assert!(style.gloves_checkbox);
        assert_eq!(style.skin_color, fpc::SkinColor::Preset);
        player.appearance.player_gloves = false;
        assert!(
            !strip_style(&player, PesVersion::Pes14)
                .expect("strip_style")
                .gloves_checkbox
        );
    }

    #[test]
    fn strip_style_feeds_the_interference_check() {
        let mut player = player_70101(PesVersion::Pes19);
        player.appearance.inners = 1;
        let findings = fpc::check(
            &strip_style(&player, PesVersion::Pes19).expect("strip_style"),
            true,
        );
        assert_eq!(findings[0].code, "fpc_inners_break_hiding");

        let mut player = player_70101(PesVersion::Pes19);
        player
            .appearance
            .ingame_face
            .set(IngameFaceField::PlayerGloves, 1)
            .expect("set");
        player.appearance.gloves_id = 0;
        let findings = fpc::check(
            &strip_style(&player, PesVersion::Pes19).expect("strip_style"),
            false,
        );
        assert_eq!(findings[0].code, "fpc_gloves_checkbox_winter_gloves");
    }

    #[test]
    fn strip_style_reads_the_booleans_and_the_skin_in_both_directions() {
        let mut player = player_70101(PesVersion::Pes19);
        player.appearance.undershorts = 0;
        player.appearance.wrist_taping = 0;
        player
            .appearance
            .ingame_face
            .set(IngameFaceField::SkinColor, 1)
            .expect("set");
        let style = strip_style(&player, PesVersion::Pes19).expect("strip_style");
        assert!(!style.undershorts_worn);
        assert!(!style.wrist_taping);
        assert_eq!(style.skin_color, fpc::SkinColor::Preset);

        player.appearance.undershorts = 2;
        player.appearance.wrist_taping = 3;
        player
            .appearance
            .ingame_face
            .set(IngameFaceField::SkinColor, 7)
            .expect("set");
        let style = strip_style(&player, PesVersion::Pes19).expect("strip_style");
        assert!(style.undershorts_worn);
        assert!(style.wrist_taping);
        assert_eq!(style.skin_color, fpc::SkinColor::Custom);
    }

    // A stored gloves ID wider than the save's other versions' 14 bits is
    // carried as decoded, not refused: PES 14 stores it in 18.
    #[test]
    fn strip_style_carries_a_gloves_id_past_u16() {
        let mut player = player_70101(PesVersion::Pes21);
        player.appearance.gloves_id = 70_000;
        let style = strip_style(&player, PesVersion::Pes21).expect("strip_style");
        assert_eq!(style.gloves_id, 70_000);
    }

    // A record the game creates on an in-game edit stores the gloves
    // id's 18 bits all set (262143), which the game draws as bare hands
    // like a stored 0: `strip_style` reads it as gloves 0, so the
    // interference check reports what it reports for gloves 0.
    #[test]
    fn strip_style_reads_a_stored_all_bits_gloves_id_as_none() {
        let mut player = find_player(
            &payload(PesVersion::Pes14),
            schema_for(PesVersion::Pes14),
            42810,
        );
        player.appearance.gloves_id = 262_143;
        player.appearance.player_gloves = true;
        let style = strip_style(&player, PesVersion::Pes14).expect("strip_style");
        assert_eq!(style.gloves_id, 0);

        player.appearance.gloves_id = 0;
        let at_zero = fpc::check(
            &strip_style(&player, PesVersion::Pes14).expect("strip_style"),
            false,
        );
        player.appearance.gloves_id = 262_143;
        let at_all_bits = fpc::check(
            &strip_style(&player, PesVersion::Pes14).expect("strip_style"),
            false,
        );
        assert!(
            !at_zero.is_empty(),
            "premise: gloves 0 with the checkbox on reports"
        );
        assert_eq!(at_all_bits, at_zero);
    }

    #[test]
    fn apply_on_an_entry_never_read_is_an_error_and_changes_nothing() {
        let mut player = PlayerEntry::default();
        match apply(
            &mut player,
            &fpc::preset(fpc::Preset::Hide, fpc::HideIds::CONVENTION),
            PesVersion::Pes19,
        ) {
            Err(FpcError::Codec(CodecError::NoIngameFaceRun { .. })) => {}
            other => panic!("expected Codec(NoIngameFaceRun), got {other:?}"),
        }
        assert_eq!(player, PlayerEntry::default());
    }

    #[test]
    fn an_apply_then_write_touches_only_the_preset_fields_bits() {
        let payload = payload(PesVersion::Pes19);
        let schema = schema_for(PesVersion::Pes19);
        let mut player = find_player(&payload, schema, 70101);
        apply(
            &mut player,
            &fpc::preset(fpc::Preset::Hide, fpc::HideIds::CONVENTION),
            PesVersion::Pes19,
        )
        .expect("apply");

        let index = (0..count(&payload, &schema.players))
            .find(|&i| {
                read_player(
                    schema.version,
                    record(&payload, &schema.players, schema.player.size, i),
                    schema.player,
                )
                .expect("decode")
                .id == 70101
            })
            .expect("player 70101's record");
        let rec = record(&payload, &schema.players, schema.player.size, index);
        let mut written = rec.to_vec();
        write_player(schema.version, &player, &mut written, schema.player).expect("encode");

        let run = schema.player.ingame_face.as_ref().expect("a run on PES 19");
        let skin = INGAME_FACE_FIELDS
            .iter()
            .find(|f| f.field == IngameFaceField::SkinColor)
            .expect("a skin row");
        let mut ranges = vec![(
            run.byte_offset * 8 + skin.bit_offset,
            run.byte_offset * 8 + skin.bit_offset + skin.bit_width,
        )];
        for field in [
            PlayerField::Sleeves,
            PlayerField::Untucked,
            PlayerField::Socks,
            PlayerField::BootsId,
            PlayerField::GlovesId,
        ] {
            let found = ranges.len();
            for (f, offset, width) in runs(schema.player) {
                if f == field {
                    ranges.push((offset, offset + width));
                }
            }
            assert!(ranges.len() > found, "a row for {field:?}");
        }
        for (byte, (was, now)) in rec.iter().zip(&written).enumerate() {
            for bit in 0..8 {
                if (was >> bit) & 1 != (now >> bit) & 1 {
                    let pos = byte as u32 * 8 + bit;
                    assert!(
                        ranges.iter().any(|(s, e)| pos >= *s && pos < *e),
                        "bit {pos} changed outside the preset fields"
                    );
                }
            }
        }
    }
}
