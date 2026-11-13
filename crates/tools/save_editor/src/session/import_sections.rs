//! The import's section choice: which parts of a Team TOML document an import writes.

use pes_savefile::interchange::team_toml::{
    EditFlagsSection, PlayerSection, TeamEditFlagsSection, TeamSection, TeamToml,
};
use pes_savefile::settings_toml::{AppearanceSettings, MotionSettings};

/// The import's three section choices: which parts of a team file it writes.
/// Stats is the team's name, short name, manager and stadium, with their edit
/// flags, and each player's body, abilities, positions and skills; aesthetics
/// is the team's colours, kits and strip flag and each player's names, shirt
/// number, nationality, age and look, every motion included; gameplan is the
/// `[tactics]` table: the lineup, the set-piece takers and captain, the
/// players who join the attack, the auto-tactics flags and the presets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Sections {
    pub(crate) stats: bool,
    pub(crate) aesthetics: bool,
    pub(crate) gameplan: bool,
}

/// `document` reduced to the chosen sections' keys: every key of an unchosen section set
/// absent, so `TeamToml::apply` leaves it as the save holds it.
pub(crate) fn keep_sections(document: &mut TeamToml, sections: Sections) {
    // Every struct is taken apart field by field, never with `..`: a key the format adds
    // later is then a compile error here until someone gives it a section.
    let TeamToml {
        pes_version: _,
        team,
        tactics,
        players,
        roster_breaks: _,
    } = document;
    *team = kept_team(std::mem::take(team), sections);
    *tactics = kept(sections.gameplan, std::mem::take(tactics));
    for player in players.values_mut() {
        *player = kept_player(std::mem::take(player), sections);
    }
    // A table left with no key is dropped rather than applied empty: applied, it would
    // still note an empty roster slot, or refuse a slot the team does not have.
    players.retain(|_, player| *player != PlayerSection::default());
}

/// `value` when its section is chosen, absent otherwise.
fn kept<T: Default>(chosen: bool, value: T) -> T {
    if chosen { value } else { T::default() }
}

/// The `[team]` table's name half (name, short name, manager, stadium, their
/// edit flags) loads with stats, its look (the colours, the kit slots, the
/// strip flag) with aesthetics. The id stays whatever the choice:
/// the import never writes it, and `apply` reads it as the team the file came from, whose
/// `id × 100 + slot` base-copy id means "none" and becomes the target player's own id.
fn kept_team(team: TeamSection, sections: Sections) -> TeamSection {
    let Sections {
        stats,
        aesthetics,
        gameplan: _,
    } = sections;
    let TeamSection {
        id,
        name,
        short_name,
        manager_id,
        stadium_id,
        color_1,
        color_2,
        kit_slots,
        edit_flags,
    } = team;
    let TeamEditFlagsSection {
        name: name_edited,
        short_name: short_name_edited,
        stadium: stadium_edited,
        strip: strip_edited,
    } = edit_flags;
    TeamSection {
        id,
        name: kept(stats, name),
        short_name: kept(stats, short_name),
        manager_id: kept(stats, manager_id),
        stadium_id: kept(stats, stadium_id),
        color_1: kept(aesthetics, color_1),
        color_2: kept(aesthetics, color_2),
        kit_slots: kept(aesthetics, kit_slots),
        edit_flags: TeamEditFlagsSection {
            name: kept(stats, name_edited),
            short_name: kept(stats, short_name_edited),
            stadium: kept(stats, stadium_edited),
            strip: kept(aesthetics, strip_edited),
        },
    }
}

/// One player's keys split between stats (body, abilities, positions, skills, their edit
/// flags) and aesthetics (names, shirt number, nationality, age, the look, every motion
/// included, and its flags).
fn kept_player(player: PlayerSection, sections: Sections) -> PlayerSection {
    let Sections {
        stats,
        aesthetics,
        gameplan: _,
    } = sections;
    let PlayerSection {
        name,
        shirt_name,
        number,
        nationality,
        age,
        height,
        weight,
        boots_id,
        gloves_id,
        base_copy_id,
        stats: player_stats,
        positions,
        skills,
        edit_flags,
        appearance,
    } = player;
    let EditFlagsSection {
        player: edited,
        basic_settings,
        registered_position,
        playable_positions,
        abilities,
        skills: skills_edited,
        playing_style,
        com_styles,
        motion: motion_edited,
        base_copy,
        face: face_edited,
        hair: hair_edited,
        physique: physique_edited,
        strip: strip_edited,
    } = edit_flags;
    let AppearanceSettings {
        skin_color,
        iris_color,
        physique,
        strip,
        motion,
        presentation,
        face,
        hair,
    } = appearance;
    let MotionSettings {
        hunching_dribbling,
        hunching_running,
        arm_movement_dribbling,
        arm_movement_running,
        corner_kick,
        free_kick,
        penalty_kick,
        dribbling,
        goal_celebration_1,
        goal_celebration_2,
    } = motion;
    PlayerSection {
        name: kept(aesthetics, name),
        shirt_name: kept(aesthetics, shirt_name),
        number: kept(aesthetics, number),
        nationality: kept(aesthetics, nationality),
        age: kept(aesthetics, age),
        height: kept(stats, height),
        weight: kept(stats, weight),
        boots_id: kept(aesthetics, boots_id),
        gloves_id: kept(aesthetics, gloves_id),
        base_copy_id: kept(aesthetics, base_copy_id),
        stats: kept(stats, player_stats),
        positions: kept(stats, positions),
        skills: kept(stats, skills),
        edit_flags: EditFlagsSection {
            player: kept(stats, edited),
            basic_settings: kept(stats, basic_settings),
            registered_position: kept(stats, registered_position),
            playable_positions: kept(stats, playable_positions),
            abilities: kept(stats, abilities),
            skills: kept(stats, skills_edited),
            playing_style: kept(stats, playing_style),
            com_styles: kept(stats, com_styles),
            motion: kept(aesthetics, motion_edited),
            base_copy: kept(aesthetics, base_copy),
            face: kept(aesthetics, face_edited),
            hair: kept(aesthetics, hair_edited),
            physique: kept(aesthetics, physique_edited),
            strip: kept(aesthetics, strip_edited),
        },
        appearance: AppearanceSettings {
            skin_color: kept(aesthetics, skin_color),
            iris_color: kept(aesthetics, iris_color),
            physique: kept(aesthetics, physique),
            strip: kept(aesthetics, strip),
            motion: MotionSettings {
                hunching_dribbling: kept(aesthetics, hunching_dribbling),
                hunching_running: kept(aesthetics, hunching_running),
                arm_movement_dribbling: kept(aesthetics, arm_movement_dribbling),
                arm_movement_running: kept(aesthetics, arm_movement_running),
                corner_kick: kept(aesthetics, corner_kick),
                free_kick: kept(aesthetics, free_kick),
                penalty_kick: kept(aesthetics, penalty_kick),
                dribbling: kept(aesthetics, dribbling),
                goal_celebration_1: kept(aesthetics, goal_celebration_1),
                goal_celebration_2: kept(aesthetics, goal_celebration_2),
            },
            presentation: kept(aesthetics, presentation),
            face: kept(aesthetics, face),
            hair: kept(aesthetics, hair),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use pes_savefile::EditFile;
    use pes_savefile::interchange::team_toml::TacticsSection;
    use pes_savefile::model::PlayerEntry;
    use pes_version::PesVersion;

    use super::*;

    /// The fixture both teams come from: PES 19's is a cup save whose /co/ (714) and /3/
    /// (701) hold real, distinct players.
    const VERSION: PesVersion = PesVersion::Pes19;
    const CO: u32 = 714;
    const THREE: u32 = 701;

    fn fixture() -> EditFile {
        EditFile::from_bytes(&pes_savefile::fixture_save(VERSION)).unwrap()
    }

    /// Team `id`'s document, as the command line's export builds it.
    fn export(file: &EditFile, id: u32) -> TeamToml {
        let players: Vec<&PlayerEntry> = file.players().iter().collect();
        TeamToml::from_team(VERSION, file.team(id).unwrap(), &players).unwrap()
    }

    /// /co/'s document and the same document filtered to `sections`.
    fn filtered(sections: Sections) -> (TeamToml, TeamToml) {
        let original = export(&fixture(), CO);
        let mut document = original.clone();
        keep_sections(&mut document, sections);
        (original, document)
    }

    const ONLY_STATS: Sections = Sections {
        stats: true,
        aesthetics: false,
        gameplan: false,
    };
    const ONLY_AESTHETICS: Sections = Sections {
        stats: false,
        aesthetics: true,
        gameplan: false,
    };
    const ONLY_GAMEPLAN: Sections = Sections {
        stats: false,
        aesthetics: false,
        gameplan: true,
    };

    /// A key's name and its value as text, `None` when the document leaves it absent (an
    /// empty table counts as absent: it writes nothing).
    type Key = (&'static str, Option<String>);

    fn key<T: Debug + Default + PartialEq>(name: &'static str, value: &T) -> Key {
        (name, (*value != T::default()).then(|| format!("{value:?}")))
    }

    fn team_stats_keys(team: &TeamSection) -> Vec<Key> {
        let flags = &team.edit_flags;
        vec![
            key("team.name", &team.name),
            key("team.short_name", &team.short_name),
            key("team.manager_id", &team.manager_id),
            key("team.stadium_id", &team.stadium_id),
            key("team.edit_flags.name", &flags.name),
            key("team.edit_flags.short_name", &flags.short_name),
            key("team.edit_flags.stadium", &flags.stadium),
        ]
    }

    fn team_aesthetics_keys(team: &TeamSection) -> Vec<Key> {
        vec![
            key("team.color_1", &team.color_1),
            key("team.color_2", &team.color_2),
            key("team.kit_slots", &team.kit_slots),
            key("team.edit_flags.strip", &team.edit_flags.strip),
        ]
    }

    fn tactics_keys(tactics: &TacticsSection) -> Vec<Key> {
        vec![
            key("tactics.starting_eleven", &tactics.starting_eleven),
            key("tactics.bench_order", &tactics.bench_order),
            key("tactics.set_pieces", &tactics.set_pieces),
            key(
                "tactics.players_to_join_attack",
                &tactics.players_to_join_attack,
            ),
            key("tactics.auto", &tactics.auto),
            key("tactics.presets", &tactics.presets),
        ]
    }

    fn player_stats_keys(player: &PlayerSection) -> Vec<Key> {
        let flags = &player.edit_flags;
        vec![
            key("height", &player.height),
            key("weight", &player.weight),
            key("stats", &player.stats),
            key("positions", &player.positions),
            key("skills", &player.skills),
            key("edit_flags.player", &flags.player),
            key("edit_flags.basic_settings", &flags.basic_settings),
            key("edit_flags.registered_position", &flags.registered_position),
            key("edit_flags.playable_positions", &flags.playable_positions),
            key("edit_flags.abilities", &flags.abilities),
            key("edit_flags.skills", &flags.skills),
            key("edit_flags.playing_style", &flags.playing_style),
            key("edit_flags.com_styles", &flags.com_styles),
        ]
    }

    fn player_aesthetics_keys(player: &PlayerSection) -> Vec<Key> {
        let flags = &player.edit_flags;
        let look = &player.appearance;
        let motion = &look.motion;
        vec![
            key("name", &player.name),
            key("shirt_name", &player.shirt_name),
            key("number", &player.number),
            key("nationality", &player.nationality),
            key("age", &player.age),
            key("boots_id", &player.boots_id),
            key("gloves_id", &player.gloves_id),
            key("base_copy_id", &player.base_copy_id),
            key("edit_flags.motion", &flags.motion),
            key("edit_flags.base_copy", &flags.base_copy),
            key("edit_flags.face", &flags.face),
            key("edit_flags.hair", &flags.hair),
            key("edit_flags.physique", &flags.physique),
            key("edit_flags.strip", &flags.strip),
            key("appearance.skin_color", &look.skin_color),
            key("appearance.iris_color", &look.iris_color),
            key("appearance.physique", &look.physique),
            key("appearance.strip", &look.strip),
            key("appearance.face", &look.face),
            key("appearance.hair", &look.hair),
            key(
                "appearance.motion.hunching_dribbling",
                &motion.hunching_dribbling,
            ),
            key(
                "appearance.motion.hunching_running",
                &motion.hunching_running,
            ),
            key(
                "appearance.motion.arm_movement_dribbling",
                &motion.arm_movement_dribbling,
            ),
            key(
                "appearance.motion.arm_movement_running",
                &motion.arm_movement_running,
            ),
            key("appearance.motion.corner_kick", &motion.corner_kick),
            key("appearance.motion.free_kick", &motion.free_kick),
            key("appearance.motion.penalty_kick", &motion.penalty_kick),
            key("appearance.motion.dribbling", &motion.dribbling),
            key(
                "appearance.motion.goal_celebration_1",
                &motion.goal_celebration_1,
            ),
            key(
                "appearance.motion.goal_celebration_2",
                &motion.goal_celebration_2,
            ),
            key("appearance.presentation", &look.presentation),
        ]
    }

    /// The keys absent from /co/'s PES 19 document, which PES 19 does not store. The premise
    /// checks skip them, since an absent key proves nothing about the filter.
    const NOT_IN_PES_19: [&str; 6] = [
        "team.kit_slots",
        "team.edit_flags.short_name",
        "team.edit_flags.stadium",
        "team.edit_flags.strip",
        "edit_flags.base_copy",
        "appearance.motion.dribbling",
    ];

    /// Premise: every key the rows list holds a value in at least one row, so that finding
    /// it absent after the filter is the filter's doing.
    fn assert_held(rows: &[Vec<Key>]) {
        let missing: Vec<&str> = rows[0]
            .iter()
            .enumerate()
            .filter(|(index, (name, _))| {
                !NOT_IN_PES_19.contains(name) && rows.iter().all(|row| row[*index].1.is_none())
            })
            .map(|(_, (name, _))| *name)
            .collect();
        assert!(
            missing.is_empty(),
            "premise: /co/'s document holds {missing:?}"
        );
    }

    fn assert_absent(keys: Vec<Key>, context: &str) {
        for (name, value) in keys {
            assert_eq!(value, None, "{context}: {name} is dropped");
        }
    }

    fn assert_kept(kept: Vec<Key>, original: Vec<Key>, context: &str) {
        for ((name, value), (_, expected)) in kept.into_iter().zip(original) {
            assert_eq!(
                value, expected,
                "{context}: {name} is kept as /co/'s document holds it"
            );
        }
    }

    /// Every player section of `document`, slot by slot, against `original`'s; a slot the
    /// filter dropped as empty compares as an all-absent section.
    fn player_pairs<'a>(
        original: &'a TeamToml,
        document: &'a TeamToml,
    ) -> Vec<(u8, &'a PlayerSection, PlayerSection)> {
        original
            .players
            .iter()
            .map(|(slot, player)| {
                let kept = document.players.get(slot).cloned().unwrap_or_default();
                (*slot, player, kept)
            })
            .collect()
    }

    fn assert_premises(original: &TeamToml) {
        assert_eq!(original.players.len(), 23, "premise: /co/ has 23 players");
        assert_held(&[team_stats_keys(&original.team)]);
        assert_held(&[team_aesthetics_keys(&original.team)]);
        assert_held(&[tactics_keys(&original.tactics)]);
        let players: Vec<&PlayerSection> = original.players.values().collect();
        assert_held(
            &players
                .iter()
                .map(|p| player_stats_keys(p))
                .collect::<Vec<_>>(),
        );
        assert_held(
            &players
                .iter()
                .map(|p| player_aesthetics_keys(p))
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn stats_alone_keeps_the_teams_name_half_and_the_players_abilities_and_nothing_else() {
        let (original, document) = filtered(ONLY_STATS);
        assert_premises(&original);
        assert_eq!(document.team.id, Some(CO), "the id is kept");
        assert_kept(
            team_stats_keys(&document.team),
            team_stats_keys(&original.team),
            "stats",
        );
        assert_absent(team_aesthetics_keys(&document.team), "stats");
        assert_absent(tactics_keys(&document.tactics), "stats");
        for (slot, player, kept) in player_pairs(&original, &document) {
            let context = format!("stats, slot {slot}");
            assert_kept(
                player_stats_keys(&kept),
                player_stats_keys(player),
                &context,
            );
            assert_absent(player_aesthetics_keys(&kept), &context);
        }
    }

    #[test]
    fn aesthetics_alone_keeps_the_teams_look_and_the_players_names_and_looks() {
        let (original, document) = filtered(ONLY_AESTHETICS);
        assert_premises(&original);
        assert_eq!(document.team.id, Some(CO), "the id is kept");
        assert_kept(
            team_aesthetics_keys(&document.team),
            team_aesthetics_keys(&original.team),
            "aesthetics",
        );
        assert_absent(team_stats_keys(&document.team), "aesthetics");
        assert_absent(tactics_keys(&document.tactics), "aesthetics");
        for (slot, player, kept) in player_pairs(&original, &document) {
            let context = format!("aesthetics, slot {slot}");
            assert_kept(
                player_aesthetics_keys(&kept),
                player_aesthetics_keys(player),
                &context,
            );
            assert_absent(player_stats_keys(&kept), &context);
        }
    }

    #[test]
    fn gameplan_alone_keeps_the_tactics_table_and_no_player() {
        let (original, document) = filtered(ONLY_GAMEPLAN);
        assert_premises(&original);
        assert_eq!(document.team.id, Some(CO), "the id is kept");
        assert_absent(team_stats_keys(&document.team), "gameplan");
        assert_absent(team_aesthetics_keys(&document.team), "gameplan");
        assert_kept(
            tactics_keys(&document.tactics),
            tactics_keys(&original.tactics),
            "gameplan",
        );
        for (slot, _, kept) in player_pairs(&original, &document) {
            let context = format!("gameplan, slot {slot}");
            assert_absent(player_stats_keys(&kept), &context);
            assert_absent(player_aesthetics_keys(&kept), &context);
        }
        // An emptied player table is dropped, not kept: applied, an empty one still notes
        // an empty target slot and refuses one past the roster's width.
        assert!(document.players.is_empty(), "no [players.NN] table is left");
    }

    #[test]
    fn every_section_on_leaves_the_document_unchanged() {
        let (original, document) = filtered(Sections {
            stats: true,
            aesthetics: true,
            gameplan: true,
        });
        assert_eq!(document, original);
    }

    #[test]
    fn every_section_off_leaves_only_the_id_the_version_and_the_roster_breaks() {
        let (original, document) = filtered(Sections {
            stats: false,
            aesthetics: false,
            gameplan: false,
        });
        assert_premises(&original);
        let expected = TeamToml {
            pes_version: Some(VERSION),
            team: TeamSection {
                id: Some(CO),
                ..TeamSection::default()
            },
            roster_breaks: original.roster_breaks.clone(),
            ..TeamToml::default()
        };
        assert_eq!(document, expected);
    }

    /// /co/'s slot 17 (DOOM) holds a free-kick motion of 15 and a corner-kick motion of 5
    /// (stored values; the file spells them 16 and 6).
    #[test]
    fn the_free_kick_and_corner_kick_motions_go_with_aesthetics() {
        let original = export(&fixture(), CO);
        let doom = &original.players[&17];
        assert_eq!(
            doom.name.as_deref(),
            Some("DOOM"),
            "premise: slot 17 is DOOM"
        );
        assert_eq!(doom.appearance.motion.free_kick, Some(15), "premise");
        assert_eq!(doom.appearance.motion.corner_kick, Some(5), "premise");

        let (_, stats) = filtered(ONLY_STATS);
        let motion = &stats.players[&17].appearance.motion;
        assert_eq!(motion.free_kick, None, "stats drops the free-kick motion");
        assert_eq!(
            motion.corner_kick, None,
            "stats drops the corner-kick motion"
        );

        let (_, aesthetics) = filtered(ONLY_AESTHETICS);
        let motion = &aesthetics.players[&17].appearance.motion;
        assert_eq!(
            motion.free_kick,
            Some(15),
            "aesthetics keeps the free-kick motion"
        );
        assert_eq!(
            motion.corner_kick,
            Some(5),
            "aesthetics keeps the corner-kick motion"
        );
    }

    /// /co/'s PES 19 document holds both halves of `[team]`: under stats alone its
    /// colours drop and its name half stays; under aesthetics alone the reverse.
    #[test]
    fn the_team_table_splits_between_stats_and_aesthetics() {
        let (original, aesthetics) = filtered(ONLY_AESTHETICS);
        assert_ne!(
            original.team.color_1, original.team.color_2,
            "premise: two distinct colours"
        );
        let team = &aesthetics.team;
        assert_eq!(
            team.color_1, original.team.color_1,
            "aesthetics keeps color_1"
        );
        assert_eq!(
            team.color_2, original.team.color_2,
            "aesthetics keeps color_2"
        );
        assert_eq!(team.name, None, "aesthetics drops the name");
        assert_eq!(team.short_name, None, "aesthetics drops the short name");
        assert_eq!(team.manager_id, None, "aesthetics drops the manager");
        assert_eq!(team.stadium_id, None, "aesthetics drops the stadium");

        let (_, stats) = filtered(ONLY_STATS);
        let team = &stats.team;
        assert_eq!(team.name, original.team.name, "stats keeps the name");
        assert_eq!(
            team.short_name, original.team.short_name,
            "stats keeps the short name"
        );
        assert_eq!(
            team.manager_id, original.team.manager_id,
            "stats keeps the manager"
        );
        assert_eq!(
            team.stadium_id, original.team.stadium_id,
            "stats keeps the stadium"
        );
        assert_eq!(team.color_1, None, "stats drops color_1");
        assert_eq!(team.color_2, None, "stats drops color_2");
    }

    /// The one-version edit flags load with the key they flag: PES 15's
    /// short-name flag and PES 20 on's stadium flag go with stats like the
    /// name flag.
    #[test]
    fn the_versions_own_team_edit_flags_go_with_stats() {
        /// One version-gated flag of `[team.edit_flags]`, read off a team.
        type Flag = fn(&TeamEditFlagsSection) -> Option<bool>;
        let cases: [(PesVersion, &'static str, Flag); 2] = [
            (PesVersion::Pes15, "short-name", |flags| flags.short_name),
            (PesVersion::Pes21, "stadium", |flags| flags.stadium),
        ];
        for (version, label, flag) in cases {
            let file = EditFile::from_bytes(&pes_savefile::fixture_save(version))
                .expect("the fixture reads");
            let players: Vec<&PlayerEntry> = file.players().iter().collect();
            let original = TeamToml::from_team(version, file.team(CO).unwrap(), &players).unwrap();
            let held = flag(&original.team.edit_flags);
            assert_eq!(
                held,
                Some(false),
                "premise: /co/'s {version:?} document holds the {label} flag"
            );

            let mut stats = original.clone();
            keep_sections(&mut stats, ONLY_STATS);
            assert_eq!(
                flag(&stats.team.edit_flags),
                held,
                "stats keeps the {label} flag"
            );

            let mut aesthetics = original.clone();
            keep_sections(&mut aesthetics, ONLY_AESTHETICS);
            assert_eq!(
                flag(&aesthetics.team.edit_flags),
                None,
                "aesthetics drops the {label} flag"
            );
        }
    }

    /// PES 17's `[team]` keys PES 19 lacks split the same way: the kit slots and
    /// the strip flag are the team's look, the name flag stats'.
    #[test]
    fn pes_17s_kit_slots_and_strip_flag_go_with_aesthetics() {
        let file = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes17))
            .expect("the PES 17 fixture reads");
        let players: Vec<&PlayerEntry> = file.players().iter().collect();
        let original =
            TeamToml::from_team(PesVersion::Pes17, file.team(CO).unwrap(), &players).unwrap();
        assert!(original.team.kit_slots.is_some(), "premise: kit slots");
        assert_eq!(
            original.team.edit_flags.strip,
            Some(false),
            "premise: the strip flag"
        );
        assert_eq!(
            original.team.edit_flags.name,
            Some(false),
            "premise: the name flag"
        );

        let mut aesthetics = original.clone();
        keep_sections(&mut aesthetics, ONLY_AESTHETICS);
        assert_eq!(
            aesthetics.team.kit_slots, original.team.kit_slots,
            "aesthetics keeps the kit slots"
        );
        assert_eq!(
            aesthetics.team.edit_flags.strip,
            Some(false),
            "aesthetics keeps the strip flag"
        );
        assert_eq!(
            aesthetics.team.edit_flags.name, None,
            "aesthetics drops the name flag"
        );

        let mut stats = original.clone();
        keep_sections(&mut stats, ONLY_STATS);
        assert_eq!(stats.team.kit_slots, None, "stats drops the kit slots");
        assert_eq!(
            stats.team.edit_flags.strip, None,
            "stats drops the strip flag"
        );
        assert_eq!(
            stats.team.edit_flags.name,
            Some(false),
            "stats keeps the name flag"
        );
    }

    /// PES 17's `[players.NN.edit_flags]` base-copy flag, which the PES 19
    /// document lacks, is the look's: it routes with aesthetics like the
    /// motion and face flags.
    #[test]
    fn pes_17s_base_copy_flag_goes_with_aesthetics() {
        let file = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes17))
            .expect("the PES 17 fixture reads");
        let players: Vec<&PlayerEntry> = file.players().iter().collect();
        let original =
            TeamToml::from_team(PesVersion::Pes17, file.team(CO).unwrap(), &players).unwrap();
        let (slot, flag) = original
            .players
            .iter()
            .find(|(_, player)| player.edit_flags.base_copy.is_some())
            .map(|(slot, player)| (*slot, player.edit_flags.base_copy))
            .expect("premise: a player holds the base-copy flag");

        let mut aesthetics = original.clone();
        keep_sections(&mut aesthetics, ONLY_AESTHETICS);
        assert_eq!(
            aesthetics.players[&slot].edit_flags.base_copy, flag,
            "aesthetics keeps the base-copy flag"
        );

        let mut stats = original.clone();
        keep_sections(&mut stats, ONLY_STATS);
        assert_eq!(
            stats.players[&slot].edit_flags.base_copy, None,
            "stats drops the base-copy flag"
        );
    }

    /// PES 21's `appearance.motion.dribbling`, which the PES 19 document
    /// lacks, is a motion: it routes with aesthetics like the free
    /// kick's.
    #[test]
    fn pes_21s_dribbling_motion_goes_with_aesthetics() {
        let file = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21))
            .expect("the PES 21 fixture reads");
        let players: Vec<&PlayerEntry> = file.players().iter().collect();
        let original =
            TeamToml::from_team(PesVersion::Pes21, file.team(CO).unwrap(), &players).unwrap();
        let (slot, dribbling) = original
            .players
            .iter()
            .find(|(_, player)| player.appearance.motion.dribbling.is_some())
            .map(|(slot, player)| (*slot, player.appearance.motion.dribbling))
            .expect("premise: a player holds the dribbling motion");

        let mut aesthetics = original.clone();
        keep_sections(&mut aesthetics, ONLY_AESTHETICS);
        assert_eq!(
            aesthetics.players[&slot].appearance.motion.dribbling, dribbling,
            "aesthetics keeps the dribbling motion"
        );

        let mut stats = original.clone();
        keep_sections(&mut stats, ONLY_STATS);
        assert_eq!(
            stats.players[&slot].appearance.motion.dribbling, None,
            "stats drops the dribbling motion"
        );
    }

    /// /co/'s document with stats alone, applied onto /3/ as `Session::import_team` does:
    /// every one of the 23 roster slots compared, 701's slot `i` against 714's slot `i`.
    #[test]
    fn stats_alone_onto_another_team_writes_its_abilities_and_keeps_its_names_and_looks() {
        let file = fixture();
        let player_at = |team: u32, slot: usize| -> &PlayerEntry {
            let id = file.team(team).unwrap().roster[slot].player_id;
            file.player(id).unwrap()
        };
        let mut document = export(&file, CO);
        keep_sections(&mut document, ONLY_STATS);
        document.team.id = Some(THREE);
        let before = file.team(THREE).unwrap().clone();
        let mut team = before.clone();
        let mut players = file.players().to_vec();
        for slot in 0..23 {
            let (three, co) = (player_at(THREE, slot), player_at(CO, slot));
            assert_ne!(three.name, co.name, "premise: slot {slot}'s names differ");
            assert_ne!(
                three.stats, co.stats,
                "premise: slot {slot}'s abilities differ"
            );
            assert_ne!(
                three.appearance, co.appearance,
                "premise: slot {slot}'s appearances differ"
            );
        }
        assert_ne!(
            before.tactics,
            file.team(CO).unwrap().tactics,
            "premise: the tactics differ"
        );
        assert_ne!(
            before.colors,
            file.team(CO).unwrap().colors,
            "premise: the colours differ"
        );
        let differ = |what: fn(&PlayerEntry) -> u8| {
            (0..23).any(|slot| what(player_at(THREE, slot)) != what(player_at(CO, slot)))
        };
        assert!(differ(|p| p.motion.free_kick), "premise: free kicks differ");
        assert!(
            differ(|p| p.motion.corner_kick),
            "premise: corner kicks differ"
        );
        assert!(
            (0..23).any(
                |slot| before.roster[slot].number != file.team(CO).unwrap().roster[slot].number
            ),
            "premise: the shirt numbers differ"
        );

        document.apply(VERSION, &mut team, &mut players).unwrap();

        assert_eq!(team.tactics, before.tactics, "/3/'s tactics are unchanged");
        assert_eq!(team.name, "/co/", "the team's name is the file's");
        assert_eq!(team.colors, before.colors, "/3/'s colours are unchanged");
        for slot in 0..23 {
            let three = player_at(THREE, slot);
            let id = before.roster[slot].player_id;
            let after = players.iter().find(|p| p.id == id).unwrap();
            assert_eq!(after.name, three.name, "slot {slot}: the name is /3/'s");
            assert_eq!(
                after.shirt_name, three.shirt_name,
                "slot {slot}: the shirt name"
            );
            assert_eq!(
                team.roster[slot].number, before.roster[slot].number,
                "slot {slot}: number"
            );
            assert_eq!(
                after.appearance, three.appearance,
                "slot {slot}: the appearance"
            );
            let co = player_at(CO, slot);
            // The presentation fields are aesthetics: the import keeps
            // /3/'s star and playing attitude, not /co/'s.
            assert_eq!(
                after.stats,
                pes_savefile::model::player::PlayerStats {
                    star: three.stats.star,
                    playing_attitude: three.stats.playing_attitude,
                    ..co.stats
                },
                "slot {slot}: /co/'s abilities, /3/'s presentation"
            );
            assert_eq!(
                after.motion, three.motion,
                "slot {slot}: /3/'s motions, the free kick's with them"
            );
        }
    }
}
