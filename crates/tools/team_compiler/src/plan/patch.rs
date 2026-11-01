//! The `aesthetics_patch.toml` planning decides (`spec/team_compiler/pipeline.md` "6. Post-processing",
//! the Fox half): each compiled team export's players' looks at their resolved values, and a
//! `write_names` export's names. `patch_names` is the names rule `check` reports and `compile`
//! builds with, restated from the export alone the way `item_rows::conflicting_id_keys` is.

use std::collections::BTreeMap;

use pes_savefile::interchange::aesthetics_patch::{PatchPlayer, PatchTeam};
use pes_savefile::settings_toml::{AppearanceSettings, NameField, NameSetting, PlayerSettings};
use pes_version::PesVersion;

use super::item_rows::RowPlayer;

/// What the names rule gives of one player folder: the names the patch writes and the facts
/// its findings read.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct PatchNames {
    /// The player's `name` write; `None` when the file gives none or the field refuses it.
    pub(crate) name: Option<String>,
    /// The player's `shirt_name` write, like `name`.
    pub(crate) shirt_name: Option<String>,
    /// Each key whose text the version's field refused (`settings_name_invalid`, the key its
    /// context), `name` then `shirt_name`.
    pub(crate) invalid: Vec<&'static str>,
    /// The folder gives `name` or `shirt_name` and more than one player maps it
    /// (`settings_toml_name_shared`), whether or not a check then drops one.
    pub(crate) shared: bool,
}

/// The names rule (`spec/aesthetics_export/settings_toml.md` "Name-writing rules"), for a kept
/// player folder of a `write_names` team export compiled for Fox. `mapped` is how many players
/// the roster maps it to, `player_name` its folder's name part. `name = true` resolves to
/// `player_name`, `name` or `shirt_name` as a string is written as is, each resolved text
/// checked as `PlayerSettings::apply` checks it (`NameField::check`), a refused one dropped and
/// its key in `invalid`. An export without the marker resolves nothing and reports nothing,
/// whatever the file says; a folder with no `settings.toml` (`settings` `None`, or the
/// defaults when the file was refused or the folder holds only an `ingame_face.toml`) resolves
/// nothing either.
pub(crate) fn patch_names(
    settings: Option<&PlayerSettings>,
    player_name: &str,
    mapped: usize,
    write_names: bool,
    version: PesVersion,
) -> PatchNames {
    let mut names = PatchNames::default();
    let Some(settings) = settings.filter(|_| write_names) else {
        return names;
    };
    names.shared = mapped > 1 && (settings.name.is_some() || settings.shirt_name.is_some());
    let name = settings.name.as_ref().map(|setting| match setting {
        NameSetting::FromFolder => player_name.to_owned(),
        NameSetting::Explicit(name) => name.clone(),
    });
    names.name = checked(NameField::Name, name, version, &mut names.invalid);
    names.shirt_name = checked(
        NameField::ShirtName,
        settings.shirt_name.clone(),
        version,
        &mut names.invalid,
    );
    names
}

/// `text` when the version's field takes it (`field.check`); `None` else, the key noted in
/// `invalid`.
fn checked(
    field: NameField,
    text: Option<String>,
    version: PesVersion,
    invalid: &mut Vec<&'static str>,
) -> Option<String> {
    let text = text?;
    match field.check(&text, version) {
        Ok(()) => Some(text),
        Err(_) => {
            invalid.push(field.key());
            None
        }
    }
}

/// The `[[teams]]` block the compiled player folders `players` of one team export give the
/// patch: `id` the team's, `name` its teams-list name (`/co/`), one `[teams.players.NN]` per
/// player a folder maps — `NN` his player id's slot — `appearance` his joined settings
/// resolved to `version` (the template's defaults when the folder has none), `name`/`shirt_name`
/// the names rule's (`write_names` the export's marker). A folder with a refused settings file
/// (`settings_refused`) gives no look: his entry holds no `[appearance]`, the save keeping
/// his whole look; left with nothing at all — no look, no names, no IDs — he gets no entry.
/// `None` when `players` holds no player with an entry: a patch with no player is no patch,
/// and the export gets no `[[teams]]` entry.
pub(super) fn patch_team(
    players: &[RowPlayer],
    team_id: u16,
    team_name: &str,
    write_names: bool,
    version: PesVersion,
) -> Option<PatchTeam> {
    let mut patch_players = BTreeMap::new();
    for player in players {
        let appearance = if player.settings_refused {
            None
        } else {
            Some(
                player
                    .settings
                    .as_ref()
                    .map_or_else(AppearanceSettings::default, |s| s.appearance.clone())
                    .resolved(version),
            )
        };
        let names = patch_names(
            player.settings.as_ref(),
            &player.player_name,
            player.player_ids.len(),
            write_names,
            version,
        );
        let entry = PatchPlayer {
            name: names.name.clone(),
            shirt_name: names.shirt_name.clone(),
            boots_id: None,
            gloves_id: None,
            base_copy_id: None,
            appearance: appearance.clone(),
        };
        if holds_nothing(&entry) {
            continue;
        }
        for player_id in &player.player_ids {
            patch_players.insert(
                u8::try_from(player_id % 100).expect("a roster slot is under 100"),
                entry.clone(),
            );
        }
    }
    if patch_players.is_empty() {
        return None;
    }
    Some(PatchTeam {
        id: u32::from(team_id),
        name: team_name.to_owned(),
        players: patch_players,
    })
}

/// Whether `entry` holds a key to write: a player the patch gives nothing
/// gets no `[teams.players.NN]` entry.
fn holds_nothing(entry: &PatchPlayer) -> bool {
    entry.name.is_none()
        && entry.shirt_name.is_none()
        && entry.boots_id.is_none()
        && entry.gloves_id.is_none()
        && entry.base_copy_id.is_none()
        && entry.appearance.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `RowPlayer` of one folder mapped to `id` alone: `settings` its joined
    /// files (default when not parsed), `settings_refused` whether one was
    /// refused.
    fn row_player(id: u32, settings_refused: bool) -> RowPlayer {
        RowPlayer {
            path: vtree::ScopePath::new(format!("Players/{id}").as_str()).unwrap(),
            player_ids: vec![id],
            linked: Vec::new(),
            fpc: None,
            player_name: "A".to_owned(),
            settings: Some(PlayerSettings::default()),
            settings_refused,
        }
    }

    /// A player the patch gives nothing — a refused settings file and no
    /// names or IDs for him — gets no `[teams.players.NN]` entry; one the
    /// patch has something for keeps his.
    #[test]
    fn a_player_left_with_nothing_to_write_gets_no_entry() {
        // His settings file was refused and nothing else of his is to be
        // written: no entry, and no `[[teams]]` block when he is the team.
        assert_eq!(
            patch_team(
                &[row_player(71403, true)],
                714,
                "/co/",
                false,
                PesVersion::Pes21
            ),
            None,
            "no player with an entry, no team block"
        );
        // Beside a kept player, the refused folder is the one missing.
        let kept = row_player(71405, false);
        let team = patch_team(
            &[row_player(71403, true), kept],
            714,
            "/co/",
            false,
            PesVersion::Pes21,
        )
        .expect("a team block");
        assert!(
            !team.players.contains_key(&3),
            "no entry for a refused file"
        );
        assert!(
            team.players[&5].appearance.is_some(),
            "the kept player's resolved look"
        );
    }

    /// `patch_names` over one folder's settings, `mapped` players, `write_names` as given.
    fn names(settings: &PlayerSettings, mapped: usize, write_names: bool) -> PatchNames {
        patch_names(
            Some(settings),
            "Snuffy",
            mapped,
            write_names,
            PesVersion::Pes21,
        )
    }

    #[test]
    fn an_export_without_the_marker_resolves_and_reports_nothing() {
        let settings = PlayerSettings::parse("name = \"X\"\nshirt_name = \"Y\"\n").unwrap();
        assert_eq!(names(&settings, 2, false), PatchNames::default());
    }

    #[test]
    fn name_true_resolves_to_the_folder_name_and_explicit_as_written() {
        let settings = PlayerSettings::parse("name = true\n").unwrap();
        let resolved = names(&settings, 1, true);
        assert_eq!(resolved.name.as_deref(), Some("Snuffy"));
        let settings = PlayerSettings::parse("name = \"Ace\"\n").unwrap();
        assert_eq!(names(&settings, 1, true).name.as_deref(), Some("Ace"));
    }

    #[test]
    fn a_name_the_field_refuses_is_dropped_with_its_key() {
        let long = format!("name = \"{}\"\nshirt_name = \"Ā\"\n", "a".repeat(70));
        let settings = PlayerSettings::parse(&long).unwrap();
        let resolved = names(&settings, 1, true);
        assert_eq!(resolved.name, None);
        assert_eq!(resolved.shirt_name, None);
        assert_eq!(resolved.invalid, ["name", "shirt_name"]);
    }

    #[test]
    fn a_folder_mapped_to_several_giving_a_name_is_shared_checked_or_not() {
        let settings = PlayerSettings::parse("name = \"Shared\"\n").unwrap();
        assert!(names(&settings, 2, true).shared);
        assert!(!names(&settings, 1, true).shared);
        // The check drops the name; the folder still gave it for two players.
        let settings = PlayerSettings::parse(&format!("name = \"{}\"\n", "a".repeat(70))).unwrap();
        let resolved = names(&settings, 2, true);
        assert!(resolved.shared);
        assert_eq!(resolved.invalid, ["name"]);
        // No name key at all: nothing shared.
        let settings =
            PlayerSettings::parse("[appearance.motion]\nhunching_dribbling = 2\n").unwrap();
        assert!(!names(&settings, 2, true).shared);
    }
}
