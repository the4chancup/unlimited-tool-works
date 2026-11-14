//! Identity resolution: the validated export's team name against the teams
//! list — the progression's last step before a consumer's run planning.

use teams_list::{TeamId, TeamName, TeamsList};

use crate::validate::ValidatedAestheticsExport;

/// What the export is: a listed team, or the referee package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportIdentity {
    /// A listed team; `TeamId` is strictly 701–920.
    Team {
        /// The teams-list id.
        id: TeamId,
        /// The canonical `/xx/` name.
        name: TeamName,
    },
    /// A referee export — rendered as the fixed game ID 999 only at format
    /// boundaries; no referee value is ever a `TeamId`.
    Referees,
}

/// A validated export with its identity resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAestheticsExport {
    /// The sanitized export.
    pub export: ValidatedAestheticsExport,
    /// Its team or referee identity.
    pub identity: ExportIdentity,
}

/// The canonical team name has no teams-list row (`team_name_unknown`).
#[derive(Debug, thiserror::Error)]
#[error("team {team_name} is not in the teams list")]
pub struct IdentityError {
    /// The canonical name that matched no row.
    pub team_name: TeamName,
}

impl ValidatedAestheticsExport {
    /// `/refs/` resolves to `Referees` without reading the list; anything else
    /// needs a teams-list row, else `IdentityError` (the Compiler's
    /// `team_name_unknown`).
    pub fn resolve_identity(
        self,
        teams_list: &TeamsList,
    ) -> Result<ResolvedAestheticsExport, IdentityError> {
        let identity = if self.team_name.is_referees() {
            ExportIdentity::Referees
        } else {
            match teams_list.id_of(&self.team_name) {
                Some(id) => ExportIdentity::Team {
                    id,
                    name: self.team_name.clone(),
                },
                None => {
                    return Err(IdentityError {
                        team_name: self.team_name.clone(),
                    });
                }
            }
        };
        Ok(ResolvedAestheticsExport {
            export: self,
            identity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::report;

    /// A minimal teams list holding only `714 /co/`.
    fn teams_list() -> TeamsList {
        TeamsList::parse("ID\tName\n714\t/co/\n").unwrap()
    }

    // TC-ID-03
    #[test]
    fn a_refs_export_resolves_without_a_teams_list_row() {
        let report = report(
            "refs Cup",
            &[("players.txt", 10)],
            &["Players/Keeper"],
            &[("players.txt", Ok(b"01 Keeper"))],
        );
        let resolved = report.validated.unwrap().resolve_identity(&teams_list());
        assert_eq!(resolved.unwrap().identity, ExportIdentity::Referees);
    }

    #[test]
    fn a_listed_team_resolves_to_its_id() {
        let report = report(
            "co Midcup Spring 2026",
            &[("Players/03 - A/face_high.fmdl", 10)],
            &[],
            &[],
        );
        let resolved = report.validated.unwrap().resolve_identity(&teams_list());
        let ExportIdentity::Team { id, name } = resolved.unwrap().identity else {
            panic!("a team export");
        };
        assert_eq!(id, TeamId::new(714).unwrap());
        assert_eq!(name.as_str(), "/co/");
    }

    #[test]
    fn an_unlisted_team_is_an_identity_error() {
        let report = report(
            "zz Midcup Spring 2026",
            &[("Players/03 - A/face_high.fmdl", 10)],
            &[],
            &[],
        );
        let error = report
            .validated
            .unwrap()
            .resolve_identity(&teams_list())
            .unwrap_err();
        assert_eq!(error.team_name.as_str(), "/zz/");
    }
}
