//! The Apply aesthetics patch panel (the draft's `patchPanel`): an
//! `egui::Window` that opens the `aesthetics_patch.toml` a Team compiler
//! run wrote and lists every value it would change in the open save in the
//! unsaved-changes table's columns (Item, Old value, New value, in the
//! views' terms), an Item's click selecting the player. The preview is the
//! open save as it stands against the patch applied to a copy of it,
//! recomputed when the save changes (an edit once it has settled, an undo,
//! another save opened), so Apply writes what the table shows, as one
//! undoable edit. A file that cannot be read, parsed or applied to this
//! save is refused in the command line's words, the save untouched; each
//! team or player of the patch the save lacks is a warning line.

use std::path::{Path, PathBuf};

use pes_savefile::EditFile;
use pes_savefile::interchange::aesthetics_patch::{AestheticsPatch, PatchError, PatchSkip};
use pes_savefile::model::PlayerEntry;
use pes_savefile::model::names::display_name;
use pes_savefile::ops::compare::compare_players;
use pes_savefile::schema::schema_for;
use pes_savefile::settings_toml::SettingsError;
use team_widgets::violations::SelectedPlayer;

use crate::view::comparator::{self, OpenSaveDialog, Place};
use crate::view::{Panel, SaveView, aatf_check, edit_settled, prompt, surrender_focus, terms};

/// The Open patch dialog: `rfd` on `.toml` files.
#[cfg(not(test))]
pub(super) fn default_patch_dialog() -> OpenSaveDialog {
    Box::new(|| {
        rfd::FileDialog::new()
            .add_filter("Aesthetics patch", &["toml"])
            .pick_file()
    })
}

/// `default_patch_dialog` under `cargo test`: a test reaching a real file
/// dialog panics; tests stub `view.patch_dialog`.
#[cfg(test)]
pub(super) fn default_patch_dialog() -> OpenSaveDialog {
    Box::new(|| panic!("a test reached a real file dialog: stub patch_dialog"))
}

/// The panel's state on `SaveView` (`view.patch`): the opened patch, its
/// preview and the save revision Apply left.
pub(super) struct Patch {
    /// The last opened file: its path and the patch it holds, or what
    /// refused it (`cannot read the file (…)`, a parse error).
    loaded: Option<(PathBuf, Result<AestheticsPatch, String>)>,
    /// The loaded patch against the open save and the save's
    /// `Session::revision` it was computed for; `apply`'s refusal is its
    /// `Err`. A revision is never drawn twice, across opens too, so it
    /// alone tells an edit, an undo or another save opened.
    preview: Option<(u64, Result<Preview, PatchError>)>,
    /// The save's revision right after Apply's edit: the Applied line
    /// stands while the save is still at it.
    applied: Option<u64>,
}

/// What the patch would change in the open save.
struct Preview {
    /// The summary line after the file's name: `team <id> <name>, players
    /// <slots>` for each team with a player the patch writes, joined by
    /// `; `.
    teams: String,
    /// Whether the patch writes at least one player of this save: with
    /// none, the warning lines are the whole preview.
    writes: bool,
    /// One warning line per team or player the save lacks, the summary's
    /// order.
    skips: Vec<String>,
    /// The table's rows: one per changed value, a player's together.
    rows: Vec<PatchRow>,
    /// The changed players as the patch leaves them: what Apply writes.
    patched: Vec<PlayerEntry>,
}

/// One row of the preview's table.
struct PatchRow {
    /// `(item, label, old, new)` as the changes table draws them: the
    /// player's name, the field's label, the two values.
    cells: (String, String, String, String),
    /// The player the Item cell's click selects (`None`: no roster holds
    /// him).
    select: Option<SelectedPlayer>,
}

/// What a click in the window asks for, done after the window is drawn.
enum Action {
    /// `Open patch…`.
    Open,
    /// `Apply`.
    Apply,
    /// An Item cell clicked: its player.
    Select(SelectedPlayer),
}

impl Patch {
    /// No patch opened.
    pub(super) fn new() -> Patch {
        Patch {
            loaded: None,
            preview: None,
            applied: None,
        }
    }

    /// Reads and parses the file at `path`, dropping the last patch's
    /// preview and Applied line.
    fn load(&mut self, path: PathBuf) {
        let patch = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read the file ({error})"))
            .and_then(|text| AestheticsPatch::parse(&text).map_err(|error| error.to_string()));
        self.loaded = Some((path, patch));
        self.preview = None;
        self.applied = None;
    }

    /// Recomputes the preview when the open save is no longer at the
    /// revision it was computed for: once per change, not every frame.
    fn refresh(&mut self, file: &EditFile, revision: u64) {
        let Some((_, Ok(patch))) = &self.loaded else {
            return;
        };
        if self
            .preview
            .as_ref()
            .is_some_and(|(computed, _)| *computed == revision)
        {
            return;
        }
        self.preview = Some((revision, preview(file, patch)));
    }
}

/// `patch` against `file`: `apply` on a copy, then each written player's
/// differences, old against new, one table row each, the item his name
/// in `file` (colour codes dropped). A player whose values the save
/// already holds has no rows and is left out. `Err` is `apply`'s refusal
/// (another version, another allocation scheme, a value a field cannot
/// hold).
fn preview(file: &EditFile, patch: &AestheticsPatch) -> Result<Preview, PatchError> {
    let mut copy = file.clone();
    let summary = patch.apply(&mut copy)?;
    let version = file.version();
    let schema = schema_for(version);
    let roster = comparator::roster_of(file);
    let mut rows = Vec::new();
    let mut patched = Vec::new();
    for &id in &summary.applied_players {
        let old = file
            .player(id)
            .expect("apply wrote a player the save holds");
        let new = copy
            .player(id)
            .expect("apply wrote a player the save holds");
        let diffs = compare_players(old, new, schema).map_err(SettingsError::from)?;
        if diffs.is_empty() {
            continue;
        }
        let item = display_name(&old.name);
        let select = roster.get(&id).copied().map(Place::select);
        for diff in &diffs {
            let term = terms::row_term(version, diff);
            rows.push(PatchRow {
                cells: (item.clone(), term.label, term.old, term.new),
                select,
            });
        }
        patched.push(new.clone());
    }
    let team_name = |id: u32| {
        patch
            .teams
            .iter()
            .find(|team| team.id == id)
            .map(|team| team.name.as_str())
            .expect("a summary's team is one of the patch's")
    };
    let mut teams = Vec::new();
    for &team in &summary.applied_teams {
        // A cup player's id is his team's id × 100 + his slot.
        let slots: Vec<String> = summary
            .applied_players
            .iter()
            .filter(|&&player| player / 100 == team)
            .map(|player| format!("{:02}", player % 100))
            .collect();
        if !slots.is_empty() {
            teams.push(format!(
                "team {team} {}, players {}",
                team_name(team),
                slots.join(", ")
            ));
        }
    }
    let skips = summary
        .skipped
        .iter()
        .map(|skip| match *skip {
            PatchSkip::TeamNotInSave { team } => format!(
                "Team {team} {} is not in this save: its players are skipped.",
                team_name(team)
            ),
            PatchSkip::PlayerNotInSave { player } => {
                format!("Player {player} is not in this save: skipped.")
            }
        })
        .collect();
    Ok(Preview {
        teams: teams.join("; "),
        writes: !summary.applied_players.is_empty(),
        skips,
        rows,
        patched,
    })
}

/// The window for one frame, while it is the open panel and a save is open:
/// the top row, the refusal or the skips, then the list. Once the edit has
/// settled (`edit_settled`) the preview is brought to the save's revision
/// before anything is drawn; while a drag or a text edit is in flight the
/// held one stays drawn, and Apply recomputes it before it writes.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let SaveView {
        panel,
        patch,
        session,
        team,
        filter,
        selected,
        tactics,
        patch_dialog,
        focused,
        ..
    } = view;
    if *panel != Some(Panel::Patch) {
        return;
    }
    let Some(session) = session.as_mut() else {
        return;
    };
    let revision = session.revision();
    // A preview is a save clone, the patch applied and a comparison per
    // player: recomputed on every frame of a slider drag or of typing, it
    // would slow the edit down, so it waits for the gesture's end.
    let (_, settled) = edit_settled(ui, *focused);
    if settled {
        patch.refresh(session.file(), revision);
    }
    let mut open = true;
    let action = comparator::panel_window(ui, "Apply aesthetics patch", "save_editor_patch")
        .open(&mut open)
        .show(ui.ctx(), |ui| contents(ui, patch, revision))
        .and_then(|response| response.inner)
        .flatten();
    if !open {
        *panel = None;
    }
    match action {
        Some(Action::Open) => {
            surrender_focus(ui);
            if let Some(path) = patch_dialog() {
                patch.load(path);
            }
        }
        Some(Action::Apply) => {
            // A preview held through a gesture is the save before it:
            // its players written back would undo the edit, so a stale
            // one is recomputed against the save as it stands first.
            patch.refresh(session.file(), revision);
            if let Some((_, Ok(preview))) = &patch.preview {
                session.edit_players(preview.patched.clone());
                patch.applied = Some(session.revision());
            }
        }
        Some(Action::Select(player)) => {
            // A listed player's team is one of the open file's.
            let team_tactics = session
                .file()
                .team(player.team_id)
                .map(|entry| &entry.tactics)
                .expect("a listed row's team is in the open file");
            aatf_check::select_player(player, selected, team, filter, team_tactics, tactics);
        }
        None => {}
    }
}

/// The window's contents: `Open patch…`, then once a patch previews its
/// summary and `Apply`; a refusal in the error colour in place of the
/// rest; the skips in the warning colour; the table, or the weak line an
/// empty one leaves when the patch writes a player of this save.
fn contents(ui: &mut egui::Ui, patch: &Patch, revision: u64) -> Option<Action> {
    let mut action = None;
    // The preview, or the refusal's text after the path.
    let shown: Option<(&Path, Result<&Preview, String>)> = match &patch.loaded {
        None => None,
        Some((path, Err(error))) => Some((path, Err(error.clone()))),
        Some((path, Ok(_))) => patch.preview.as_ref().map(|(_, preview)| {
            (
                path.as_path(),
                preview.as_ref().map_err(ToString::to_string),
            )
        }),
    };
    ui.horizontal_wrapped(|ui| {
        if ui
            .button("Open patch\u{2026}")
            .on_hover_text("Pick a patch the Team compiler wrote")
            .clicked()
        {
            action = Some(Action::Open);
        }
        if let Some((path, Ok(preview))) = &shown {
            ui.weak(summary_line(path, preview));
            if ui
                .add_enabled(!preview.rows.is_empty(), egui::Button::new("Apply"))
                .on_hover_text("Write the patch into the save as one undoable edit")
                .clicked()
            {
                action = Some(Action::Apply);
            }
        }
    });
    match shown {
        None => {}
        Some((path, Err(error))) => {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("{}: {error}", path.display()),
            );
        }
        Some((_, Ok(preview))) => {
            for skip in &preview.skips {
                ui.colored_label(ui.visuals().warn_fg_color, skip);
            }
            if preview.rows.is_empty() {
                // A patch writing no player of this save draws no weak
                // line: its warnings say it all.
                if preview.writes {
                    ui.weak(if patch.applied == Some(revision) {
                        "Applied. The changes stay unsaved until Save."
                    } else {
                        "The save already holds every value this patch writes."
                    });
                }
            } else if let Some(player) = table(ui, &preview.rows) {
                action = Some(Action::Select(player));
            }
        }
    }
    action
}

/// `<file name>: <teams>`, the file's name alone when no team of the
/// patch is in the save.
fn summary_line(path: &Path, preview: &Preview) -> String {
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy();
    if preview.teams.is_empty() {
        name.into_owned()
    } else {
        format!("{name}: {}", preview.teams)
    }
}

/// The changes table in a scroll area as tall as its rows, up to the
/// panel's remaining height, so a long patch scrolls inside the panel and a
/// short one leaves no empty room under it (the height not shrunk, the
/// panel grew some 200 points of blank under a five-row patch); the player
/// whose Item cell was clicked.
fn table(ui: &mut egui::Ui, rows: &[PatchRow]) -> Option<SelectedPlayer> {
    egui::ScrollArea::both()
        .id_salt("save_editor_patch_rows")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            prompt::changes_table(
                ui,
                "save_editor_patch_table",
                rows.iter().map(|row| &row.cells),
                true,
            )
        })
        .inner
        .and_then(|index| rows[index].select)
}
