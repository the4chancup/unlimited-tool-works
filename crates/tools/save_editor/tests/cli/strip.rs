//! `strip-and-seed` and `check-stripped` (`spec/save_editor.md` "Stripped save"): the Fox
//! player tables seeded from a save's players, every appearance id set to -1, and the
//! restored-id listing and re-strip.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use cpk::{CpkArchive, CpkTimestamp, CpkWriter};
use pes_savefile::EditFile;
use pes_savefile::model::player::STRIPPED_APPEARANCE_ID;
use pes_savefile::player_tables::{AppearanceTable, ItemList, PlayerTableFile};
use pes_version::PesVersion;

use crate::common::Run;
use crate::{Sandbox, TEST_SALT, WritingCommand, payload, payload_after};

/// `UniColor.bin`'s path in the CPK: the unrelated file the tests' archives carry.
const UNI_COLOR: &str = "common/character0/model/character/uniform/team/UniColor.bin";

/// The timestamp the tests' CPK entries carry.
const WHEN: CpkTimestamp = CpkTimestamp {
    year: 2026,
    month: 10,
    day: 14,
    hour: 12,
    minute: 0,
    second: 0,
};

/// `BootsList.bin`/`GloveList.bin` pair value of `item` (the item ID little-endian).
fn pair(item: u32) -> [u8; 4] {
    item.to_le_bytes()
}

impl Sandbox {
    /// `strip-and-seed --base <base> --bins <bins> --out <out> --bins-out <bins_out>`.
    fn strip_and_seed(&self, base: &str, bins: &str, out: &str, bins_out: &str) -> Run {
        self.run(&[
            "strip-and-seed",
            "--base",
            &self.arg(base),
            "--bins",
            &self.arg(bins),
            "--out",
            &self.arg(out),
            "--bins-out",
            &self.arg(bins_out),
        ])
    }

    /// `check-stripped <save>` with the given extra arguments.
    fn check_stripped(&self, save: &str, extra: &[&str]) -> Run {
        let save = self.arg(save);
        let mut args = vec!["check-stripped", save.as_str()];
        args.extend_from_slice(extra);
        self.run(&args)
    }
}

/// Writes `version`'s fixture save at `relative` with `stripped`'s appearance id at -1,
/// the way Edit mode's strip leaves it, and returns the save's model for the assertions.
fn pre_stripped_fixture(
    sandbox: &Sandbox,
    version: PesVersion,
    relative: &str,
    stripped: u32,
) -> EditFile {
    let mut file = EditFile::from_bytes(&pes_savefile::fixture_save(version)).unwrap();
    file.player_mut(stripped).unwrap().appearance_id = Some(STRIPPED_APPEARANCE_ID);
    fs::write(sandbox.path(relative), file.to_bytes(&TEST_SALT).unwrap()).unwrap();
    file
}

/// Writes `entries` (CPK path, bytes) at `path` as a CPK, every entry timestamped `WHEN`.
fn write_cpk(path: &Path, entries: &[(&str, Vec<u8>)]) {
    let mut writer = CpkWriter::new(Cursor::new(Vec::new()), "test").unwrap();
    for (entry_path, bytes) in entries {
        writer.add(entry_path, bytes, Some(WHEN)).unwrap();
    }
    fs::write(path, writer.finish().unwrap().into_inner()).unwrap();
}

/// The CPK at `path`, opened over its bytes.
fn read_cpk(path: &Path) -> CpkArchive<Cursor<Vec<u8>>> {
    CpkArchive::open(Cursor::new(fs::read(path).unwrap())).unwrap()
}

/// `archive`'s copy of `file`'s table bytes, when the CPK holds it.
fn table_bytes(
    archive: &mut CpkArchive<Cursor<Vec<u8>>>,
    file: PlayerTableFile,
) -> Option<Vec<u8>> {
    let entry = archive
        .entries()
        .iter()
        .find(|entry| entry.path == file.cpk_path())
        .cloned()?;
    Some(archive.read(&entry).unwrap())
}

/// Asserts the CPK at `bins_out` holds, for every player of `input` but `stripped`, his
/// record's `PlayerAppearance.bin` row and his boots and gloves ids as the list pairs, and
/// `stripped`'s three rows exactly as `kept` gives them (`None`: no row for him).
fn assert_seeded(
    bins_out: &Path,
    input: &EditFile,
    stripped: u32,
    kept: Option<([u8; 56], [u8; 4], [u8; 4])>,
) {
    let mut archive = read_cpk(bins_out);
    let appearance = AppearanceTable::read(
        PlayerTableFile::PlayerAppearance,
        &table_bytes(&mut archive, PlayerTableFile::PlayerAppearance).unwrap(),
    )
    .unwrap();
    let boots = ItemList::read(
        PlayerTableFile::BootsList,
        &table_bytes(&mut archive, PlayerTableFile::BootsList).unwrap(),
    )
    .unwrap();
    let gloves = ItemList::read(
        PlayerTableFile::GloveList,
        &table_bytes(&mut archive, PlayerTableFile::GloveList).unwrap(),
    )
    .unwrap();
    for player in input.players() {
        if player.id == stripped {
            continue;
        }
        let row = input.appearance_row(player.id).unwrap();
        assert_eq!(
            appearance.get(player.id).map(|row| &row[..]),
            Some(&row[4..]),
            "player {}",
            player.id
        );
        assert_eq!(
            boots.get(player.id),
            Some(&pair(player.appearance.boots_id))
        );
        assert_eq!(
            gloves.get(player.id),
            Some(&pair(player.appearance.gloves_id))
        );
    }
    match kept {
        Some((row, boots_pair, gloves_pair)) => {
            assert_eq!(appearance.get(stripped), Some(&row));
            assert_eq!(boots.get(stripped), Some(&boots_pair));
            assert_eq!(gloves.get(stripped), Some(&gloves_pair));
        }
        None => {
            assert_eq!(appearance.get(stripped), None);
            assert_eq!(boots.get(stripped), None);
            assert_eq!(gloves.get(stripped), None);
        }
    }
}

// SE-STR-01
#[test]
fn the_tables_are_seeded_and_every_appearance_id_is_set_to_minus_one() {
    for version in [PesVersion::Pes21, PesVersion::Pes18, PesVersion::Pes19] {
        let sandbox = Sandbox::new(&format!("strip_seed_{}", version.number()));
        let probe = EditFile::from_bytes(&pes_savefile::fixture_save(version)).unwrap();
        let (a, b) = (probe.players()[0].id, probe.players()[1].id);
        let stripped = probe.players()[2].id;
        let input = pre_stripped_fixture(&sandbox, version, "EDIT", stripped);

        // The three tables carry a row per listed player; the stripped player's rows are
        // the CPK's own and stand, the others' are replaced by the seed.
        let kept_row = [0xab; 56];
        let (kept_boots, kept_gloves) = (pair(0xb001), pair(0x6101));
        let mut appearance = AppearanceTable::new();
        for (id, row) in [(a, [0x11; 56]), (b, [0x22; 56]), (stripped, kept_row)] {
            appearance.set(id, row);
        }
        let mut boots = ItemList::new();
        boots.set(a, pair(11));
        boots.set(stripped, kept_boots);
        let mut gloves = ItemList::new();
        gloves.set(b, pair(22));
        gloves.set(stripped, kept_gloves);
        write_cpk(
            &sandbox.path("BINS"),
            &[
                (
                    PlayerTableFile::PlayerAppearance.cpk_path(),
                    appearance.to_bytes(),
                ),
                (PlayerTableFile::BootsList.cpk_path(), boots.to_bytes()),
                (PlayerTableFile::GloveList.cpk_path(), gloves.to_bytes()),
                (UNI_COLOR, b"unicolor bytes".to_vec()),
            ],
        );

        let run = sandbox.strip_and_seed("EDIT", "BINS", "OUT", "BINS_OUT");
        assert_eq!(run.exit_code(), 0, "{version}: {:?}", run.messages());

        assert_seeded(
            &sandbox.path("BINS_OUT"),
            &input,
            stripped,
            Some((kept_row, kept_boots, kept_gloves)),
        );
        let mut archive = read_cpk(&sandbox.path("BINS_OUT"));
        let entry = archive
            .entries()
            .iter()
            .find(|entry| entry.path == UNI_COLOR)
            .cloned()
            .unwrap();
        assert_eq!(archive.read(&entry).unwrap(), b"unicolor bytes");
        assert_eq!(entry.modified, Some(WHEN), "{version}: timestamp kept");

        let written = EditFile::load(&sandbox.path("OUT")).unwrap();
        for (before, after) in input.players().iter().zip(written.players()) {
            assert_eq!(after.appearance_id, Some(STRIPPED_APPEARANCE_ID));
            let mut masked = after.clone();
            masked.appearance_id = before.appearance_id;
            assert_eq!(&masked, before, "{version} player {}", before.id);
        }
        assert_eq!(written.teams(), input.teams(), "{version} teams");
        let expected = payload_after(&sandbox.path("EDIT"), |file| {
            for player in file.players_mut() {
                player.appearance_id = Some(STRIPPED_APPEARANCE_ID);
            }
        });
        assert_eq!(payload(&sandbox.path("OUT")), expected, "{version}");
    }

    // The CPK's one-level backup: an existing --bins-out is kept as <file>.bak, and
    // --bins-out naming --bins keeps the CPK it replaces.
    let sandbox = Sandbox::new("strip_seed_bins_bak");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    write_cpk(&sandbox.path("BINS"), &[(UNI_COLOR, b"unicolor".to_vec())]);
    fs::write(sandbox.path("BINS_OUT"), b"the previous bins").unwrap();
    let run = sandbox.strip_and_seed("EDIT", "BINS", "OUT", "BINS_OUT");
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());
    assert_eq!(
        fs::read(sandbox.path("BINS_OUT.bak")).unwrap(),
        b"the previous bins".to_vec()
    );

    write_cpk(&sandbox.path("BINS2"), &[(UNI_COLOR, b"unicolor".to_vec())]);
    let replaced = fs::read(sandbox.path("BINS2")).unwrap();
    let run = sandbox.strip_and_seed("EDIT", "BINS2", "OUT2", "BINS2");
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());
    assert_eq!(fs::read(sandbox.path("BINS2.bak")).unwrap(), replaced);
}

// SE-STR-02
#[test]
fn a_pre_fox_save_is_refused_by_both_commands_with_nothing_written() {
    let sandbox = Sandbox::new("strip_pre_fox");
    sandbox.fixture(PesVersion::Pes17, "EDIT");
    write_cpk(&sandbox.path("BINS"), &[(UNI_COLOR, b"unicolor".to_vec())]);

    sandbox
        .strip_and_seed("EDIT", "BINS", "OUT", "BINS_OUT")
        .assert_refused(2, &["PES 2017"]);
    sandbox
        .check_stripped("EDIT", &[])
        .assert_refused(2, &["PES 2017"]);
    sandbox
        .check_stripped("EDIT", &["--strip-again", "--out", "OUT"])
        .assert_refused(2, &["PES 2017"]);
    assert!(!sandbox.path("OUT").exists());
    assert!(!sandbox.path("BINS_OUT").exists());
}

// SE-STR-03
#[test]
fn restored_ids_are_listed_and_strip_again_strips_them() {
    let sandbox = Sandbox::new("check_stripped");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    write_cpk(&sandbox.path("BINS"), &[(UNI_COLOR, b"unicolor".to_vec())]);
    let run = sandbox.strip_and_seed("EDIT", "BINS", "STRIPPED", "BINS_OUT");
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());

    // As Edit mode leaves it: two players' appearance ids back at their own.
    let mut file = EditFile::load(&sandbox.path("STRIPPED")).unwrap();
    let restored = [file.players()[0].id, file.players()[1].id];
    for id in restored {
        file.player_mut(id).unwrap().appearance_id = Some(id);
    }
    fs::write(sandbox.path("STRIPPED"), file.to_bytes(&TEST_SALT).unwrap()).unwrap();
    let names = restored.map(|id| file.player(id).unwrap().name.clone());

    let run = sandbox.check_stripped("STRIPPED", &[]);
    assert_eq!(run.exit_code(), 1);
    assert_eq!(
        run.messages(),
        restored
            .iter()
            .zip(&names)
            .map(|(id, name)| format!("Error appearance_id_restored (player={id}, name={name})"))
            .collect::<Vec<_>>()
    );

    let out = sandbox.arg("FIXED");
    let run = sandbox.check_stripped("STRIPPED", &["--strip-again", "--out", out.as_str()]);
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());
    assert_eq!(
        run.messages(),
        restored
            .iter()
            .zip(&names)
            .map(|(id, name)| format!("Info appearance_id_restored (player={id}, name={name})"))
            .collect::<Vec<_>>()
    );
    let fixed = EditFile::load(&sandbox.path("FIXED")).unwrap();
    assert!(
        fixed
            .players()
            .iter()
            .all(|player| player.appearance_id == Some(STRIPPED_APPEARANCE_ID)),
        "both ids stripped again, the rest still stripped"
    );
}

// SE-STR-04
#[test]
fn no_stripped_player_or_no_set_id_lists_nobody() {
    let sandbox = Sandbox::new("check_stripped_none");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    sandbox.stripped_fixture("STRIPPED");
    for save in ["EDIT", "STRIPPED"] {
        let run = sandbox.check_stripped(save, &[]);
        assert_eq!(run.exit_code(), 0, "{save}: {:?}", run.messages());
        assert!(run.messages().is_empty(), "{save}");
    }
}

// SE-STR-05
#[test]
fn a_bins_out_that_cannot_be_written_leaves_the_save_unwritten() {
    let sandbox = Sandbox::new("strip_seed_bins_out");
    sandbox.fixture(PesVersion::Pes21, "EDIT");
    write_cpk(&sandbox.path("BINS"), &[(UNI_COLOR, b"unicolor".to_vec())]);

    let run = sandbox.strip_and_seed("EDIT", "BINS", "OUT", "missing/dir/BINS_OUT");
    run.assert_refused(3, &["BINS_OUT"]);
    assert!(!sandbox.path("OUT").exists(), "--out was not written");
}

// SE-STR-06
#[test]
fn tables_the_bins_lacks_are_created_from_the_seed() {
    for version in [PesVersion::Pes21, PesVersion::Pes18, PesVersion::Pes19] {
        let sandbox = Sandbox::new(&format!("strip_seed_no_tables_{}", version.number()));
        let probe = EditFile::from_bytes(&pes_savefile::fixture_save(version)).unwrap();
        let stripped = probe.players()[2].id;
        let input = pre_stripped_fixture(&sandbox, version, "EDIT", stripped);
        write_cpk(&sandbox.path("BINS"), &[(UNI_COLOR, b"unicolor".to_vec())]);

        let run = sandbox.strip_and_seed("EDIT", "BINS", "OUT", "BINS_OUT");
        assert_eq!(run.exit_code(), 0, "{version}: {:?}", run.messages());

        assert_seeded(&sandbox.path("BINS_OUT"), &input, stripped, None);
        let mut archive = read_cpk(&sandbox.path("BINS_OUT"));
        let entry = archive
            .entries()
            .iter()
            .find(|entry| entry.path == UNI_COLOR)
            .cloned()
            .unwrap();
        assert_eq!(archive.read(&entry).unwrap(), b"unicolor", "{version}");
    }
}

// SE-STR-07
#[test]
fn each_base_copied_face_is_one_warning_once_stripped() {
    let sandbox = Sandbox::new("strip_seed_base_copy");
    // The PES 21 fixture's players copy 7511 with the face edit flag clear, but for team
    // 795's, which copy their own ids; one copying player is set to 0 through the lib.
    let mut input = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
    let cleared = input
        .players()
        .iter()
        .find(|player| player.appearance.base_copy_id == 7511)
        .unwrap()
        .id;
    input.player_mut(cleared).unwrap().appearance.base_copy_id = 0;
    fs::write(sandbox.path("EDIT"), input.to_bytes(&TEST_SALT).unwrap()).unwrap();
    // The warnings do not depend on the CPK's rows: its tables are created from the seed.
    write_cpk(&sandbox.path("BINS"), &[(UNI_COLOR, b"unicolor".to_vec())]);

    let run = sandbox.strip_and_seed("EDIT", "BINS", "OUT", "BINS_OUT");
    assert_eq!(run.exit_code(), 0, "{:?}", run.messages());

    // One warning per player whose record copies 7511; team 795's, who copy their own
    // ids, and the one set to 0 get none.
    let warnings: Vec<String> = input
        .players()
        .iter()
        .filter(|player| player.appearance.base_copy_id == 7511)
        .map(|player| {
            format!(
                "Warning strip_base_copied_face (player={}, name={}, copied=7511)",
                player.id, player.name
            )
        })
        .collect();
    assert_eq!(run.messages(), warnings);

    // The seed and the strip are SE-STR-01's: no player is pre-stripped (u32::MAX is no
    // player), so every record seeds a row and every id goes to -1.
    assert_seeded(&sandbox.path("BINS_OUT"), &input, u32::MAX, None);
    let expected = payload_after(&sandbox.path("EDIT"), |file| {
        for player in file.players_mut() {
            player.appearance_id = Some(STRIPPED_APPEARANCE_ID);
        }
    });
    assert_eq!(payload(&sandbox.path("OUT")), expected);
}

// SE-STR-08
#[test]
fn a_strip_again_whose_out_cannot_be_written_reports_nobody_fixed() {
    let sandbox = Sandbox::new("check_stripped_bad_out");
    sandbox.stripped_fixture("STRIPPED");
    // As Edit mode leaves it: one player's appearance id back at his own.
    let mut file = EditFile::load(&sandbox.path("STRIPPED")).unwrap();
    let restored = file.players()[0].id;
    file.player_mut(restored).unwrap().appearance_id = Some(restored);
    fs::write(sandbox.path("STRIPPED"), file.to_bytes(&TEST_SALT).unwrap()).unwrap();
    // An --out that is a folder cannot be written.
    fs::create_dir(sandbox.path("OUT_DIR")).unwrap();

    let out = sandbox.arg("OUT_DIR");
    let run = sandbox.check_stripped("STRIPPED", &["--strip-again", "--out", &out]);
    // assert_refused asserts no event was emitted: no appearance_id_restored line, so no
    // player is reported fixed.
    run.assert_refused(3, &["OUT_DIR"]);
}

/// `strip-and-seed` writing the save and the CPK in place, the CPK holding no player table
/// yet.
const STRIP_AND_SEED: WritingCommand = WritingCommand {
    name: "strip_seed_running",
    setup: |sandbox| {
        sandbox.fixture(PesVersion::Pes21, "EDIT");
        write_cpk(&sandbox.path("BINS"), &[(UNI_COLOR, b"unicolor".to_vec())]);
    },
    args: |sandbox| {
        let (edit, bins) = (sandbox.arg("EDIT"), sandbox.arg("BINS"));
        vec![
            "strip-and-seed".to_owned(),
            "--base".to_owned(),
            edit.clone(),
            "--bins".to_owned(),
            bins.clone(),
            "--out".to_owned(),
            edit,
            "--bins-out".to_owned(),
            bins,
        ]
    },
    saves: &["EDIT"],
    files: &["BINS"],
};

/// `check-stripped --strip-again` writing in place a save where one player is stripped and
/// every other one's id came back.
const STRIP_AGAIN: WritingCommand = WritingCommand {
    name: "strip_again_running",
    setup: |sandbox| {
        let probe = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        pre_stripped_fixture(sandbox, PesVersion::Pes21, "EDIT", probe.players()[0].id);
    },
    args: |sandbox| {
        let edit = sandbox.arg("EDIT");
        vec![
            "check-stripped".to_owned(),
            edit.clone(),
            "--strip-again".to_owned(),
            "--out".to_owned(),
            edit,
        ]
    },
    saves: &["EDIT"],
    files: &[],
};

// SE-CLI-03
#[test]
fn strip_and_seed_is_refused_while_the_saves_own_pes_runs() {
    STRIP_AND_SEED.assert_refused_while_its_game_runs();
}

// SE-CLI-03
#[test]
fn strip_and_seed_writes_while_its_pes_runs_given_edit_mode_closed() {
    STRIP_AND_SEED.assert_written_given_edit_mode_closed();
}

// SE-CLI-03
#[test]
fn strip_and_seed_writes_while_only_another_versions_pes_runs() {
    STRIP_AND_SEED.assert_written_while_another_version_runs();
}

// SE-CLI-03
#[test]
fn a_strip_again_is_refused_while_the_saves_own_pes_runs() {
    STRIP_AGAIN.assert_refused_while_its_game_runs();
}

// SE-CLI-03
#[test]
fn a_strip_again_writes_while_its_pes_runs_given_edit_mode_closed() {
    STRIP_AGAIN.assert_written_given_edit_mode_closed();
}

// SE-CLI-03
#[test]
fn a_strip_again_writes_while_only_another_versions_pes_runs() {
    STRIP_AGAIN.assert_written_while_another_version_runs();
}
