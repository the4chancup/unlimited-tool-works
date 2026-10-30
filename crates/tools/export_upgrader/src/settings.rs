//! Step 8's `settings.toml` files (`spec/export_upgrader.md` step 8): each
//! rostered player's look as the save records it, generated through the same
//! `PlayerSettings` the save editor writes, one file per output player folder;
//! a marked folder's `ingame_face.toml` beside it, the in-game face's keys the
//! `settings.toml` does not hold. `from_player`'s output is adjusted for what the export's folders
//! already decide, a line each: a boots or gloves folder of his worn ID
//! claims the category, so its key is left out whatever the record holds; a
//! stored 0 stays unset in a Full output but is written `0` in a Midcup one,
//! where an unset key keeps the installed ID, or under `fpc_on`, where an
//! unset key gives the hide preset's ID; and name writing is opt-in, so
//! the file gets `name = true` only when the folder-derived name equals the
//! raw save name (a derived name cannot carry colour codes). A value no
//! `settings.toml` can hold is left out and reported, not fatal. Loose mode
//! (step 11) has no records: `loose_settings_files` writes a named folder's
//! `name = true` alone.

use std::collections::BTreeMap;

use pes_savefile::EditFile;
use pes_savefile::settings_toml::{NameSetting, PlayerSettings};
use studio_core::{Disposition, Message, Scope};

use crate::messages::{Code, INPUT, tool_message};
use crate::players::OutputFolder;
use crate::roster::Roster;

/// Step 8's `settings.toml` files and findings.
#[derive(Debug, Default)]
pub(crate) struct SettingsFiles {
    /// Output path (`Players/15 - Snuffy/settings.toml`) → the file's bytes.
    pub(crate) written: Vec<(String, Vec<u8>)>,
    /// The findings, in slot order.
    pub(crate) findings: Vec<Message>,
}

/// The step-8 plan for `roster`'s slots: one `settings.toml` per output
/// folder in `folders`, and the findings each slot's decisions report.
/// `full` is the output's coverage tag: only a Full output reports a
/// folderless player's missing custom model.
pub(crate) fn settings_files(
    save: &EditFile,
    roster: &Roster,
    folders: &BTreeMap<u8, OutputFolder>,
    full: bool,
) -> SettingsFiles {
    let mut files = SettingsFiles::default();
    for (nn, player) in &roster.0 {
        let slot = format!("{nn:02}");
        let record = save
            .player(player.id)
            .expect("the roster read his record from the save");
        let folder = folders.get(nn);
        // A worn custom ID no migrated folder claims is missing content:
        // checked for every rostered player of a Full output but in a
        // Midcup one only for a player the output gives a folder (a Midcup
        // compile leaves a folderless player as installed).
        for (key, worn, claimed) in [
            (
                "boots_id",
                player.boots_id,
                folder.is_some_and(|folder| folder.boots_folder),
            ),
            (
                "gloves_id",
                player.gloves_id,
                folder.is_some_and(|folder| folder.gloves_folder),
            ),
        ] {
            if worn > 100 && !claimed && (full || folder.is_some()) {
                files.findings.push(tool_message(
                    Code::CustomModelMissing,
                    Scope::Export { export_id: INPUT },
                    Disposition::Keep,
                    vec![
                        ("slot", slot.clone()),
                        ("name", player.name.clone()),
                        ("key", key.to_owned()),
                        ("id", worn.to_string()),
                    ],
                ));
            }
        }
        let Some(folder) = folder else {
            continue;
        };
        // The generation the save editor shares: a value past its key's
        // widest range is left out and reported, any other failure loses
        // the file.
        let (mut settings, refused) = match PlayerSettings::from_player_skipping(record) {
            Ok(generated) => generated,
            Err(error) => {
                files.findings.push(tool_message(
                    Code::SettingsNotGenerated,
                    Scope::Export { export_id: INPUT },
                    Disposition::Keep,
                    vec![
                        ("slot", slot),
                        ("name", player.name.clone()),
                        ("error", error.to_string()),
                    ],
                ));
                continue;
            }
        };
        for value in &refused {
            files.findings.push(tool_message(
                Code::SettingsValueRefused,
                Scope::Export { export_id: INPUT },
                Disposition::Keep,
                vec![
                    ("slot", slot.clone()),
                    ("name", player.name.clone()),
                    ("key", value.key.clone()),
                    ("value", value.value.to_string()),
                ],
            ));
        }
        // `from_player` gives a stock ID (1 to 100) or `None`. A folder of
        // the category decides it instead; else a stored 0 is written in a
        // Midcup output, where an unset key keeps the installed ID, or
        // under `fpc_on`, where an unset key gives the hide preset's
        // blank-model ID (step 8).
        for (claimed, stored, field) in [
            (
                folder.boots_folder,
                record.appearance.boots_id,
                &mut settings.boots_id,
            ),
            (
                folder.gloves_folder,
                record.appearance.gloves_id,
                &mut settings.gloves_id,
            ),
        ] {
            if claimed {
                *field = None;
            } else if stored == 0 && (!full || folder.fpc_on) {
                *field = Some(0);
            }
        }
        // `name = true` writes the name the compiler derives from the
        // folder name; the explicit string when they differ, the only
        // spelling that can carry colour codes.
        let folder_name = folder.path.rsplit('/').next().unwrap_or(&folder.path);
        settings.name = if record.name == aesthetics_export::folder_player_name(folder_name) {
            Some(NameSetting::FromFolder)
        } else {
            Some(NameSetting::Explicit(record.name.clone()))
        };
        match settings.to_toml() {
            Ok(text) => {
                files
                    .written
                    .push((format!("{}/settings.toml", folder.path), text.into_bytes()));
                // The in-game face's keys the `settings.toml` does not hold go
                // to the file that marks it, filled from the same settings
                // (a marked folder's `ingame_face.toml`; a failed generation
                // leaves the marker's empty file).
                if folder.ingame_face {
                    let face = settings
                        .ingame_face()
                        .to_toml()
                        .expect("every stored value parses back: `from_player_skipping` left the refused unset");
                    files.written.push((
                        format!("{}/ingame_face.toml", folder.path),
                        face.into_bytes(),
                    ));
                }
            }
            Err(error) => files.findings.push(tool_message(
                Code::SettingsNotGenerated,
                Scope::Export { export_id: INPUT },
                Disposition::Keep,
                vec![
                    ("slot", slot),
                    ("name", player.name.clone()),
                    ("error", error.to_string()),
                ],
            )),
        }
    }
    files
}

/// Loose mode's `settings.toml` files (step 11): step 8's generation needs
/// the save's records, which loose mode has none of, but `name` handling
/// needs none — a folder whose name has a name part gets a file holding
/// `name = true` alone, one named by its number alone none. No findings.
pub(crate) fn loose_settings_files(folders: &BTreeMap<u8, OutputFolder>) -> SettingsFiles {
    let mut files = SettingsFiles::default();
    for folder in folders.values() {
        if !folder.named {
            continue;
        }
        let text = PlayerSettings {
            name: Some(NameSetting::FromFolder),
            ..PlayerSettings::default()
        }
        .to_toml()
        .expect("a name-only PlayerSettings has no value to refuse");
        files
            .written
            .push((format!("{}/settings.toml", folder.path), text.into_bytes()));
    }
    files
}

#[cfg(test)]
mod tests {
    use pes_savefile::settings_toml::IngameFaceSettings;
    use pes_version::PesVersion;
    use teams_list::TeamId;

    use super::*;

    /// The PES 21 fixture save with `edit` applied to its model.
    fn save_edited(edit: impl FnOnce(&mut EditFile)) -> EditFile {
        let mut file = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21))
            .expect("the fixture save loads");
        edit(&mut file);
        file
    }

    /// `/co/`'s roster read from `save` (team 714).
    fn roster_of(save: &EditFile) -> Roster {
        Roster::read(save, TeamId::new(714).unwrap())
    }

    /// A slot's output folder at `path` (`Players/NN …`), claiming neither
    /// category, named by its number alone.
    fn plain(path: &str) -> OutputFolder {
        OutputFolder {
            path: path.to_owned(),
            named: false,
            boots_folder: false,
            gloves_folder: false,
            fpc_on: false,
            ingame_face: false,
        }
    }

    /// The `settings.toml` written for the folder `path`, parsed back.
    fn settings_of(files: &SettingsFiles, path: &str) -> PlayerSettings {
        let wanted = format!("{path}/settings.toml");
        let matching: Vec<_> = files
            .written
            .iter()
            .filter(|(written, _)| *written == wanted)
            .collect();
        assert_eq!(matching.len(), 1, "one settings.toml for {path}");
        PlayerSettings::parse(std::str::from_utf8(&matching[0].1).unwrap())
            .expect("the file parses")
    }

    fn findings_named<'a>(files: &'a SettingsFiles, code: &str) -> Vec<&'a Message> {
        files
            .findings
            .iter()
            .filter(|message| message.code.code.as_ref() == code)
            .collect()
    }

    #[test]
    fn a_stock_id_is_kept_and_a_claimed_one_left_out() {
        let save = save_edited(|file| {
            file.player_mut(71405).unwrap().appearance.boots_id = 12;
            file.player_mut(71406).unwrap().appearance.boots_id = 12;
        });
        let roster = roster_of(&save);
        let mut folders = BTreeMap::new();
        folders.insert(5, plain("Players/05 - A"));
        folders.insert(
            6,
            OutputFolder {
                path: "Players/06 - B".to_owned(),
                named: true,
                boots_folder: true,
                gloves_folder: false,
                fpc_on: false,
                ingame_face: false,
            },
        );
        let files = settings_files(&save, &roster, &folders, true);
        assert_eq!(settings_of(&files, "Players/05 - A").boots_id, Some(12));
        assert_eq!(settings_of(&files, "Players/06 - B").boots_id, None);
    }

    #[test]
    fn a_stored_zero_is_unset_full_and_written_midcup() {
        let save = save_edited(|file| {
            file.player_mut(71405).unwrap().appearance.gloves_id = 0;
        });
        let roster = roster_of(&save);
        let mut folders = BTreeMap::new();
        folders.insert(5, plain("Players/05 - A"));
        let files = settings_files(&save, &roster, &folders, true);
        assert_eq!(settings_of(&files, "Players/05 - A").gloves_id, None);
        let files = settings_files(&save, &roster, &folders, false);
        assert_eq!(settings_of(&files, "Players/05 - A").gloves_id, Some(0));
    }

    #[test]
    fn a_custom_id_no_folder_claims_is_reported_as_full_allows() {
        let save = save_edited(|file| {
            file.player_mut(71405).unwrap().appearance.boots_id = 433;
            file.player_mut(71406).unwrap().appearance.boots_id = 433;
            // The stock band's top, a game model: not custom content.
            file.player_mut(71407).unwrap().appearance.boots_id = 100;
        });
        let roster = roster_of(&save);
        // 05 gets a folder that claims no boots; 06 and 07 get none.
        let mut folders = BTreeMap::new();
        folders.insert(5, plain("Players/05 - A"));

        let files = settings_files(&save, &roster, &folders, true);
        let missing = findings_named(&files, "custom_model_missing");
        assert_eq!(missing.len(), 2, "{:?}", files.findings);
        let slots: Vec<&str> = missing
            .iter()
            .map(|message| {
                message
                    .context
                    .iter()
                    .find(|(key, _)| key == "slot")
                    .unwrap()
                    .1
                    .as_str()
            })
            .collect();
        assert_eq!(slots, ["05", "06"], "every rostered player is checked");
        assert_eq!(
            settings_of(&files, "Players/05 - A").boots_id,
            None,
            "the missing custom ID is not written"
        );

        // Midcup: only the player the output gives a folder is checked.
        let files = settings_files(&save, &roster, &folders, false);
        let missing = findings_named(&files, "custom_model_missing");
        assert_eq!(missing.len(), 1, "{:?}", files.findings);
        assert_eq!(
            missing[0]
                .context
                .iter()
                .find(|(key, _)| key == "slot")
                .unwrap()
                .1,
            "05"
        );

        // No folder at all in Midcup: nothing reported, nothing written.
        let files = settings_files(&save, &roster, &BTreeMap::new(), false);
        assert!(findings_named(&files, "custom_model_missing").is_empty());
        assert!(files.written.is_empty());
    }

    #[test]
    fn the_name_is_from_folder_only_when_it_derives_to_the_save_name() {
        let save = save_edited(|_| {});
        let roster = roster_of(&save);
        let raw = save.player(71405).unwrap().name.clone();
        let mut folders = BTreeMap::new();
        // His folder named as his save name: `name = true`.
        folders.insert(5, plain(&format!("Players/05 - {raw}")));
        // A folder named by its number alone: the explicit save name.
        folders.insert(7, plain("Players/07"));
        let files = settings_files(&save, &roster, &folders, true);
        assert_eq!(
            settings_of(&files, &format!("Players/05 - {raw}")).name,
            Some(NameSetting::FromFolder)
        );
        assert_eq!(
            settings_of(&files, "Players/07").name,
            Some(NameSetting::Explicit(raw))
        );
    }

    /// The `ingame_face.toml` written for the marked folder `path`, parsed
    /// back; panics when the path was written twice or not at all.
    fn face_of(files: &SettingsFiles, path: &str) -> IngameFaceSettings {
        let wanted = format!("{path}/ingame_face.toml");
        let matching: Vec<_> = files
            .written
            .iter()
            .filter(|(written, _)| *written == wanted)
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "one write of {wanted}: {:?}",
            files.written
        );
        IngameFaceSettings::parse(std::str::from_utf8(&matching[0].1).unwrap())
            .expect("the file parses")
    }

    /// A folder marked `ingame_face` (`OutputFolder::ingame_face`) gets one
    /// `ingame_face.toml` write beside its `settings.toml`, the record's face,
    /// hair and iris filling it — never an empty file over the filled one.
    #[test]
    fn a_marked_folder_gets_the_filled_ingame_face_toml_once() {
        let save = save_edited(|_| {});
        let roster = roster_of(&save);
        let mut folders = BTreeMap::new();
        folders.insert(
            5,
            OutputFolder {
                path: "Players/05 - A".to_owned(),
                ingame_face: true,
                ..plain("Players/05 - A")
            },
        );
        let files = settings_files(&save, &roster, &folders, true);
        assert_eq!(
            face_of(&files, "Players/05 - A"),
            PlayerSettings::from_player(save.player(71405).unwrap())
                .unwrap()
                .ingame_face(),
            "the record's face, hair and iris"
        );
        assert_eq!(
            settings_of(&files, "Players/05 - A").appearance.iris_color,
            None,
            "settings.toml holds none of the face keys"
        );
    }

    #[test]
    fn loose_settings_name_a_named_folder_alone() {
        // Step 11: a folder whose name has a name part gets a file holding
        // `name = true` alone; one named by its number alone gets none.
        let mut folders = BTreeMap::new();
        folders.insert(
            3,
            OutputFolder {
                path: "Players/03 - Ann".to_owned(),
                named: true,
                boots_folder: false,
                gloves_folder: false,
                fpc_on: false,
                ingame_face: false,
            },
        );
        folders.insert(5, plain("Players/05"));
        let files = loose_settings_files(&folders);
        assert_eq!(files.written.len(), 1, "{:?}", files.written);
        assert_eq!(files.written[0].0, "Players/03 - Ann/settings.toml");
        let parsed = PlayerSettings::parse(std::str::from_utf8(&files.written[0].1).unwrap())
            .expect("the file parses");
        assert_eq!(
            parsed,
            PlayerSettings {
                name: Some(NameSetting::FromFolder),
                ..PlayerSettings::default()
            }
        );
        assert!(files.findings.is_empty());
    }
}
