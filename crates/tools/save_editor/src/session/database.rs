//! The Savefile menu's database operations. Output rosters to TSV writes the
//! roster sheet members paste into spreadsheets; it is a pure read of the
//! open save, so it is never locked and takes no undo step. Fix database is
//! the maintenance repair behind the confirmation dialog, one sealed undo
//! entry. It repairs base copies only where the record has the flag (PES 15
//! to 18): later versions have no such flag, and clearing copy ids there
//! would change players' looks. It does not write PES 17's strip table: a
//! repair of it waits for the table's layout to be measured. egui-free, so
//! the sheet and the repair's computation read without a frame.

use std::path::Path;

use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::labels::POSITIONS;
use pes_savefile::model::{KitSlot, PlayStyle, PlayerEntry, TeamEntry};
use pes_version::PesVersion;
use studio_core::Recorder;

use crate::session::Session;
use crate::session::journal;

/// The skill names the sheet spells, index = the canonical skill slot
/// (0 Scissors Feint to 40 Through Passing). The spellings are the parity
/// table's ("Acro Finishing", "Acro Clear", "GK High Punt Trajectory"),
/// kept as they are, as `team_ops`' `POSITION_NAMES` keeps its own.
const SKILLS: [&str; 41] = [
    "Scissors Feint",
    "Flip Flap",
    "Marseille Turn",
    "Sombrero",
    "Cut Behind & Turn",
    "Scotch Move",
    "Heading",
    "Long Range Drive",
    "Knuckle Shot",
    "Acro Finishing",
    "Heel Trick",
    "First Time Shot",
    "One Touch Pass",
    "Weighted Pass",
    "Pinpoint Crossing",
    "Outside Curler",
    "Rabona",
    "Low Lofted Pass",
    "Low Punt Trajectory",
    "Long Throw",
    "GK Long Throw",
    "Malicia",
    "Man Marking",
    "Track Back",
    "Acro Clear",
    "Captaincy",
    "Super Sub",
    "Fighting Spirit",
    "Double Touch",
    "Crossover Turn",
    "Step on Skill",
    "Chip Shot",
    "Dipping Shots",
    "Rising Shots",
    "No Look Pass",
    "GK High Punt Trajectory",
    "Penalty Specialist",
    "GK Penalty Specialist",
    "Interception",
    "Long Range Shooting",
    "Through Passing",
];

/// The COM playing-style names the sheet spells, index = the canonical
/// slot (0 Trickster to 6 Long Ranger); the parity table's spellings.
const COM_STYLES: [&str; 7] = [
    "Trickster",
    "Mazing Runner",
    "Speeding Bullet",
    "Incisive Run",
    "Long Ball Expert",
    "Early Cross",
    "Long Ranger",
];

/// The kit slots Fix database writes: Kit 1, Kit 2 and the goalkeeper kit
/// bound to the team (`id * 64`), the seven slots past them unbound
/// (`0xFFFFC0`, a team id no save holds).
fn default_kit_slots(team_id: u32) -> [KitSlot; 10] {
    let bound = KitSlot {
        number: 0,
        binding: team_id * 64,
    };
    let mut slots = [bound; 10];
    slots[1].number = 1;
    slots[2].number = 0x80;
    for slot in &mut slots[3..] {
        *slot = KitSlot {
            number: 0,
            binding: 0xFFFFC0,
        };
    }
    slots
}

/// The roster sheet's text (`rosters_tsv`): per team of the save, in
/// record order and hidden ones included, the team's name on its own
/// line, then one tab-separated row per filled roster slot in slot order
/// — shirt number, name (raw, the colour codes kept), registered
/// position, Finishing, `(C)` on the captain's slot, height, playing
/// style, then each skill and COM style set — and an empty line after
/// the team.
pub(crate) fn rosters_tsv(file: &EditFile) -> String {
    let mut out = String::new();
    for team in file.teams() {
        out.push_str(&team.name);
        out.push('\n');
        for (index, slot) in team.roster.iter().enumerate() {
            if slot.player_id == 0 {
                continue;
            }
            let Some(player) = file.player(slot.player_id) else {
                continue;
            };
            let captain = usize::from(team.tactics.set_pieces.captain) == index;
            let position = POSITIONS
                .get(usize::from(player.positions.registered))
                .copied()
                .unwrap_or("?");
            let style = playing_style(file.version(), player.positions.playing_style);
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                slot.number,
                player.name,
                position,
                player.stats.finishing,
                if captain { "(C)" } else { "" },
                player.basic.height,
                style,
            ));
            for (i, &has) in player.skills.skills.iter().enumerate() {
                if has {
                    out.push('\t');
                    out.push_str(SKILLS[i]);
                }
            }
            for (i, &has) in player.skills.com_styles.iter().enumerate() {
                if has {
                    out.push('\t');
                    out.push_str(COM_STYLES[i]);
                }
            }
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

/// The stored playing-style index as the sheet spells it: the version's
/// own list (`schema::playstyle::decode`; on PES 15/16 the list's slot 16
/// is a hole no style occupies, so the stored 17 and 18 past it already
/// read as the goalkeepers' styles). An index the list lacks, or the
/// hole, spells `?`, as a position past `POSITIONS` does.
fn playing_style(version: PesVersion, stored: u8) -> &'static str {
    pes_savefile::schema::playstyle::decode(version, stored)
        .map(play_style_name)
        .unwrap_or("?")
}

/// The sheet's spelling of `style`, the same words everywhere the roster
/// sheet is read.
fn play_style_name(style: PlayStyle) -> &'static str {
    match style {
        PlayStyle::None => "None",
        PlayStyle::GoalPoacher => "Goal Poacher",
        PlayStyle::DummyRunner => "Dummy Runner",
        PlayStyle::FoxInTheBox => "Fox in the Box",
        PlayStyle::TargetMan => "Target Man",
        PlayStyle::CreativePlaymaker => "Creative Playmaker",
        PlayStyle::ProlificWinger => "Prolific Winger",
        PlayStyle::RoamingFlank => "Roaming Flank",
        PlayStyle::CrossingSpecialist => "Crossing Specialist",
        PlayStyle::ClassicNo10 => "Classic No. 10",
        PlayStyle::HolePlayer => "Hole Player",
        PlayStyle::BoxToBox => "Box to Box",
        PlayStyle::TheDestroyer => "The Destroyer",
        PlayStyle::Orchestrator => "Orchestrator",
        PlayStyle::AnchorMan => "Anchor Man",
        PlayStyle::OffensiveFullback => "Offensive Fullback",
        PlayStyle::FullbackFinisher => "Fullback Finisher",
        PlayStyle::DefensiveFullback => "Defensive Fullback",
        PlayStyle::BuildUp => "Build Up",
        PlayStyle::ExtraFrontman => "Extra Frontman",
        PlayStyle::OffensiveGoalkeeper => "Offensive Goalkeeper",
        PlayStyle::DefensiveGoalkeeper => "Defensive Goalkeeper",
    }
}

/// Writes the roster sheet at `path`: a UTF-8 byte-order mark first, the
/// sheet's text after it (a spreadsheet reads accented names right by
/// the mark). The error names the path; `recorder` gets the `wrote`
/// step and keeps the sheet as an output.
pub(crate) fn write_rosters_tsv(
    file: &EditFile,
    recorder: &Recorder,
    path: &Path,
) -> anyhow::Result<()> {
    let mut text = String::from("\u{feff}");
    text.push_str(&rosters_tsv(file));
    journal::write_and_record(recorder, path, text.as_bytes())
}

/// The changed players and teams Fix database writes, for
/// `Session::edit_entries` (each entry cloned, repaired, kept only when
/// it differs): the repair reads, `edit_entries` writes.
///
/// On every player the four visual flags clear; where the record has the
/// base-copy flag (PES 15 to 18) a copy id naming another player while
/// the flag is off becomes 0, and the flag itself clears when the id is
/// the player's own or 0. On every team the strip flag clears where the
/// record has it (PES 17), and the kit slots go back to their defaults
/// where the record has them (PES 17 again).
pub(crate) fn fix_database(file: &EditFile) -> (Vec<PlayerEntry>, Vec<TeamEntry>) {
    let mut players = Vec::new();
    for player in file.players() {
        let mut repaired = player.clone();
        match repaired.edit_flags.base_copy {
            Some(false)
                if repaired.appearance.base_copy_id != repaired.id
                    && repaired.appearance.base_copy_id != 0 =>
            {
                repaired.appearance.base_copy_id = 0;
            }
            Some(true)
                if repaired.appearance.base_copy_id == repaired.id
                    || repaired.appearance.base_copy_id == 0 =>
            {
                repaired.edit_flags.base_copy = Some(false);
            }
            // A consistent base copy, and PES 19 to 21's `None` (the
            // record has no flag): neither repairs.
            Some(_) | None => {}
        }
        repaired.edit_flags.face = false;
        repaired.edit_flags.hair = false;
        repaired.edit_flags.physique = false;
        repaired.edit_flags.strip = false;
        if repaired != *player {
            players.push(repaired);
        }
    }
    let mut teams = Vec::new();
    for team in file.teams() {
        let mut repaired = team.clone();
        if repaired.edit_flags.strip.is_some() {
            repaired.edit_flags.strip = Some(false);
        }
        if let Some(slots) = repaired.kit_slots.as_mut() {
            *slots = default_kit_slots(team.id);
        }
        if repaired != *team {
            teams.push(repaired);
        }
    }
    (players, teams)
}

impl Session {
    /// Fix database, as one sealed undo entry holding every changed
    /// player and team: the pure `fix_database` computes the changed
    /// entries, `edit_entries` applies them. The aesthetics
    /// lock is the menu's to apply, not this function's.
    pub(crate) fn fix_database(&mut self) {
        let (players, teams) = fix_database(self.file());
        self.edit_entries(players, teams);
    }
}

#[cfg(test)]
mod tests {
    use pes_savefile::fixture_save;
    use pes_version::PesVersion;

    use super::*;

    /// `version`'s fixture save as an `EditFile`.
    fn save(version: PesVersion) -> EditFile {
        EditFile::from_bytes(&fixture_save(version)).unwrap()
    }

    /// The index of `team_id`'s name line in the sheet: every team before
    /// it writes a name line, one row per filled roster slot and an empty
    /// line (the fixture's rosters point at players the save holds).
    fn block_start(file: &EditFile, team_id: u32) -> usize {
        file.teams()
            .iter()
            .take_while(|team| team.id != team_id)
            .map(|team| {
                2 + team
                    .roster
                    .iter()
                    .filter(|slot| slot.player_id != 0)
                    .count()
            })
            .sum()
    }

    // SE-GUI-27
    #[test]
    fn the_sheet_writes_a_line_per_team_and_a_row_per_filled_slot() {
        let mut file = save(PesVersion::Pes21);
        // Distinctive values on the first two slots, so the literal rows
        // exercise every column: a colour-coded name, the captain's mark,
        // skills and COM styles, two playing-style labels.
        {
            let player = file.player_mut(70101).unwrap();
            player.name = "\u{11}c112233ffACE".to_owned();
            player.positions.registered = 0;
            player.stats.finishing = 88;
            player.basic.height = 195;
            player.positions.playing_style = 0;
            player.skills.skills[6] = true;
            player.skills.skills[25] = true;
            player.skills.com_styles[0] = true;
        }
        {
            let player = file.player_mut(70102).unwrap();
            player.name = "ROOKIE".to_owned();
            player.positions.registered = 12;
            player.stats.finishing = 91;
            player.basic.height = 171;
            player.positions.playing_style = 1;
            player.skills.skills[0] = true;
            player.skills.skills[40] = true;
            player.skills.com_styles[6] = true;
        }
        assert_eq!(
            file.team(701).unwrap().tactics.set_pieces.captain,
            0,
            "premise: slot 0 is the captain's"
        );
        let filled = file
            .team(701)
            .unwrap()
            .roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .count();
        assert!(filled > 2, "premise: several filled slots");

        let tsv = rosters_tsv(&file);
        let lines: Vec<&str> = tsv.lines().collect();
        assert_eq!(
            lines[0],
            file.teams()[0].name,
            "the first line is the first team's name"
        );
        let start = block_start(&file, 701);
        assert_eq!(lines[start], file.team(701).unwrap().name);
        // The literals are written by hand from the fixture's values
        // through the parity tables (`menu_lists.cpp`'s `gpc_playstyle20`,
        // `playskills`, `comstyles` and `gpc_pos_short`), not through
        // `rosters_tsv`.
        assert_eq!(
            lines[start + 1],
            "1\t\u{11}c112233ffACE\tGK\t88\t(C)\t195\tNone\tHeading\tCaptaincy\tTrickster",
            "slot 1's row"
        );
        assert_eq!(
            lines[start + 2],
            "2\tROOKIE\tCF\t91\t\t171\tGoal Poacher\tScissors Feint\tThrough Passing\tLong Ranger",
            "slot 2's row"
        );
        assert!(
            lines[start + filled].starts_with("23\t"),
            "the last filled slot's row follows: {}",
            lines[start + filled]
        );
        assert_eq!(
            lines[start + filled + 1],
            "",
            "an empty line follows the last row"
        );

        // Every team of the save writes one block, in record order: the
        // name line, one row per filled roster slot, an empty line.
        let mut offset = 0;
        for team in file.teams() {
            assert_eq!(lines[offset], team.name, "team {}'s name line", team.id);
            let team_filled = team
                .roster
                .iter()
                .filter(|slot| slot.player_id != 0)
                .count();
            for row in &lines[offset + 1..offset + 1 + team_filled] {
                assert!(
                    row.contains('\t'),
                    "a player row of team {}: {row}",
                    team.id
                );
            }
            assert_eq!(
                lines[offset + 1 + team_filled],
                "",
                "an empty line after team {}",
                team.id
            );
            offset += team_filled + 2;
        }
        assert_eq!(offset, lines.len(), "one block per team, no more");

        // The written file: the byte-order mark first, the text after it.
        let dir = crate::test_support::scratch("rosters_tsv");
        let path = dir.join("EDIT.tsv");
        let recorder = Recorder::default();
        write_rosters_tsv(&file, &recorder, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(
            &bytes[..3],
            &[0xEF, 0xBB, 0xBF],
            "the UTF-8 byte-order mark"
        );
        assert_eq!(&bytes[3..], tsv.as_bytes(), "the sheet's text follows");

        // The sheet's write is a journal step and its bytes the kept
        // output.
        crate::test_support::assert_wrote_and_kept(&recorder, &dir, &path, "outputs/0-EDIT.tsv");
    }

    // SE-GUI-27
    #[test]
    fn a_pes19_playing_style_uses_the_versions_own_order() {
        let mut file = save(PesVersion::Pes19);
        // PES 19's stored order puts Defensive Fullback at 18 and Extra
        // Frontman at 19, the other way round from PES 20/21's.
        file.player_mut(70101).unwrap().positions.playing_style = 18;
        file.player_mut(70102).unwrap().positions.playing_style = 19;

        let tsv = rosters_tsv(&file);
        let lines: Vec<&str> = tsv.lines().collect();
        let start = block_start(&file, 701);
        assert_eq!(
            lines[start + 1].split('\t').nth(6),
            Some("Defensive Fullback"),
            "stored 18"
        );
        assert_eq!(
            lines[start + 2].split('\t').nth(6),
            Some("Extra Frontman"),
            "stored 19"
        );
    }

    // SE-GUI-27
    #[test]
    fn a_pes16_playing_style_uses_the_versions_own_list() {
        let mut file = save(PesVersion::Pes16);
        // PES 15/16's style list has a hole at stored 16 and the two
        // goalkeeper styles at 17 and 18: `decode` maps stored 18 to
        // Defensive Goalkeeper, what the shifted index-17 read spells.
        file.player_mut(70101).unwrap().positions.playing_style = 18;
        file.player_mut(70102).unwrap().positions.playing_style = 16;

        let tsv = rosters_tsv(&file);
        let lines: Vec<&str> = tsv.lines().collect();
        let start = block_start(&file, 701);
        assert_eq!(
            lines[start + 1].split('\t').nth(6),
            Some("Defensive Goalkeeper"),
            "stored 18 is the defensive goalkeeper's style"
        );
        assert_eq!(
            lines[start + 2].split('\t').nth(6),
            Some("?"),
            "stored 16 is the list's hole: `decode` refuses it"
        );
    }

    // SE-GUI-27
    #[test]
    fn the_repair_reports_only_the_entries_it_changes() {
        let mut file = save(PesVersion::Pes17);
        {
            let player = file.player_mut(70101).unwrap();
            player.edit_flags.base_copy = Some(false);
            player.appearance.base_copy_id = 70102;
            player.edit_flags.face = true;
            player.edit_flags.hair = true;
        }
        {
            let player = file.player_mut(70102).unwrap();
            player.edit_flags.base_copy = Some(true);
            player.appearance.base_copy_id = 70102;
        }
        // A real base copy: the flag set and the id another player's.
        {
            let player = file.player_mut(70103).unwrap();
            player.edit_flags.base_copy = Some(true);
            player.appearance.base_copy_id = 70104;
        }
        // The flag-on arm's zero half: the flag set with copy id 0.
        {
            let player = file.player_mut(70104).unwrap();
            player.edit_flags.base_copy = Some(true);
            player.appearance.base_copy_id = 0;
        }
        // The flag-off arm's own-id half: a copy id naming himself is
        // consistent, not a stray.
        {
            let player = file.player_mut(70105).unwrap();
            player.edit_flags.base_copy = Some(false);
            player.appearance.base_copy_id = 70105;
        }
        for id in [70103u32, 70104, 70105] {
            let flags = &file.player(id).unwrap().edit_flags;
            assert!(
                !(flags.face || flags.hair || flags.physique || flags.strip),
                "premise: {id}'s visual flags are clear"
            );
        }
        file.team_mut(701).unwrap().edit_flags.strip = Some(true);

        let (players, teams) = fix_database(&file);

        let first = players
            .iter()
            .find(|player| player.id == 70101)
            .expect("the flag-off copy id is repaired");
        assert_eq!(first.appearance.base_copy_id, 0, "the stray copy id clears");
        assert!(
            !(first.edit_flags.face
                || first.edit_flags.hair
                || first.edit_flags.physique
                || first.edit_flags.strip),
            "the visual flags clear"
        );
        let second = players
            .iter()
            .find(|player| player.id == 70102)
            .expect("the baseless flag clears");
        assert_eq!(second.edit_flags.base_copy, Some(false));
        assert_eq!(second.appearance.base_copy_id, 70102, "the copy id stays");
        assert!(
            !players.iter().any(|player| player.id == 70103),
            "a real base copy changes nothing: he is not returned"
        );
        let fourth = players
            .iter()
            .find(|player| player.id == 70104)
            .expect("the flag with copy id 0 clears");
        assert_eq!(fourth.edit_flags.base_copy, Some(false));
        assert_eq!(fourth.appearance.base_copy_id, 0, "the copy id stays 0");
        assert!(
            !players.iter().any(|player| player.id == 70105),
            "a flag-off copy of himself changes nothing: he is not returned"
        );
        let team = teams
            .iter()
            .find(|team| team.id == 701)
            .expect("the set strip flag is repaired");
        assert_eq!(team.edit_flags.strip, Some(false));
        assert!(
            !teams.iter().any(|team| team.id != 701),
            "no other team changed"
        );
    }

    // SE-GUI-27
    #[test]
    fn a_pes21_player_keeps_his_base_copy() {
        let mut file = save(PesVersion::Pes21);
        assert_eq!(
            file.player(70101).unwrap().edit_flags.base_copy,
            None,
            "premise: PES 21's record has no base-copy flag"
        );
        file.player_mut(70101).unwrap().appearance.base_copy_id = 70102;

        let (players, _) = fix_database(&file);

        let kept = players
            .iter()
            .find(|player| player.id == 70101)
            .map(|player| player.appearance.base_copy_id)
            .unwrap_or(70102);
        assert_eq!(kept, 70102, "the repair never touches a PES 21 copy id");
    }
}
