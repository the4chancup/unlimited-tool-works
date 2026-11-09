//! The 4cc team identity space (`TeamName`, `TeamId`) and the `teams_list.txt`
//! file.
//!
//! `TeamName::new` is the one place the `/xx/` fold lives, so export names and
//! the list's `Name` column cannot drift apart. The list itself parses and
//! writes `teams_list.txt` and [`reconcile`] merges an incoming list into a
//! working copy; callers own filesystem I/O.

mod file;
mod id;
mod name;
mod reconcile;

pub use file::{Row, TeamsList, TeamsListError};
pub use id::{TeamId, TeamIdError};
pub use name::{TeamName, TeamNameError};
pub use reconcile::{MergeSummary, reconcile};

#[cfg(test)]
mod tests {
    use super::*;

    fn list(text: &str) -> TeamsList {
        TeamsList::parse(text).unwrap()
    }

    /// A merged list must still satisfy the file's own invariants.
    fn reparses(list: &TeamsList) {
        assert!(
            TeamsList::parse(&list.write()).is_ok(),
            "merged list does not parse:\n{}",
            list.write()
        );
    }

    /// The shipped list changes with every league's allocation, so this checks only what
    /// any version of it must hold: one row for each team ID, 701 to 920, in order (a team
    /// or a placeholder). Which team holds which ID is the data's business, not a test's.
    #[test]
    fn upstream_has_one_row_per_team_id() {
        let list = list(TeamsList::UPSTREAM);
        let ids: Vec<u16> = list
            .rows()
            .iter()
            .map(|row| match row {
                Row::Team { cells, .. } | Row::Placeholder { cells } => cells[0].parse().unwrap(),
            })
            .collect();
        assert_eq!(ids, (701..=920).collect::<Vec<u16>>());
    }

    #[test]
    fn unmodified_list_writes_back_identically() {
        let list = list(TeamsList::UPSTREAM);
        assert_eq!(list.write(), TeamsList::UPSTREAM);
    }

    #[test]
    fn bom_and_lf_input_writes_crlf_without_bom() {
        let list = list("\u{feff}ID\tName\n701\t/co/\n\n702\t/a/\r\n");
        assert_eq!(list.write(), "ID\tName\r\n701\t/co/\r\n702\t/a/\r\n");
    }

    #[test]
    fn header_errors_carry_the_right_variant_and_line() {
        assert_eq!(
            TeamsList::parse("ID\tOther\n"),
            Err(TeamsListError::MissingColumn("Name"))
        );
        assert_eq!(TeamsList::parse(""), Err(TeamsListError::MissingHeader));
        assert_eq!(
            TeamsList::parse("ID\tName\n700\t/co/\n"),
            Err(TeamsListError::InvalidId {
                line: 2,
                text: "700".to_owned()
            })
        );
        assert_eq!(
            TeamsList::parse("ID\tName\nabc\t/co/\n"),
            Err(TeamsListError::InvalidId {
                line: 2,
                text: "abc".to_owned()
            })
        );
        assert_eq!(
            TeamsList::parse("ID\tName\n701\t/co/\n701\t/a/\n"),
            Err(TeamsListError::DuplicateId { line: 3, id: 701 })
        );
        assert_eq!(
            TeamsList::parse("ID\tName\n701\t/co/\n702\t/CO/\n"),
            Err(TeamsListError::DuplicateName {
                line: 3,
                name: "/co/".to_owned()
            })
        );
    }

    #[test]
    fn reconcile_merges_incoming_into_working() {
        let working = list(
            "ID\tName\tMin\n\
             701\t/co/\t1\n\
             772\tBackup 1\t9\n\
             703\t/shared/\t3\n",
        );
        let incoming = list(
            "ID\tName\tMin\n\
             701\t/co/\t11\n\
             799\t/shared/\t33\n\
             750\t/newteam/\t55\n",
        );
        let (merged, summary) = reconcile(&working, &incoming);

        assert_eq!(
            summary.added,
            [(TeamId::new(750).unwrap(), TeamName::new("newteam").unwrap())]
        );
        // 701 kept (identical in both), 703 overridden — `kept` counts
        // working team rows that survived unchanged.
        assert_eq!(summary.kept, 1);
        assert_eq!(
            summary.overridden,
            [(
                TeamName::new("shared").unwrap(),
                TeamId::new(703).unwrap(),
                TeamId::new(799).unwrap()
            )]
        );
        assert!(summary.unresolved.is_empty());

        // Placeholder preserved, appended row last, cells copied verbatim.
        assert!(matches!(merged.rows()[1], Row::Placeholder { .. }));
        let last = merged.rows().last().unwrap();
        assert!(matches!(
            last,
            Row::Team { cells, id, .. }
            if *id == TeamId::new(750).unwrap()
                && cells == &["750".to_owned(), "/newteam/".to_owned(), "55".to_owned()]
        ));
        assert!(merged.write().ends_with("750\t/newteam/\t55\r\n"));
        assert_eq!(
            merged.id_of(&TeamName::new("shared").unwrap()),
            TeamId::new(799).ok()
        );
        reparses(&merged);
    }

    #[test]
    fn reconcile_maps_columns_by_header_when_they_differ() {
        // The working list's header order differs from the incoming one: the
        // id and name cells land in the working columns, not the positions
        // they held in the incoming list.
        let working = list(
            "Name\tID\tNotes\n\
             /co/\t701\tx\n\
             Backup 1\t772\tnote\n",
        );
        let incoming = list(
            "ID\tName\n\
             701\t/co/\n\
             772\t/new/\n\
             750\t/other/\n",
        );
        let (merged, summary) = reconcile(&working, &incoming);

        assert!(summary.unresolved.is_empty());
        // The placeholder slot takes the incoming row, id and name written
        // into the working list's columns.
        assert!(matches!(
            &merged.rows()[1],
            Row::Team { cells, id, name }
                if *id == TeamId::new(772).unwrap()
                    && name.as_str() == "/new/"
                    && cells == &["/new/".to_owned(), "772".to_owned(), "note".to_owned()]
        ));
        // The appended row is blank except the id and name cells, placed by
        // the working header.
        let last = merged.rows().last().unwrap();
        assert!(matches!(
            last,
            Row::Team { cells, id, name }
                if *id == TeamId::new(750).unwrap()
                    && name.as_str() == "/other/"
                    && cells == &["/other/".to_owned(), "750".to_owned(), String::new()]
        ));
        assert_eq!(
            summary.added,
            [
                (TeamId::new(772).unwrap(), TeamName::new("new").unwrap()),
                (TeamId::new(750).unwrap(), TeamName::new("other").unwrap())
            ]
        );
        reparses(&merged);
    }

    #[test]
    fn reconcile_resizes_a_placeholder_shorter_than_the_name_column() {
        // A one-cell placeholder claiming 705: the incoming team takes the
        // slot and the cells grow to cover the name column.
        let working = list("ID\tName\tNotes\n705\n");
        let incoming = list("ID\tName\n705\t/xx/\n");
        let (merged, summary) = reconcile(&working, &incoming);

        assert_eq!(
            summary.added,
            [(TeamId::new(705).unwrap(), TeamName::new("xx").unwrap())]
        );
        assert!(matches!(
            merged.rows(),
            [Row::Team { cells, id, name }]
                if *id == TeamId::new(705).unwrap()
                    && name.as_str() == "/xx/"
                    && cells == &["705".to_owned(), "/xx/".to_owned()]
        ));
        reparses(&merged);
    }

    #[test]
    fn reconcile_reverts_a_collision_cascade() {
        // Reverting one collision can reveal another: /a/ takes 702 while /b/
        // still holds it and is reverted — and now /c/'s appended 701 collides
        // with the restored /a/, so a second pass must revert /c/ too.
        let working = list("ID\tName\n701\t/a/\n702\t/b/\n");
        let incoming = list("ID\tName\n702\t/a/\n701\t/c/\n");
        let (merged, summary) = reconcile(&working, &incoming);

        assert_eq!(merged, working);
        assert_eq!(
            summary.unresolved,
            [
                (TeamId::new(702).unwrap(), TeamName::new("a").unwrap()),
                (TeamId::new(701).unwrap(), TeamName::new("c").unwrap())
            ]
        );
        assert!(summary.added.is_empty());
        assert_eq!(summary.kept, 2);
        reparses(&merged);
    }

    #[test]
    fn reconcile_override_consumes_the_placeholder_holding_its_new_id() {
        // /a/ moves to 702, which a placeholder claims: the placeholder's
        // slot is freed, so the merged list holds exactly one row claiming
        // 702 and still parses under the file's duplicate rule.
        let working = list("ID\tName\n701\t/a/\n702\tBackup 1\n");
        let incoming = list("ID\tName\n702\t/a/\n");
        let (merged, summary) = reconcile(&working, &incoming);

        assert_eq!(
            summary.overridden,
            [(
                TeamName::new("a").unwrap(),
                TeamId::new(701).unwrap(),
                TeamId::new(702).unwrap()
            )]
        );
        assert!(summary.added.is_empty());
        assert!(summary.unresolved.is_empty());
        assert!(matches!(
            merged.rows(),
            [Row::Team { id, name, .. }]
                if *id == TeamId::new(702).unwrap() && name.as_str() == "/a/"
        ));
        reparses(&merged);
    }

    #[test]
    fn reconcile_reports_unresolved_collisions() {
        let working = list("ID\tName\n701\t/co/\n702\t/a/\n");
        // New name but existing id -> unresolved by id.
        let (merged, summary) = reconcile(&working, &list("ID\tName\n701\t/other/\n"));
        assert_eq!(
            summary.unresolved,
            [(TeamId::new(701).unwrap(), TeamName::new("other").unwrap())]
        );
        assert_eq!(merged.teams().count(), 2);
        assert!(merged.id_of(&TeamName::new("other").unwrap()).is_none());
        reparses(&merged);

        // Existing name but the incoming id belongs to a different row ->
        // unresolved by name.
        let (merged, summary) = reconcile(&working, &list("ID\tName\n702\t/co/\n"));
        assert_eq!(
            summary.unresolved,
            [(TeamId::new(702).unwrap(), TeamName::new("co").unwrap())]
        );
        assert_eq!(
            merged.id_of(&TeamName::new("co").unwrap()),
            TeamId::new(701).ok()
        );
        reparses(&merged);
    }

    #[test]
    fn reconcile_override_reverted_leaves_every_claim_unique() {
        // The unresolved form of the same shape: /a/ cannot move to 702
        // because /b/ sits there, so nothing changes and the working list's
        // claims stay valid.
        let working = list("ID\tName\n701\t/a/\n702\t/b/\n");
        let (merged, summary) = reconcile(&working, &list("ID\tName\n702\t/a/\n"));
        assert_eq!(
            summary.unresolved,
            [(TeamId::new(702).unwrap(), TeamName::new("a").unwrap())]
        );
        assert!(summary.added.is_empty());
        assert_eq!(merged, working);
        reparses(&merged);
    }

    #[test]
    fn columns_are_carried_in_any_header_order() {
        // `ID` and `Name` located by header position; other columns verbatim.
        let parsed = list("ID\tNote\tName\n701\tkeep\t/co/\n");
        assert_eq!(
            parsed.id_of(&TeamName::new("/co/").unwrap()),
            TeamId::new(701).ok()
        );
        assert_eq!(parsed.write(), "ID\tNote\tName\r\n701\tkeep\t/co/\r\n");

        let reordered = list("Name\tID\n/co/\t701\n");
        assert_eq!(
            reordered.id_of(&TeamName::new("/co/").unwrap()),
            TeamId::new(701).ok()
        );
        assert_eq!(reordered.write(), "Name\tID\r\n/co/\t701\r\n");
    }

    #[test]
    fn only_slash_wrapped_name_cells_are_teams() {
        // A bare token folds for export names but stays inert in the list:
        // only a cell that starts AND ends with `/` makes a team row.
        for text in ["701\tBackup\n", "701\t/co\n", "701\tco/\n"] {
            let parsed = list(&format!("ID\tName\n{text}"));
            assert!(
                matches!(parsed.rows(), [Row::Placeholder { .. }]),
                "{text:?} must be a placeholder"
            );
        }
        let parsed = list("ID\tName\n701\t/co/\n");
        assert_eq!(
            parsed.id_of(&TeamName::new("/co/").unwrap()),
            TeamId::new(701).ok()
        );
        // The bare-token placeholder still claims its numeric id.
        assert!(matches!(
            TeamsList::parse("ID\tName\n701\tBackup\n701\t/co/\n"),
            Err(TeamsListError::DuplicateId { line: 3, id: 701 })
        ));
    }

    #[test]
    fn placeholder_ids_share_the_id_space() {
        // A placeholder's numeric in-range id duplicates like a team's.
        assert_eq!(
            TeamsList::parse("ID\tName\n701\t/co/\n701\tBackup 1\n"),
            Err(TeamsListError::DuplicateId { line: 3, id: 701 })
        );
        // A blank or non-numeric placeholder id is exempt.
        assert!(TeamsList::parse("ID\tName\n701\t/co/\n\tBackup 1\nabc\tBackup 2\n").is_ok());
    }

    #[test]
    fn appended_cells_follow_the_header_labels() {
        // A different header order still carries every shared label: the
        // appended team's Notes lands in the working Notes column, and a
        // working column the incoming header does not have stays blank.
        let working = list("ID\tName\tNotes\tExtra\n701\t/co/\tx\te\n");
        let incoming = list("Name\tID\tNotes\n/zz/\t750\tn7\n");
        let (merged, summary) = reconcile(&working, &incoming);
        assert_eq!(
            summary.added,
            [(TeamId::new(750).unwrap(), TeamName::new("zz").unwrap())]
        );
        assert!(matches!(
            merged.rows().last(),
            Some(Row::Team { cells, .. })
                if cells == &["750".to_owned(), "/zz/".to_owned(), "n7".to_owned(), String::new()]
        ));
        reparses(&merged);
    }

    #[test]
    fn reconcile_appends_new_incoming_placeholders() {
        // A placeholder only in the incoming list is carried over verbatim.
        let working = list("ID\tName\n");
        let incoming = list("ID\tName\n701\tBackup 1\n");
        let (merged, summary) = reconcile(&working, &incoming);
        assert_eq!(summary.placeholders_added, ["Backup 1".to_owned()]);
        assert!(matches!(
            merged.rows(),
            [Row::Placeholder { cells }]
                if cells == &["701".to_owned(), "Backup 1".to_owned()]
        ));
        reparses(&merged);

        // An id the working list already claims leaves the placeholder out.
        let working = list("ID\tName\n701\t/co/\n");
        let (merged, summary) = reconcile(&working, &incoming);
        assert!(summary.placeholders_added.is_empty());
        assert_eq!(merged, working);
        reparses(&merged);

        // A working row claiming a different id does not block the append.
        let working = list("ID\tName\n799\t/co/\n");
        let (merged, summary) = reconcile(&working, &incoming);
        assert_eq!(summary.placeholders_added, ["Backup 1".to_owned()]);
        reparses(&merged);

        // A working placeholder showing the same raw Name does.
        let working = list("ID\tName\n701\tBackup 1\n");
        let incoming = list("ID\tName\n702\tBackup 1\n");
        let (merged, summary) = reconcile(&working, &incoming);
        assert!(summary.placeholders_added.is_empty());
        assert_eq!(merged, working);
        reparses(&merged);

        // Two new placeholders both carry over, in order.
        let working = list("ID\tName\n");
        let incoming = list("ID\tName\n701\tBackup 1\n702\tBackup 2\n");
        let (merged, summary) = reconcile(&working, &incoming);
        assert_eq!(
            summary.placeholders_added,
            ["Backup 1".to_owned(), "Backup 2".to_owned()]
        );
        assert_eq!(merged.rows().len(), 2);
        reparses(&merged);

        // An unrelated collision elsewhere does not touch the append.
        let working = list("ID\tName\n701\t/a/\n702\t/b/\n");
        let incoming = list("ID\tName\n702\t/a/\n750\tBackup 1\n");
        let (merged, summary) = reconcile(&working, &incoming);
        assert_eq!(summary.placeholders_added, ["Backup 1".to_owned()]);
        assert_eq!(
            summary.unresolved,
            [(TeamId::new(702).unwrap(), TeamName::new("a").unwrap())]
        );
        assert!(matches!(
            merged.rows().last(),
            Some(Row::Placeholder { cells })
                if cells == &["750".to_owned(), "Backup 1".to_owned()]
        ));
        reparses(&merged);
    }

    #[test]
    fn reconcile_placeholder_append_loses_to_an_incoming_team() {
        // The same id incoming as a team and as a placeholder: the team is
        // added, the placeholder dropped, nothing unresolved.
        let working = list("ID\tName\n");
        let incoming = TeamsList::from_parts(
            vec!["ID".to_owned(), "Name".to_owned()],
            0,
            1,
            vec![
                Row::Team {
                    cells: vec!["701".to_owned(), "/xx/".to_owned()],
                    id: TeamId::new(701).unwrap(),
                    name: TeamName::new("/xx/").unwrap(),
                },
                Row::Placeholder {
                    cells: vec!["701".to_owned(), "Backup 1".to_owned()],
                },
            ],
        );
        let (merged, summary) = reconcile(&working, &incoming);
        assert_eq!(
            summary.added,
            [(TeamId::new(701).unwrap(), TeamName::new("xx").unwrap())]
        );
        assert!(summary.unresolved.is_empty());
        assert!(summary.placeholders_added.is_empty());
        assert!(matches!(
            merged.rows(),
            [Row::Team { id, .. }] if *id == TeamId::new(701).unwrap()
        ));
        reparses(&merged);
    }

    #[test]
    fn reconcile_takes_a_placeholder_slot() {
        let working = list("ID\tName\n772\tBackup 1\n");
        let incoming = list("ID\tName\n772\t/new/\n");
        let (merged, summary) = reconcile(&working, &incoming);

        assert_eq!(
            summary.added,
            [(TeamId::new(772).unwrap(), TeamName::new("new").unwrap())]
        );
        assert!(summary.unresolved.is_empty());
        assert!(matches!(
            merged.rows(),
            [Row::Team { id, name, .. }]
                if *id == TeamId::new(772).unwrap() && name.as_str() == "/new/"
        ));
        assert_eq!(merged.write(), "ID\tName\r\n772\t/new/\r\n");
        reparses(&merged);
    }

    #[test]
    fn reconcile_validates_the_final_mapping() {
        // An id swap between two working rows is legal: after every incoming
        // change is applied, no id is held twice.
        let working = list("ID\tName\n701\t/co/\n702\t/a/\n");
        let incoming = list("ID\tName\n701\t/a/\n702\t/co/\n");
        let (merged, summary) = reconcile(&working, &incoming);

        assert_eq!(
            summary.overridden,
            [
                (
                    TeamName::new("co").unwrap(),
                    TeamId::new(701).unwrap(),
                    TeamId::new(702).unwrap()
                ),
                (
                    TeamName::new("a").unwrap(),
                    TeamId::new(702).unwrap(),
                    TeamId::new(701).unwrap()
                )
            ]
        );
        assert!(summary.unresolved.is_empty());
        // The rows stay in place; only the ids moved.
        assert_eq!(merged.write(), "ID\tName\r\n702\t/co/\r\n701\t/a/\r\n");
        reparses(&merged);

        // A one-sided move collides: `/a/` cannot take 701 while `/co/` keeps
        // it, so the change is reverted and reported.
        let (merged, summary) = reconcile(&working, &list("ID\tName\n701\t/a/\n"));
        assert_eq!(
            summary.unresolved,
            [(TeamId::new(701).unwrap(), TeamName::new("a").unwrap())]
        );
        assert_eq!(merged, working);
        reparses(&merged);
    }
}
