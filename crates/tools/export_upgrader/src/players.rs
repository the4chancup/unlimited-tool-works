//! The face folders and the boots and gloves folders moved into player
//! folders (spec/export_upgrader.md "Builds player folders", "Keeps shared
//! folders"). Each face folder becomes its player folder under `Players/`.
//! Its name is the first non-empty one of: the face folder's name part, the
//! worn boots folder's, the worn gloves folder's, the save's name, the
//! number alone. Its portrait is renamed `portrait.dds` and its
//! `ingame_face` spellings are normalized to `ingame_face.toml`.
//!
//! A boots or gloves folder one player wears merges into his folder on a Fox
//! export. One worn by two or more, or any worn one on a pre-Fox export,
//! stays a named shared folder with a link file — unless `--legacy-ids`
//! (VGL27's legacy mode) is given, where a Fox folder merges into every
//! wearer instead. A byte clash keeps the folder shared in every mode
//! (`merge_kept_shared`); in legacy mode the wearers still record its old
//! ID, which the compiler packs the shared folder under. Each slot's
//! output folder lands in `PlayerFolders::folders` for the `settings.toml` generation
//! (`settings.rs`). A referee export's folders are `referees.rs`'s, and loose
//! mode pairs each item folder's wearer from its ID's position in its kind's
//! old block of 25.

use std::collections::{BTreeMap, BTreeSet};

use aesthetics_export::{FileKind, Marker, classify, is_txt_note};
use pes_version::Engine;
use studio_core::{Disposition, Message, Scope};
use vtree::ScopePath;

use crate::messages::{Code, INPUT, UnmigratedReason, tool_message, unmigrated};
use crate::roster::Roster;

/// What steps 2-4 make of the export's face and item folders.
#[derive(Debug, Default)]
pub(crate) struct PlayerFolders {
    /// Files to copy: input path → output path (both `/`-joined,
    /// root-relative).
    pub(crate) copies: Vec<(String, String)>,
    /// Files copied at their input paths below `Players/`, which step 7's name
    /// respells, renames and reference rewrites skip: a referee folder step 12
    /// leaves as it was (empty for a team export).
    pub(crate) untouched: Vec<(String, String)>,
    /// Empty marker files to write, output paths (`Players/15 -
    /// Snuffy/ingame_face.toml`, `Players/15 - Snuffy/Crocs.boots`).
    pub(crate) markers: Vec<String>,
    /// Every input file the `PlayerFolders` accounts for (copied, refused as part of a
    /// refused folder, or a marker it normalized): the unmigrated-content
    /// loop skips them.
    pub(crate) consumed: BTreeSet<String>,
    /// Input files the steps recognized as leftovers (step 10): the run's
    /// fallback reports each `leftover_dropped`, the run clean, rather than
    /// `unmigrated_content`. A file here is not in `consumed`.
    pub(crate) leftovers: BTreeSet<String>,
    /// The findings on the input, in input-path order.
    pub(crate) findings: Vec<Message>,
    /// Each slot's face-folder portrait: slot (1-23) → its input path, for
    /// step 7's comparison with `Portraits/`.
    pub(crate) portraits: BTreeMap<u8, String>,
    /// Each slot's output player folder (1-23), for step 8.
    pub(crate) folders: BTreeMap<u8, OutputFolder>,
    /// The `(kind, worn ID)` pairs the pairing's cross-kind rule claims as
    /// "no custom item of the kind": step 8 writes the hide ID and reports
    /// `custom_model_not_worn` for them (roster mode; empty in loose).
    pub(crate) not_worn: BTreeSet<(ItemKind, u32)>,
}

/// What came of the boots or gloves folder a player's worn ID names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemFolder {
    /// It migrated (merged into the player folder, or kept shared with a
    /// link): the ID the folder's name carries, which `legacy_ids.toml`
    /// records.
    Migrated(u32),
    /// It held no model (step 3): it claims the category — its wearer is
    /// `fpc_on` — but packs nothing, so it records no ID.
    Modelless,
}

/// One slot's output player folder as steps 3-4 leave it.
#[derive(Debug)]
pub(crate) struct OutputFolder {
    /// The folder's output path (`Players/15 - Snuffy`).
    pub(crate) path: String,
    /// The folder's name has a name part after its number: `Players/03 -
    /// Ann`, not `Players/05`.
    pub(crate) named: bool,
    /// A boots folder of his worn ID was merged into it or is linked from
    /// it, or a modelless one claims the category.
    pub(crate) boots_folder: Option<ItemFolder>,
    /// The same for a gloves folder.
    pub(crate) gloves_folder: Option<ItemFolder>,
    /// A modelless boots or gloves folder of his worn ID marked him `fpc_on`
    /// (step 3): step 8 keeps an unclaimed category's stored 0 explicit.
    pub(crate) fpc_on: bool,
    /// The folder got the in-game face: it holds `ingame_face.toml`, which
    /// step 8 fills from the player's record (the empty marker file where
    /// the generation fails or there is no record).
    pub(crate) ingame_face: bool,
}

/// The slot a face folder's name gives: characters 4-5, after any three
/// characters (`XXX` or a baked-in team ID, `71416`), two ASCII digits making
/// 01-23.
pub(crate) fn slot_of(name: &str) -> Option<u8> {
    let digits = name.get(3..5)?;
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u8>().ok().filter(|nn| (1..=23).contains(nn))
}

/// A candidate name as a folder name can hold it: the characters `vtree`'s
/// `validate_segments` refuses — control characters and `< > : " / \ | ? *` —
/// dropped, then trailing `.` and spaces trimmed (a name cannot end in them
/// either: `T.O.M.` → `T.O.M`); an empty remainder counts as absent.
pub(crate) fn clean_folder_name(name: &str) -> Option<String> {
    let cleaned: String = name
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '/' | '\\')
        })
        .collect();
    let trimmed = cleaned.trim_end_matches(['.', ' ']);
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// One slot's player folder as `player_folders` builds it: the face folder's files
/// and every folder merged into it later, keyed by relative path for the
/// byte clash check.
struct PlayerFolder {
    /// The face folder's whole name (`XXX05 - Snuffy`); `""` when the slot
    /// has no kept face folder. The name-pairing reads it whole: an item
    /// folder's name part can spell it (`k2964 - 81514- LET ME DEFEND`).
    face_name: String,
    /// The face folder's name part, leading spaces and hyphens off; `""`
    /// when the slot has no kept face folder.
    face_name_part: String,
    /// The worn boots folder's name part, `""` when none.
    boots_name_part: String,
    /// The worn gloves folder's name part, `""` when none.
    gloves_name_part: String,
    /// A boots folder of his worn ID was merged into the folder or is linked
    /// from it (step 8's "the folder decides the category"), or a modelless
    /// one claims it.
    boots_folder: Option<ItemFolder>,
    /// The same for a gloves folder.
    gloves_folder: Option<ItemFolder>,
    /// A kept face folder exists for the slot.
    has_face: bool,
    /// The `ingame_face.toml` marker file to write.
    marker: bool,
    /// The `fpc_on` marker to write: a worn boots or gloves folder holding
    /// no model hides the player's boots or hands (step 3).
    fpc_on: bool,
    /// The folder's files: relative path → input path.
    files: BTreeMap<String, String>,
    /// The shared-folder link file names (`Crocs.boots`, `Mitts.gloves`).
    links: BTreeSet<String>,
}

impl PlayerFolder {
    fn new() -> PlayerFolder {
        PlayerFolder {
            face_name: String::new(),
            face_name_part: String::new(),
            boots_name_part: String::new(),
            gloves_name_part: String::new(),
            boots_folder: None,
            gloves_folder: None,
            has_face: false,
            marker: false,
            fpc_on: false,
            files: BTreeMap::new(),
            links: BTreeSet::new(),
        }
    }
}

/// A file's own name: the part after its last `/`.
pub(crate) fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// A path of the listing as a `ScopePath`: every listed path was checked
/// when the source opened.
pub(crate) fn path_scope(path: &str) -> ScopePath {
    ScopePath::new(path).expect("every listed path was checked when the source opened")
}

/// One kind of item folder (the boots folders' `k`, the gloves' `g`, lower
/// case as the old format requires).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ItemKind {
    Boots,
    Gloves,
}

impl ItemKind {
    /// Its top folder, matched in any letter case.
    fn top(self) -> &'static str {
        match self {
            ItemKind::Boots => "boots",
            ItemKind::Gloves => "gloves",
        }
    }

    /// Its name's first character.
    pub(crate) fn letter(self) -> char {
        match self {
            ItemKind::Boots => 'k',
            ItemKind::Gloves => 'g',
        }
    }

    /// The other kind: the pairing's cross-kind rules read it.
    fn other(self) -> ItemKind {
        match self {
            ItemKind::Boots => ItemKind::Gloves,
            ItemKind::Gloves => ItemKind::Boots,
        }
    }

    /// Its index where both kinds are kept in one order (Boots first).
    fn index(self) -> usize {
        match self {
            ItemKind::Boots => 0,
            ItemKind::Gloves => 1,
        }
    }

    /// The `settings.toml` key its worn ID writes (`boots_id`,
    /// `gloves_id`), in `custom_model_*` contexts.
    fn key(self) -> &'static str {
        match self {
            ItemKind::Boots => "boots_id",
            ItemKind::Gloves => "gloves_id",
        }
    }

    /// Its shared output folder.
    fn shared_top(self) -> &'static str {
        match self {
            ItemKind::Boots => "Boots",
            ItemKind::Gloves => "Gloves",
        }
    }

    /// Its link file's extension.
    fn link_ext(self) -> &'static str {
        match self {
            ItemKind::Boots => "boots",
            ItemKind::Gloves => "gloves",
        }
    }

    /// The worn ID a rostered player carries for this kind.
    fn worn_id(self, player: &crate::roster::RosteredPlayer) -> u32 {
        match self {
            ItemKind::Boots => player.boots_id,
            ItemKind::Gloves => player.gloves_id,
        }
    }

    /// The `.fmdl` stems the game loads in a Fox folder of this kind, each
    /// alone or after a prefix and `_` (`kit_boots.fmdl`, step 10's stray
    /// rule).
    fn fmdl_names(self) -> &'static [&'static str] {
        match self {
            ItemKind::Boots => &["boots"],
            ItemKind::Gloves => &["glove_l", "glove_r"],
        }
    }
}

/// An item folder's name: `k` (boots) or `g` (gloves), lower case, then four
/// ASCII digits, then its name part (leading spaces and hyphens trimmed, as
/// a face folder's); a gloves folder named `g` and exactly three digits and
/// nothing else reads as four (`g567` → ID 567). `None` for any other
/// spelling (`K0430`, `k430 - X`, `boots`).
pub(crate) fn parse_item_name(kind: ItemKind, name: &str) -> Option<(u32, String)> {
    let name = name.strip_prefix(kind.letter())?;
    if kind == ItemKind::Gloves && name.len() == 3 && name.bytes().all(|b| b.is_ascii_digit()) {
        return Some((name.parse().ok()?, String::new()));
    }
    let digits = name.get(..4)?;
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some((
        digits.parse().ok()?,
        name[4..].trim_start_matches([' ', '-']).to_owned(),
    ))
}

/// Whether `name`, a file's own name, is a stray the old format's compile
/// never loaded (step 10): a file `classify` reads as no known kind and
/// that is no `.txt` note, wherever it sits in the old format's folders,
/// or, for a Fox boots or gloves folder (`item`), an `.fmdl` under no name
/// the game loads there — the kind's names, each alone or after a prefix
/// and `_` (`boots - Copy.fmdl` beside `boots.fmdl`). Dropped as a
/// leftover; the folder migrates without it.
pub(crate) fn is_stray(name: &str, item: Option<(ItemKind, Engine)>) -> bool {
    if classify(name) == FileKind::Other && !is_txt_note(name) {
        return true;
    }
    let kind = match item {
        Some((kind, Engine::Fox)) => kind,
        Some((_, Engine::PreFox)) | None => return false,
    };
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return false;
    };
    if !extension.eq_ignore_ascii_case("fmdl") {
        return false;
    }
    let stem = vtree::fold_name(stem);
    !kind.fmdl_names().iter().any(|base| {
        stem == *base
            || stem
                .strip_suffix(base)
                .is_some_and(|prefix| prefix.ends_with('_'))
    })
}

/// The grouped files by folder under `top` (any letter case): every file
/// below a direct subfolder, mapped `top/folder` → its files. A file
/// directly in `top` belongs to none. `folders`' entries seed the map the
/// same way, the first two segments of each listed folder: a direct
/// subfolder holding no file below it (an empty folder) is a folder too.
fn folders_below<'a>(
    files: &'a [String],
    folders: &[String],
    top: &str,
) -> BTreeMap<String, Vec<&'a String>> {
    let mut grouped: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for file in files {
        let mut parts = file.splitn(3, '/');
        let (Some(head), Some(folder), Some(_)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if head.eq_ignore_ascii_case(top) {
            grouped
                .entry(format!("{head}/{folder}"))
                .or_default()
                .push(file);
        }
    }
    for folder in folders {
        let mut parts = folder.splitn(3, '/');
        let (Some(head), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        if head.eq_ignore_ascii_case(top) {
            let key = format!("{head}/{name}");
            // The same folder as a file group keeps the file's spelling.
            if !grouped
                .keys()
                .any(|group| vtree::fold_name(group) == vtree::fold_name(&key))
            {
                grouped.entry(key).or_default();
            }
        }
    }
    grouped
}

/// What steps 2-3 make of `files`' face folders for a team export (a referee
/// export's folders are `referees.rs`'s: `run` does not call this for one) with
/// `roster` for the names and empty slots (`None` in loose mode), and steps
/// 3-4 make of its boots and gloves folders for `engine`, `legacy_ids`
/// (`--legacy-ids`, VGL27's legacy mode: a Fox folder merges into every
/// wearer rather than staying shared) and `same_bytes`.
/// `folders` is the export's folder listing: a `Faces/`, `Boots/` or
/// `Gloves/` subfolder holding no file below it is still a folder (step 3's
/// empty face folder, and the modelless item folder).
pub(crate) fn player_folders(
    files: &[String],
    folders: &[String],
    roster: Option<&Roster>,
    engine: Engine,
    legacy_ids: bool,
    mut same_bytes: impl FnMut(&str, &str) -> bool,
) -> PlayerFolders {
    let mut plan = PlayerFolders::default();
    let mut players: BTreeMap<u8, PlayerFolder> = BTreeMap::new();
    // Step 3's face folders.
    face_folders(
        folders_below(files, folders, "faces"),
        roster,
        &mut players,
        &mut plan,
    );
    // Steps 3-4's item folders. The pairing needs both kinds' folders at
    // once — the cross-kind rules read the other kind's pairings — so it is
    // collected first, roster mode only; loose mode has none. Who wears
    // each folder is then the save's, the pairing's, or loose mode's
    // inference from the ID's position in its kind's block (step 11).
    let item_sets = [
        folders_below(files, folders, ItemKind::Boots.top()),
        folders_below(files, folders, ItemKind::Gloves.top()),
    ];
    let pairings = match roster {
        Some(roster) => pair_wearers(&item_sets[0], &item_sets[1], roster, &players),
        None => Pairings::default(),
    };
    plan.not_worn = pairings.not_worn;
    plan.findings.extend(pairings.findings);
    let [boots_set, gloves_set] = item_sets;
    let merge = match engine {
        Engine::PreFox => MergeMode::Never,
        Engine::Fox if legacy_ids => MergeMode::EveryWearer,
        Engine::Fox => MergeMode::OneWearer,
    };
    for (kind, set) in [(ItemKind::Boots, boots_set), (ItemKind::Gloves, gloves_set)] {
        item_folders(
            set,
            kind,
            roster.map(|roster| Wearers {
                roster,
                paired: &pairings.folders[kind.index()],
            }),
            merge,
            &mut same_bytes,
            &mut players,
            &mut plan,
        );
    }
    // The player folder names and copies, every slot in order: the name is
    // the first non-empty candidate, cleaned — the face folder's name part,
    // then the worn boots folder's, then the worn gloves folder's, then the
    // save's name, then `NN` alone.
    for (nn, player) in &mut players {
        let candidates = [
            player.face_name_part.as_str(),
            player.boots_name_part.as_str(),
            player.gloves_name_part.as_str(),
            roster
                .and_then(|roster| roster.get(*nn))
                .map_or("", |p| p.name.as_str()),
        ];
        let name = candidates
            .iter()
            .find_map(|candidate| clean_folder_name(candidate));
        let folder = match &name {
            Some(name) => format!("Players/{nn:02} - {name}"),
            None => format!("Players/{nn:02}"),
        };
        for (relative, source) in &player.files {
            plan.copies
                .push((source.clone(), format!("{folder}/{relative}")));
        }
        if player.marker {
            plan.markers.push(format!("{folder}/ingame_face.toml"));
        }
        if player.fpc_on {
            plan.markers.push(format!("{folder}/fpc_on"));
        }
        for link in &player.links {
            plan.markers.push(format!("{folder}/{link}"));
        }
        plan.folders.insert(
            *nn,
            OutputFolder {
                path: folder,
                named: name.is_some(),
                boots_folder: player.boots_folder,
                gloves_folder: player.gloves_folder,
                fpc_on: player.fpc_on,
                ingame_face: player.marker,
            },
        );
    }
    plan
}

/// Whether `files`, a face folder's files at `skip` bytes below their
/// input paths' root, are the game's own extracted path to its face
/// (step 3): every file below `common/character0/model/character/face/
/// real/` and one folder there, each segment's letter case aside, the
/// folder the same for every file. An empty folder is not one.
fn game_path(files: &[&String], skip: usize) -> bool {
    const REAL: [&str; 6] = ["common", "character0", "model", "character", "face", "real"];
    let mut inner: Option<&str> = None;
    for file in files {
        let mut segments = file[skip..].split('/');
        for known in REAL {
            if !segments
                .next()
                .is_some_and(|segment| segment.eq_ignore_ascii_case(known))
            {
                return false;
            }
        }
        let Some(segment) = segments.next() else {
            return false;
        };
        // A file directly at `real/`'s level is no innermost folder's.
        if segments.next().is_none() {
            return false;
        }
        match inner {
            None => inner = Some(segment),
            Some(seen) if !segment.eq_ignore_ascii_case(seen) => return false,
            _ => {}
        }
    }
    inner.is_some()
}

/// `relative` below its first seven segments (`common/character0/model/
/// character/face/real/<one folder>/`): the file's place in the face
/// folder once the game's own path is taken off.
fn below_game_path(relative: &str) -> &str {
    relative
        .splitn(8, '/')
        .nth(7)
        .expect("a `game_path` file has an eighth segment")
}

/// Step 3's face folders into `players`: each becomes its player folder,
/// the portrait renamed and the `ingame_face` spellings normalized to
/// `ingame_face.toml`.
fn face_folders(
    folders: BTreeMap<String, Vec<&String>>,
    roster: Option<&Roster>,
    players: &mut BTreeMap<u8, PlayerFolder>,
    plan: &mut PlayerFolders,
) {
    // Each slot's kept folder's input path, for the `other` context of a
    // second folder of the same slot.
    let mut taken: BTreeMap<u8, String> = BTreeMap::new();
    for (folder, folder_files) in folders {
        let folder_name = folder.rsplit('/').next().unwrap_or(folder.as_str());
        // Strays first (step 10): nothing after — not the refusals, not the
        // game-path check, not the file loop — sees them.
        let (strays, folder_files): (Vec<&String>, Vec<&String>) = folder_files
            .into_iter()
            .partition(|file| is_stray(file_name(file), None));
        plan.leftovers
            .extend(strays.iter().map(|file| (*file).clone()));
        let refused =
            |plan: &mut PlayerFolders, reason: UnmigratedReason, other: Option<String>| {
                plan.findings.push(unmigrated(
                    Scope::Folder {
                        export_id: INPUT,
                        path: path_scope(&folder),
                    },
                    Disposition::DropFolder,
                    reason,
                    other,
                    vec![],
                ));
                plan.consumed
                    .extend(folder_files.iter().map(|file| (*file).clone()));
            };
        // A slot that is not two digits making 01 to 23 is unmigrated content as
        // a whole folder, as is the second face folder of a slot (the first
        // by name is kept).
        let Some(nn) = slot_of(folder_name) else {
            refused(&mut *plan, UnmigratedReason::SlotInvalid, None);
            continue;
        };
        if let Some(first) = taken.get(&nn) {
            let first = first.clone();
            refused(&mut *plan, UnmigratedReason::SlotTaken, Some(first));
            continue;
        }
        taken.insert(nn, folder.clone());
        // The name part is what follows the five characters, its leading
        // spaces and hyphens trimmed.
        let name_part = folder_name
            .get(5..)
            .unwrap_or("")
            .trim_start_matches([' ', '-']);
        if roster.is_some_and(|roster| roster.get(nn).is_none()) {
            // An empty slot still becomes its player folder, with no
            // `settings.toml`; the compiler's validation rules on it as for
            // any export.
            plan.findings.push(tool_message(
                Code::RosterSlotEmpty,
                Scope::Folder {
                    export_id: INPUT,
                    path: path_scope(&folder),
                },
                Disposition::Keep,
                vec![("slot", format!("{nn:02}"))],
            ));
        }
        let player = players.entry(nn).or_insert_with(PlayerFolder::new);
        player.face_name = folder_name.to_owned();
        player.face_name_part = name_part.to_owned();
        player.has_face = true;
        // A face folder that is the game's own extracted path to its face:
        // the innermost folder's contents move instead, so the face the
        // old compile packed as it stood lands where the new format puts
        // it (step 3).
        let on_game_path = game_path(&folder_files, folder.len() + 1);
        // Every file lands under the player folder at its relative path,
        // except the portrait (`*portrait.dds`, renamed when alone) and the
        // `ingame_face` spellings (normalized to `ingame_face.toml`).
        let mut portraits = Vec::new();
        for file in &folder_files {
            plan.consumed.insert((*file).clone());
            let relative = &file[folder.len() + 1..];
            let relative = if on_game_path {
                below_game_path(relative)
            } else {
                relative
            };
            if relative.contains('/') {
                player.files.insert(relative.to_owned(), (*file).clone());
                continue;
            }
            if classify(relative) == FileKind::Marker(Marker::OldIngameFace) {
                player.marker = true;
            } else if relative.to_ascii_lowercase().ends_with("portrait.dds") {
                portraits.push((*file, relative));
            } else {
                player.files.insert(relative.to_owned(), (*file).clone());
            }
        }
        match portraits.as_slice() {
            [] => {}
            [(file, _)] => {
                player
                    .files
                    .insert("portrait.dds".to_owned(), (*file).clone());
                plan.portraits.insert(nn, (*file).clone());
            }
            many => {
                // One too many: each is reported, `other` naming another
                // candidate's input path, and left in the folder as named.
                for (index, &(file, relative)) in many.iter().enumerate() {
                    let other = many[if index == 0 { 1 } else { 0 }].0;
                    plan.findings.push(unmigrated(
                        Scope::File {
                            export_id: INPUT,
                            path: path_scope(file),
                        },
                        Disposition::Keep,
                        UnmigratedReason::PortraitTaken,
                        Some(other.clone()),
                        vec![],
                    ));
                    player.files.insert(relative.to_owned(), (*file).clone());
                }
            }
        }
    }
}

/// The player slot an ID's position in its kind's old block of 25 IDs gives
/// (step 11): `101 + 25 × n`, the block `lowest` falls in (`101 + 25 ×
/// ⌊(lowest − 101) / 25⌋` is the block's first ID). Positions 24-25, the
/// block's spare positions, and IDs past it name nobody.
fn loose_slot(id: u32, lowest: u32) -> Option<u8> {
    // `i64`: a `lowest` below 101 must not underflow.
    let range_start = 101 + 25 * (i64::from(lowest) - 101).div_euclid(25);
    let slot = u8::try_from(i64::from(id) - range_start + 1).ok()?;
    (1..=23).contains(&slot).then_some(slot)
}

/// A name as the wearer pairing compares it: the lowercase letters and
/// digits alone (`char::is_alphanumeric`, lowercased), so punctuation and
/// letter case cannot keep a match apart (`81514- LET ME DEFEND` folds to
/// `81514letmedefend`). An empty remainder names nobody.
fn fold_wearer_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// The rule a paired folder got its worn ID by, which the finding names.
enum PairRule {
    /// `wearer_by_name`.
    Name,
    /// `wearer_by_number`: the other kind's same-numbered folder it
    /// followed.
    Number(String),
    /// `wearer_by_order`.
    Order,
}

/// What the pairing pass makes of the two kinds' folders (roster mode):
/// each paired folder's worn ID and rule, per kind; the `(kind, ID)` pairs
/// the cross-kind "no custom item" rule claims (step 8's
/// `custom_model_not_worn`); and the `custom_model_unpaired` refusals.
#[derive(Default)]
struct Pairings {
    /// Paired folders by kind: folder path → (the paired worn ID, the
    /// rule).
    folders: [BTreeMap<String, (u32, PairRule)>; 2],
    /// The `(kind, ID)` pairs the "no custom item" rule claims.
    not_worn: BTreeSet<(ItemKind, u32)>,
    /// The refusal findings of the last rule.
    findings: Vec<Message>,
}

/// One kind's folders as the pairing pass reads them.
struct KindState<'a> {
    /// The parsed folders, first of each ID: (folder path, its number,
    /// folded name part).
    folders: Vec<(&'a String, u32, String)>,
    /// Every parsed ID of the kind, a second folder's of one ID included.
    carried: BTreeSet<u32>,
    /// Indexes into `folders` of the unworn ones (no save record wears
    /// their number, a modelless folder included).
    unworn: Vec<usize>,
    /// The missing IDs — a rostered player's worn custom ID (above 100) no
    /// folder of the kind carries — to their wearer slots.
    missing: BTreeMap<u32, Vec<u8>>,
    /// Paired folders: folder path → (the worn ID, the rule).
    paired: BTreeMap<String, (u32, PairRule)>,
}

/// `kind`'s folders as the pairing pass reads them.
fn kind_state<'a>(
    kind: ItemKind,
    folders: &'a BTreeMap<String, Vec<&String>>,
    roster: &Roster,
) -> KindState<'a> {
    let mut parsed = Vec::new();
    let mut carried = BTreeSet::new();
    let mut taken = BTreeSet::new();
    for folder in folders.keys() {
        let name = folder.rsplit('/').next().unwrap_or(folder.as_str());
        let Some((id, name_part)) = parse_item_name(kind, name) else {
            continue;
        };
        carried.insert(id);
        if !taken.insert(id) {
            continue;
        }
        parsed.push((folder, id, fold_wearer_name(&name_part)));
    }
    let unworn = parsed
        .iter()
        .enumerate()
        .filter_map(|(index, &(_, id, _))| {
            (!(1..=23).any(|nn| {
                roster
                    .get(nn)
                    .is_some_and(|player| kind.worn_id(player) == id)
            }))
            .then_some(index)
        })
        .collect();
    let mut missing: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
    for nn in 1..=23 {
        let Some(player) = roster.get(nn) else {
            continue;
        };
        let id = kind.worn_id(player);
        if id > 100 && !carried.contains(&id) {
            missing.entry(id).or_default().push(nn);
        }
    }
    KindState {
        folders: parsed,
        carried,
        unworn,
        missing,
        paired: BTreeMap::new(),
    }
}

/// The pairing pass (roster mode), over both kinds' folder listings at
/// once — the cross-kind rules read the other kind's pairings. In order:
/// name first (a missing ID pairs the one unworn folder named after its
/// wearers — two name-matching folders pair nothing); then a missing ID
/// that is a folder number of the other kind is claimed as "no custom
/// item"; then, over both kinds until neither pairs another folder, an
/// unworn folder follows its paired same-numbered other-kind folder when
/// the ID is missing of its own kind, and — only when that pairs nothing
/// more — a kind left with as many missing IDs as unworn folders pairs
/// them rank to rank; then a missing ID left beside an unworn folder
/// refuses.
fn pair_wearers(
    boots: &BTreeMap<String, Vec<&String>>,
    gloves: &BTreeMap<String, Vec<&String>>,
    roster: &Roster,
    players: &BTreeMap<u8, PlayerFolder>,
) -> Pairings {
    let mut kinds = [
        kind_state(ItemKind::Boots, boots, roster),
        kind_state(ItemKind::Gloves, gloves, roster),
    ];
    let mut pairings = Pairings::default();
    for kind in [ItemKind::Boots, ItemKind::Gloves] {
        let state = &mut kinds[kind.index()];
        for (id, wearers) in &state.missing {
            let mut names = BTreeSet::new();
            for &nn in wearers {
                if let Some(player_folder) = players.get(&nn) {
                    names.insert(fold_wearer_name(&player_folder.face_name));
                    names.insert(fold_wearer_name(&player_folder.face_name_part));
                }
                if let Some(player) = roster.get(nn) {
                    names.insert(fold_wearer_name(&player.name));
                }
            }
            names.retain(|name| !name.is_empty());
            let candidates: Vec<&String> = state
                .unworn
                .iter()
                .map(|&index| &state.folders[index])
                .filter(|(_, _, folded)| names.contains(folded))
                .map(|(folder, _, _)| *folder)
                .collect();
            // Two or more folders matching the ID's wearers: nothing pairs
            // by name.
            if let [folder] = candidates.as_slice() {
                state
                    .paired
                    .insert((*folder).clone(), (*id, PairRule::Name));
            }
        }
        for (folder, id) in state
            .paired
            .iter()
            .map(|(folder, (id, _))| (folder.clone(), *id))
            .collect::<Vec<_>>()
        {
            state.missing.remove(&id);
            state
                .unworn
                .retain(|&index| state.folders[index].0 != &folder);
        }
    }
    // A missing ID that is a folder number of the other kind is "no custom
    // item of the kind": claimed, and takes no part in what follows.
    for kind in [ItemKind::Boots, ItemKind::Gloves] {
        let other = kind.other();
        let ids: Vec<u32> = kinds[kind.index()].missing.keys().copied().collect();
        for id in ids {
            if kinds[other.index()].carried.contains(&id) {
                kinds[kind.index()].missing.remove(&id);
                pairings.not_worn.insert((kind, id));
            }
        }
    }
    // Over both kinds until neither pairs another folder, each kind in
    // turn: an unworn folder follows a paired same-numbered folder of the
    // other kind, taking the ID it paired with when that is missing of its
    // own kind — and only when that pairs nothing more does the kind's own
    // order rule pair its equal counts rank to rank. A kind's ordered
    // folders may feed the other kind's next round.
    loop {
        let mut progress = false;
        for kind in [ItemKind::Boots, ItemKind::Gloves] {
            // One pass pairs all the number rule can this turn: the other
            // kind's pairings do not change during it, and this kind's
            // missing IDs and unworn folders only shrink.
            let (state, other_state) = {
                let (boots, gloves) = kinds.split_at_mut(1);
                match kind {
                    ItemKind::Boots => (&mut boots[0], &gloves[0]),
                    ItemKind::Gloves => (&mut gloves[0], &boots[0]),
                }
            };
            for index in state.unworn.clone() {
                let (folder, number, _) = &state.folders[index];
                let Some(other_folder) = other_state
                    .folders
                    .iter()
                    .find(|(_, other_number, _)| other_number == number)
                    .map(|(folder, _, _)| *folder)
                else {
                    continue;
                };
                let Some(&(id, _)) = other_state.paired.get(other_folder) else {
                    continue;
                };
                if !state.missing.contains_key(&id) {
                    continue;
                }
                state.paired.insert(
                    (*folder).clone(),
                    (id, PairRule::Number(other_folder.clone())),
                );
                state.missing.remove(&id);
                state.unworn.retain(|&other_index| other_index != index);
                progress = true;
            }
            let state = &mut kinds[kind.index()];
            if !state.missing.is_empty() && state.missing.len() == state.unworn.len() {
                let mut by_number = state.unworn.clone();
                by_number.sort_by_key(|&index| state.folders[index].1);
                for (&index, id) in by_number
                    .iter()
                    .zip(state.missing.keys().copied().collect::<Vec<_>>())
                {
                    state
                        .paired
                        .insert(state.folders[index].0.clone(), (id, PairRule::Order));
                }
                state.missing.clear();
                state.unworn.clear();
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
    // A missing ID left beside an unworn folder of its kind: the save and
    // the export disagree — a refusal per wearer of each leftover ID.
    for kind in [ItemKind::Boots, ItemKind::Gloves] {
        let state = &kinds[kind.index()];
        if state.missing.is_empty() || state.unworn.is_empty() {
            continue;
        }
        for (id, wearers) in &state.missing {
            for &nn in wearers {
                pairings.findings.push(tool_message(
                    Code::CustomModelUnpaired,
                    Scope::Export { export_id: INPUT },
                    Disposition::Keep,
                    vec![
                        ("slot", format!("{nn:02}")),
                        ("key", kind.key().to_owned()),
                        ("id", id.to_string()),
                    ],
                ));
            }
        }
    }
    pairings.folders = [
        std::mem::take(&mut kinds[0].paired),
        std::mem::take(&mut kinds[1].paired),
    ];
    pairings
}

/// Roster mode's answer to "who may wear a folder of this kind": the save's
/// wearer map and the pass's pairings for it (`None` in loose mode, where
/// the wearer is the ID's position in the kind's old block, step 11).
#[derive(Clone, Copy)]
struct Wearers<'a> {
    /// The save's wearer map.
    roster: &'a Roster,
    /// The pass's pairings for the kind.
    paired: &'a BTreeMap<String, (u32, PairRule)>,
}

/// How a worn item folder merges (step 3): the engine's and
/// `--legacy-ids`'s joint answer.
#[derive(Clone, Copy)]
enum MergeMode {
    /// A Fox export's folder merges into its one wearer; more share it.
    OneWearer,
    /// A Fox export's folder under `--legacy-ids` (VGL27's legacy mode)
    /// merges into every wearer, a byte clash keeping the folder shared as
    /// outside the mode.
    EveryWearer,
    /// A pre-Fox export's folder never merges: every worn one stays shared.
    Never,
}

impl MergeMode {
    /// The engine the mode comes from: `Never` is pre-Fox's.
    fn engine(self) -> Engine {
        match self {
            MergeMode::OneWearer | MergeMode::EveryWearer => Engine::Fox,
            MergeMode::Never => Engine::PreFox,
        }
    }
}

/// Steps 3-4's item folders of `kind`: name, ID and wearers resolve, then
/// each is merged into its one wearer's folder (Fox only), or kept shared
/// with a link — `merge` says how (`MergeMode`). `roster` is the save's
/// wearer map, `None` in loose mode, where the wearer is the ID's
/// position in the kind's block (step 11).
fn item_folders(
    folders: BTreeMap<String, Vec<&String>>,
    kind: ItemKind,
    roster: Option<Wearers>,
    merge: MergeMode,
    same_bytes: &mut impl FnMut(&str, &str) -> bool,
    players: &mut BTreeMap<u8, PlayerFolder>,
    plan: &mut PlayerFolders,
) {
    /// A shared folder as the inputs give it: name part or the ID spelling,
    /// colliding names resolved below.
    struct Shared {
        id: u32,
        name_part: String,
        files: Vec<(String, String)>,
        wearers: Vec<u8>,
    }
    /// Whether `other` holds the same files as `first`: the same relative
    /// paths, letter case aside, each byte for byte.
    fn same_files(
        first: &Shared,
        other: &Shared,
        same_bytes: &mut impl FnMut(&str, &str) -> bool,
    ) -> bool {
        fn by_folded_path(item: &Shared) -> BTreeMap<String, &String> {
            item.files
                .iter()
                .map(|(input, relative)| (vtree::fold_name(relative), input))
                .collect()
        }
        let (first_files, other_files) = (by_folded_path(first), by_folded_path(other));
        first_files.len() == other_files.len()
            && other_files.iter().all(|(path, input)| {
                first_files
                    .get(path)
                    .is_some_and(|first_input| same_bytes(first_input, input))
            })
    }
    // Each ID's kept folder's input path, for the `other` context of a
    // second folder of the same ID.
    let mut taken: BTreeMap<u32, String> = BTreeMap::new();
    let mut shared: Vec<Shared> = Vec::new();
    // Loose mode's block is inferred from the kind's own lowest folder ID
    // (step 11): one kind's numbering cannot move the other's pairings.
    let lowest: Option<u32> = folders
        .keys()
        .filter_map(|folder| {
            parse_item_name(kind, folder.rsplit('/').next().unwrap_or(folder.as_str()))
                .map(|(id, _)| id)
        })
        .min();
    for (folder, folder_files) in folders {
        let folder_name = folder.rsplit('/').next().unwrap_or(folder.as_str());
        // Strays first (step 10): nothing after — not the refusals, not the
        // shared-folder comparison, the merge or the modelless check —
        // sees them.
        let (strays, folder_files): (Vec<&String>, Vec<&String>) = folder_files
            .into_iter()
            .partition(|file| is_stray(file_name(file), Some((kind, merge.engine()))));
        plan.leftovers
            .extend(strays.iter().map(|file| (*file).clone()));
        let refused =
            |plan: &mut PlayerFolders, reason: UnmigratedReason, other: Option<String>| {
                plan.findings.push(unmigrated(
                    Scope::Folder {
                        export_id: INPUT,
                        path: path_scope(&folder),
                    },
                    Disposition::DropFolder,
                    reason,
                    other,
                    vec![],
                ));
                plan.consumed
                    .extend(folder_files.iter().map(|file| (*file).clone()));
            };
        // Any other spelling is unmigrated content as a whole folder, as is
        // the second folder of one kind with one ID (the first by name) and
        // a folder nobody wears — no rostered player's worn ID, and in loose
        // mode an ID on a spare position or past the block.
        let Some((id, name_part)) = parse_item_name(kind, folder_name) else {
            refused(&mut *plan, UnmigratedReason::IdInvalid, None);
            continue;
        };
        if let Some(first) = taken.get(&id) {
            let first = first.clone();
            refused(&mut *plan, UnmigratedReason::IdTaken, Some(first));
            continue;
        }
        taken.insert(id, folder.clone());
        let mut wearers: Vec<u8> = match roster {
            Some(w) => (1..=23)
                .filter(|nn| {
                    w.roster
                        .get(*nn)
                        .is_some_and(|player| kind.worn_id(player) == id)
                })
                .collect(),
            // Loose mode (step 11): the ID's position in its kind's block.
            None => loose_slot(id, lowest.expect("this folder's own ID is among them"))
                .into_iter()
                .collect(),
        };
        let mut pairing: Option<(u32, &PairRule)> = None;
        if wearers.is_empty() {
            // With a roster nobody wears it: the pass may still have paired
            // it, by name, number or order; otherwise its files are
            // leftovers (step 3). Loose mode keeps the refusal: a folder
            // it cannot place may be anyone's (step 11).
            match roster {
                Some(w) => match w.paired.get(&folder) {
                    Some(&(paired_id, ref rule)) => {
                        wearers = (1..=23)
                            .filter(|nn| {
                                w.roster
                                    .get(*nn)
                                    .is_some_and(|player| kind.worn_id(player) == paired_id)
                            })
                            .collect();
                        pairing = Some((paired_id, rule));
                    }
                    None => {
                        plan.leftovers
                            .extend(folder_files.iter().map(|file| (*file).clone()));
                        continue;
                    }
                },
                None => {
                    refused(&mut *plan, UnmigratedReason::LooseUnplaceable, None);
                    continue;
                }
            }
        }
        // Step 11's assumed pairing is stated; a pass pairing is reported
        // the same way, the save's worn ID named per wearer so the member
        // can check it.
        match (roster, pairing) {
            (None, _) => plan.findings.push(tool_message(
                Code::WearerAssumed,
                Scope::Folder {
                    export_id: INPUT,
                    path: path_scope(&folder),
                },
                Disposition::Keep,
                vec![
                    ("id", format!("{id:04}")),
                    ("slot", format!("{:02}", wearers[0])),
                ],
            )),
            (Some(_), Some((paired_id, rule))) => {
                for &nn in &wearers {
                    let mut context =
                        vec![("slot", format!("{nn:02}")), ("id", paired_id.to_string())];
                    let code = match rule {
                        PairRule::Name => Code::WearerByName,
                        PairRule::Order => Code::WearerByOrder,
                        PairRule::Number(from) => {
                            context.push(("from", from.clone()));
                            Code::WearerByNumber
                        }
                    };
                    plan.findings.push(tool_message(
                        code,
                        Scope::Folder {
                            export_id: INPUT,
                            path: path_scope(&folder),
                        },
                        Disposition::Keep,
                        context,
                    ));
                }
            }
            (Some(_), None) => {}
        }
        // A folder holding no model (nothing, or a `boots.skl` alone) is
        // how a member hid the player's boots or hands: each wearer gets
        // the `fpc_on` marker and his category is accounted for, the
        // folder's files are leftovers, and its name part names nobody
        // (step 3).
        let modelless = !folder_files
            .iter()
            .any(|file| matches!(classify(file_name(file)), FileKind::Model(_)));
        if modelless {
            for &nn in &wearers {
                let player = players.entry(nn).or_insert_with(PlayerFolder::new);
                // As for a worn folder's wearer: no kept face folder still
                // means `ingame_face.toml`.
                player.marker = player.marker || !player.has_face;
                player.fpc_on = true;
                match kind {
                    ItemKind::Boots => player.boots_folder = Some(ItemFolder::Modelless),
                    ItemKind::Gloves => player.gloves_folder = Some(ItemFolder::Modelless),
                }
            }
            plan.leftovers
                .extend(folder_files.iter().map(|file| (*file).clone()));
            continue;
        }
        for &nn in &wearers {
            let player = players.entry(nn).or_insert_with(PlayerFolder::new);
            // A wearer with no kept face folder still gets the
            // `ingame_face.toml` marker file.
            player.marker = player.marker || !player.has_face;
            match kind {
                ItemKind::Boots => {
                    player.boots_name_part = name_part.clone();
                    player.boots_folder = Some(ItemFolder::Migrated(id));
                }
                ItemKind::Gloves => {
                    player.gloves_name_part = name_part.clone();
                    player.gloves_folder = Some(ItemFolder::Migrated(id));
                }
            }
        }
        // How the folder merges: `Never` (pre-Fox) keeps every worn folder
        // shared, `OneWearer` merges it into its one wearer and shares it
        // between two or more, `EveryWearer` (`--legacy-ids`, VGL27) merges
        // it into every wearer, so each one's own package carries the
        // folder's one old ID. A path the item folder and a merging player
        // folder share, folded (`Skin.dds` and `skin.dds` are one on
        // Windows), with different bytes is a clash, the first such path in
        // the finding's context: `merge_kept_shared` keeps the folder shared
        // in every mode; in legacy mode the wearers still record its old ID,
        // which the compiler packs the shared folder under.
        let never_merges = matches!(merge, MergeMode::Never);
        let legacy = matches!(merge, MergeMode::EveryWearer);
        // The wearers the folder merges into: all of them in legacy mode,
        // the one outside it (a share never reaches the merge).
        let merging: &[u8] = if legacy { &wearers } else { &wearers[..1] };
        let mut share = never_merges || (!legacy && wearers.len() > 1);
        if !share {
            let clash = merging.iter().find_map(|&nn| {
                folder_files.iter().find_map(|file| {
                    let relative = &file[folder.len() + 1..];
                    let folded = vtree::fold_name(relative);
                    players[&nn]
                        .files
                        .iter()
                        .find(|(path, _)| vtree::fold_name(path) == folded)
                        .and_then(|(_, existing)| {
                            (!same_bytes(existing, file)).then(|| relative.to_owned())
                        })
                })
            });
            if let Some(path) = clash {
                plan.findings.push(tool_message(
                    Code::MergeKeptShared,
                    Scope::Folder {
                        export_id: INPUT,
                        path: path_scope(&folder),
                    },
                    Disposition::Keep,
                    vec![("path", path)],
                ));
                share = true;
            }
        }
        plan.consumed
            .extend(folder_files.iter().map(|file| (*file).clone()));
        if share {
            shared.push(Shared {
                id,
                name_part,
                files: folder_files
                    .iter()
                    .map(|file| ((*file).clone(), file[folder.len() + 1..].to_owned()))
                    .collect(),
                wearers,
            });
        } else {
            for &nn in merging {
                let player = players
                    .get_mut(&nn)
                    .expect("the wearer's folder was ensured");
                for file in &folder_files {
                    let relative = &file[folder.len() + 1..];
                    let folded = vtree::fold_name(relative);
                    // A folded-equal path the player folder already holds
                    // keeps its own file: byte-equal copies merge, and a
                    // clash's finding names the folder file left out.
                    if player
                        .files
                        .keys()
                        .any(|path| vtree::fold_name(path) == folded)
                    {
                        continue;
                    }
                    player.files.insert(relative.to_owned(), (*file).clone());
                }
            }
        }
    }
    // Two shared folders of one kind whose names match, letter case aside,
    // compared each with the lowest-ID one of its name. One holding the same
    // files (the same paths folded, each byte for byte) is the same model
    // under another ID: it merges — its files are not copied and its wearers
    // link the lowest ID's folder name. One that differs takes its ID
    // spelling, the links following, so no wearer loads another's model.
    let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut names: Vec<String> = shared
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let name = clean_folder_name(&item.name_part)
                .unwrap_or_else(|| format!("{}{:04}", kind.letter(), item.id));
            by_name
                .entry(vtree::fold_name(&name))
                .or_default()
                .push(index);
            name
        })
        .collect();
    let mut merged = vec![false; shared.len()];
    for indexes in by_name.values_mut() {
        indexes.sort_by_key(|&index| shared[index].id);
        let first = indexes[0];
        for &index in &indexes[1..] {
            if same_files(&shared[first], &shared[index], same_bytes) {
                names[index] = names[first].clone();
                merged[index] = true;
            } else {
                names[index] = format!("{}{:04}", kind.letter(), shared[index].id);
            }
        }
    }
    for (index, (item, name)) in shared.iter().zip(&names).enumerate() {
        if !merged[index] {
            for (file, relative) in &item.files {
                plan.copies.push((
                    file.clone(),
                    format!("{}/{name}/{relative}", kind.shared_top()),
                ));
            }
        }
        for &nn in &item.wearers {
            players
                .get_mut(&nn)
                .expect("a wearer's folder was ensured")
                .links
                .insert(format!("{name}.{}", kind.link_ext()));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use studio_core::Severity;

    use super::*;
    use crate::roster::RosteredPlayer;

    /// A roster of (slot → name) pairs; boots and gloves are 0.
    fn roster(entries: &[(u8, &str)]) -> Roster {
        Roster(
            entries
                .iter()
                .map(|(slot, name)| {
                    (
                        *slot,
                        RosteredPlayer {
                            id: 71400 + u32::from(*slot),
                            name: (*name).to_owned(),
                            boots_id: 0,
                            gloves_id: 0,
                        },
                    )
                })
                .collect::<BTreeMap<u8, RosteredPlayer>>(),
        )
    }

    /// A roster of (slot → (name, boots_id, gloves_id)) triples.
    fn wearing(entries: &[(u8, (&str, u32, u32))]) -> Roster {
        Roster(
            entries
                .iter()
                .map(|(slot, (name, boots, gloves))| {
                    (
                        *slot,
                        RosteredPlayer {
                            id: 71400 + u32::from(*slot),
                            name: (*name).to_owned(),
                            boots_id: *boots,
                            gloves_id: *gloves,
                        },
                    )
                })
                .collect::<BTreeMap<u8, RosteredPlayer>>(),
        )
    }

    fn files(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    /// The folders a real listing of `files` holds: every file's ancestors.
    /// A test needing a folder no file sits below calls `player_folders`
    /// itself.
    fn ancestors_of(files: &[String]) -> Vec<String> {
        let mut folders = BTreeSet::new();
        for file in files {
            let mut ancestor = file.as_str();
            while let Some((parent, _)) = ancestor.rsplit_once('/') {
                folders.insert(parent.to_owned());
                ancestor = parent;
            }
        }
        folders.into_iter().collect()
    }

    /// `player_folders` with `bytes` as the byte-comparison's answers: each
    /// input path to its bytes; a path not listed answers `false` (as a read
    /// failure does).
    fn plan_with_bytes(
        files: &[String],
        roster: Option<&Roster>,
        engine: Engine,
        bytes: &[(&str, &[u8])],
    ) -> PlayerFolders {
        let bytes_of = |path: &str| {
            bytes
                .iter()
                .find(|(listed, _)| *listed == path)
                .map(|(_, bytes)| *bytes)
        };
        player_folders(
            files,
            &ancestors_of(files),
            roster,
            engine,
            false,
            |a, b| match (bytes_of(a), bytes_of(b)) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            },
        )
    }

    /// `player_folders` with no roster, Fox, and a byte check that answers
    /// false to every pair (nothing is ever byte-equal).
    fn plan_folders(files: &[String]) -> PlayerFolders {
        plan_with_bytes(files, None, Engine::Fox, &[])
    }

    /// The `code` finding on `folder`'s Folder scope in `plan` (panics
    /// when absent).
    fn finding_at<'a>(plan: &'a PlayerFolders, code: &str, folder: &str) -> &'a Message {
        plan.findings
            .iter()
            .find(|m| {
                m.code.code == code
                    && matches!(
                        &m.scope,
                        Scope::Folder { path, .. } if path.as_str() == folder
                    )
            })
            .unwrap_or_else(|| panic!("{code} on {folder}: {:?}", plan.findings))
    }

    #[test]
    fn the_slot_is_two_digits_making_01_to_23() {
        for (name, accepted) in [
            ("XXX01", true),
            ("XXX15", true),
            ("71416", true),
            ("XXX23", true),
            ("XXX00", false),
            ("XXX24", false),
            ("XXXab", false),
            ("XX15", false),
            ("ab15x", false),
        ] {
            let plan = plan_with_bytes(
                &files(&[&format!("Faces/{name}/face.fmdl")]),
                None,
                Engine::Fox,
                &[],
            );
            assert_eq!(
                plan.copies.is_empty(),
                !accepted,
                "{name}: {:?}",
                plan.findings
            );
            if !accepted {
                assert!(plan.consumed.contains(&format!("Faces/{name}/face.fmdl")));
                assert_eq!(
                    plan.findings[0].scope,
                    Scope::Folder {
                        export_id: INPUT,
                        path: ScopePath::new(&format!("Faces/{name}")).unwrap()
                    },
                    "{name}"
                );
                assert_eq!(
                    plan.findings[0].context,
                    [("reason".to_owned(), "slot_invalid".to_owned())],
                    "{name}"
                );
            }
        }
    }

    #[test]
    fn the_first_folder_of_a_slot_by_name_is_kept() {
        let plan = plan_folders(&files(&[
            "Faces/XXX15 - Alpha/face.fmdl",
            "Faces/XXX15 - Beta/face.fmdl",
        ]));
        let refused: Vec<&Message> = plan
            .findings
            .iter()
            .filter(|m| m.code.code == "unmigrated_content")
            .collect();
        assert_eq!(refused.len(), 1);
        assert_eq!(
            refused[0].scope,
            Scope::Folder {
                export_id: INPUT,
                path: ScopePath::new("Faces/XXX15 - Beta").unwrap()
            }
        );
        // The second of one slot: `other` names the kept folder's input path.
        assert_eq!(
            refused[0].context,
            [
                ("reason".to_owned(), "slot_taken".to_owned()),
                ("other".to_owned(), "Faces/XXX15 - Alpha".to_owned()),
            ]
        );
        assert!(
            plan.copies
                .iter()
                .any(|(_, to)| to.starts_with("Players/15 - Alpha/"))
        );
        assert!(!plan.copies.iter().any(|(_, to)| to.contains("Beta")));
    }

    #[test]
    fn the_name_comes_from_the_face_folder_then_the_save_then_the_number() {
        // The face folder's own name part wins over the save's.
        let list = files(&["Faces/XXX09 - FaceName/face.fmdl"]);
        let plan = plan_with_bytes(&list, Some(&roster(&[(9, "SaveName")])), Engine::Fox, &[]);
        assert_eq!(plan.copies[0].1, "Players/09 - FaceName/face.fmdl");
        // Without a name part the save's name is taken.
        let list = files(&["Faces/XXX09/face.fmdl"]);
        let plan = plan_with_bytes(&list, Some(&roster(&[(9, "SaveName")])), Engine::Fox, &[]);
        assert_eq!(plan.copies[0].1, "Players/09 - SaveName/face.fmdl");
        // With neither, the number alone names the folder.
        let plan = plan_folders(&list);
        assert_eq!(plan.copies[0].1, "Players/09/face.fmdl");
        // A name part that cleans to nothing is absent too.
        let list = files(&["Faces/XXX09 -   ./face.fmdl"]);
        let plan = plan_folders(&list);
        assert_eq!(plan.copies[0].1, "Players/09/face.fmdl");
    }

    #[test]
    fn a_cleaned_name_is_always_a_valid_segment() {
        for (input, expected) in [
            ("Snuffy", Some("Snuffy")),
            ("T.O.M.", Some("T.O.M")),
            ("a:b?c", Some("abc")),
            ("a<b>c", Some("abc")),
            ("a\"b|c", Some("abc")),
            ("a*b", Some("ab")),
            ("a/b\\c", Some("abc")),
            ("a b ", Some("a b")),
            ("x  .  ", Some("x")),
            ("...", None),
            ("   ", None),
        ] {
            let cleaned = clean_folder_name(input);
            assert_eq!(cleaned.as_deref(), expected, "{input:?}");
            if let Some(cleaned) = &cleaned {
                assert!(ScopePath::new(cleaned).is_ok(), "{cleaned:?}");
            }
        }
    }

    #[test]
    fn either_marker_spelling_becomes_one_ingame_face_toml() {
        for names in [
            vec!["ingame_face"],
            vec!["ingame_face.txt"],
            vec!["ingame_face", "ingame_face.txt"],
        ] {
            let files: Vec<String> = names
                .iter()
                .map(|name| format!("Faces/XXX05 - A/{name}"))
                .chain(std::iter::once("Faces/XXX05 - A/face.fmdl".to_owned()))
                .collect();
            let plan = plan_folders(&files);
            assert_eq!(
                plan.markers,
                ["Players/05 - A/ingame_face.toml".to_owned()],
                "{names:?}"
            );
            assert!(!plan.copies.iter().any(|(_, to)| to.contains("ingame_face")));
            assert!(
                plan.consumed
                    .contains(&format!("Faces/XXX05 - A/{}", names[0]))
            );
        }
    }

    #[test]
    fn one_portrait_is_renamed_and_two_are_reported_as_named() {
        let list = files(&[
            "Faces/XXX15 - A/x_portrait.dds",
            "Faces/XXX15 - A/face.fmdl",
        ]);
        let plan = plan_folders(&list);
        assert!(plan.copies.contains(&(
            "Faces/XXX15 - A/x_portrait.dds".to_owned(),
            "Players/15 - A/portrait.dds".to_owned()
        )));
        // Step 7's comparison map: the renamed portrait alone gives an
        // entry; two portraits as named give none.
        assert_eq!(
            plan.portraits.get(&15),
            Some(&"Faces/XXX15 - A/x_portrait.dds".to_owned())
        );
        let list = files(&[
            "Faces/XXX16 - A/portrait.dds",
            "Faces/XXX16 - A/alt_portrait.dds",
        ]);
        let plan = plan_folders(&list);
        let reported: Vec<&Message> = plan
            .findings
            .iter()
            .filter(|m| m.code.code == "unmigrated_content")
            .collect();
        assert_eq!(reported.len(), 2);
        // Each names the other candidate's input path.
        assert_eq!(
            reported[0].context,
            [
                ("reason".to_owned(), "portrait_taken".to_owned()),
                (
                    "other".to_owned(),
                    "Faces/XXX16 - A/alt_portrait.dds".to_owned()
                ),
            ]
        );
        assert_eq!(
            reported[1].context,
            [
                ("reason".to_owned(), "portrait_taken".to_owned()),
                (
                    "other".to_owned(),
                    "Faces/XXX16 - A/portrait.dds".to_owned()
                ),
            ]
        );
        assert!(
            plan.copies
                .iter()
                .any(|(_, to)| to == "Players/16 - A/portrait.dds")
        );
        assert!(
            plan.copies
                .iter()
                .any(|(_, to)| to == "Players/16 - A/alt_portrait.dds")
        );
        assert!(!plan.portraits.contains_key(&16));
    }

    #[test]
    fn an_empty_roster_slot_is_a_warning_and_the_folder_still_forms() {
        let files = files(&["Faces/XXX23 - Pinto/face.fmdl"]);
        let plan = plan_with_bytes(&files, Some(&roster(&[])), Engine::Fox, &[]);
        let warning = plan
            .findings
            .iter()
            .find(|m| m.code.code == "roster_slot_empty")
            .expect("roster_slot_empty expected");
        assert_eq!(warning.severity, Severity::Warning);
        assert_eq!(warning.disposition, Disposition::Keep);
        assert_eq!(warning.context, vec![("slot".to_owned(), "23".to_owned())]);
        assert!(
            plan.copies
                .iter()
                .any(|(_, to)| to.starts_with("Players/23 - Pinto/"))
        );
    }

    #[test]
    fn a_file_below_a_subfolder_of_another_top_folder_is_not_planned() {
        // A file below `Other/` is not a face folder's.
        let plan = plan_folders(&files(&[
            "Faces/XXX15 - A/face.fmdl",
            "Other/docs/readme.txt",
        ]));
        let scopes: Vec<String> = plan
            .findings
            .iter()
            .map(|m| match &m.scope {
                Scope::Folder { path, .. } | Scope::File { path, .. } => path.as_str().to_owned(),
                Scope::Run | Scope::Export { .. } | Scope::RosterEntry { .. } => String::new(),
            })
            .collect();
        let path = "Other/docs/readme.txt";
        assert!(!plan.consumed.contains(path), "{path}");
        assert!(
            !plan.copies.iter().any(|(_, to)| to.contains(path)),
            "{path}"
        );
        assert!(scopes.iter().all(|s| s != path), "{path}: {scopes:?}");
        assert_eq!(plan.copies.len(), 1);
    }

    #[test]
    fn the_item_folder_grammar() {
        // `k`/`g` lower case, four digits, name part trimmed; `g` and exactly
        // three digits alone reads as four with the `gNNNN` fallback name.
        for (kind, name, expected) in [
            (
                ItemKind::Boots,
                "k0430 - Crocs",
                Some((430, "Crocs".to_owned())),
            ),
            (ItemKind::Boots, "k0430", Some((430, String::new()))),
            (ItemKind::Gloves, "g567", Some((567, String::new()))),
            (
                ItemKind::Gloves,
                "g0432 Mitts",
                Some((432, "Mitts".to_owned())),
            ),
            (ItemKind::Gloves, "g567 - X", None),
            (ItemKind::Boots, "K0430", None),
            (ItemKind::Boots, "k430 - X", None),
            // Three digits alone read as four for gloves only.
            (ItemKind::Boots, "k430", None),
            (ItemKind::Boots, "boots", None),
        ] {
            assert_eq!(parse_item_name(kind, name), expected, "{name}");
        }
        // `g567`'s shared-name fallback is `g0567`, as the folder's ID spells.
        let files = files(&["Gloves/g567/glove_l.fmdl"]);
        let roster = wearing(&[(1, ("A", 0, 567)), (2, ("B", 0, 567))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        assert!(
            plan.copies
                .iter()
                .any(|(_, to)| to == "Gloves/g0567/glove_l.fmdl")
        );
        assert!(
            plan.markers
                .contains(&"Players/01 - A/g0567.gloves".to_owned())
        );
        assert!(
            plan.markers
                .contains(&"Players/02 - B/g0567.gloves".to_owned())
        );
    }

    #[test]
    fn two_shared_folders_of_one_kind_sharing_a_name_resolve_by_lowest_id() {
        // `k0430 - Crocs` and `k0431 - crocs`, each worn by two: the lowest ID
        // keeps the name, the other takes its ID spelling.
        let files = files(&[
            "Boots/k0430 - Crocs/boots.fmdl",
            "Boots/k0431 - crocs/boots.fmdl",
        ]);
        let roster = wearing(&[
            (1, ("A", 430, 0)),
            (2, ("B", 430, 0)),
            (3, ("C", 431, 0)),
            (4, ("D", 431, 0)),
        ]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        assert!(
            plan.copies
                .iter()
                .any(|(from, to)| from == "Boots/k0430 - Crocs/boots.fmdl"
                    && to == "Boots/Crocs/boots.fmdl")
        );
        assert!(
            plan.copies
                .iter()
                .any(|(from, to)| from == "Boots/k0431 - crocs/boots.fmdl"
                    && to == "Boots/k0431/boots.fmdl")
        );
        // Their player folders take the worn folders' name parts (`Crocs`,
        // `crocs`), and each link follows its shared folder's name.
        for marker in [
            "Players/01 - Crocs/Crocs.boots",
            "Players/02 - Crocs/Crocs.boots",
            "Players/03 - crocs/k0431.boots",
            "Players/04 - crocs/k0431.boots",
        ] {
            assert!(plan.markers.contains(&marker.to_owned()), "{marker}");
        }
    }

    #[test]
    fn two_same_named_shared_folders_merge_into_the_lowest_id_s_whatever_the_listing_order() {
        // `g567`'s three digits carry no name part, so it publishes `g0567`;
        // `g0600 - g0567` publishes the same and sorts first: the same-files
        // merge still goes into the lowest ID's files.
        let files = files(&[
            "Gloves/g0600 - g0567/glove_l.fmdl",
            "Gloves/g567/glove_l.fmdl",
        ]);
        let roster = wearing(&[(1, ("A", 0, 567)), (2, ("B", 0, 600))]);
        let plan = plan_with_bytes(
            &files,
            Some(&roster),
            Engine::PreFox,
            &[
                ("Gloves/g0600 - g0567/glove_l.fmdl", b"same"),
                ("Gloves/g567/glove_l.fmdl", b"same"),
            ],
        );
        assert_eq!(
            plan.copies,
            [(
                "Gloves/g567/glove_l.fmdl".to_owned(),
                "Gloves/g0567/glove_l.fmdl".to_owned()
            )]
        );
        // The link names the merged folder's name; slot 02's own folder
        // takes his worn folder's name part.
        for marker in [
            "Players/01 - A/g0567.gloves",
            "Players/02 - g0567/g0567.gloves",
        ] {
            assert!(plan.markers.contains(&marker.to_owned()), "{marker}");
        }
    }

    #[test]
    fn a_wearer_without_a_face_folder_is_named_by_the_item_then_roster() {
        // The worn boots folder's name part first, then the gloves folder's,
        // then the save's, then `NN` alone.
        let roster = wearing(&[(5, ("Save", 430, 432))]);
        for (paths, expected) in [
            (
                vec![
                    "Boots/k0430 - Crocs/boots.fmdl",
                    "Gloves/g0432 - Mitts/glove_l.fmdl",
                ],
                "Players/05 - Crocs/ingame_face.toml",
            ),
            (
                vec![
                    "Boots/k0430/boots.fmdl",
                    "Gloves/g0432 - Mitts/glove_l.fmdl",
                ],
                "Players/05 - Mitts/ingame_face.toml",
            ),
            (
                vec!["Boots/k0430/boots.fmdl", "Gloves/g0432/glove_l.fmdl"],
                "Players/05 - Save/ingame_face.toml",
            ),
        ] {
            let plan = plan_with_bytes(&files(&paths), Some(&roster), Engine::Fox, &[]);
            assert!(plan.markers.contains(&expected.to_owned()), "{paths:?}");
        }
        // And `NN` alone when nothing names it.
        let plan = plan_with_bytes(
            &files(&["Boots/k0430/boots.fmdl"]),
            Some(&wearing(&[(5, ("", 430, 0))])),
            Engine::Fox,
            &[],
        );
        assert!(
            plan.markers
                .contains(&"Players/05/ingame_face.toml".to_owned())
        );
    }

    #[test]
    fn a_gloves_folder_merges_into_a_player_folder_that_merged_boots() {
        // Boots are planned before gloves: both kinds land in the one
        // wearer's folder.
        let files = files(&[
            "Faces/XXX05 - A/face.fmdl",
            "Boots/k0430 - Crocs/boots.fmdl",
            "Gloves/g0432 - Mitts/glove_l.fmdl",
            "Gloves/g0432 - Mitts/glove_r.fmdl",
        ]);
        let roster = wearing(&[(5, ("A", 430, 432))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        for rel in ["face.fmdl", "boots.fmdl", "glove_l.fmdl", "glove_r.fmdl"] {
            assert!(
                plan.copies
                    .iter()
                    .any(|(_, to)| *to == format!("Players/05 - A/{rel}")),
                "{rel}"
            );
        }
        assert!(
            !plan
                .copies
                .iter()
                .any(|(_, to)| to.contains("Crocs/") || to.contains("Mitts/"))
        );
    }

    #[test]
    fn the_pre_fox_engine_never_merges() {
        // Even one wearer stays shared on pre-Fox, with the link file.
        let files = files(&[
            "Faces/XXX05 - A/face.fmdl",
            "Boots/k0430 - Crocs/oral_boots_win32.model",
        ]);
        let roster = wearing(&[(5, ("A", 430, 0))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::PreFox, &[]);
        assert!(
            plan.copies
                .iter()
                .any(|(_, to)| to == "Boots/Crocs/oral_boots_win32.model")
        );
        assert!(
            plan.markers
                .contains(&"Players/05 - A/Crocs.boots".to_owned())
        );
        assert!(
            !plan
                .copies
                .iter()
                .any(|(_, to)| to.starts_with("Players/05 - A/oral"))
        );
    }

    #[test]
    fn a_byte_clash_keeps_the_item_folder_shared() {
        // Equal bytes merge silently; different bytes keep the folder shared
        // with `merge_kept_shared` naming it and its first clashing path.
        let files = files(&[
            "Faces/XXX05 - A/skin.dds",
            "Boots/k0430 - Crocs/skin.dds",
            "Boots/k0430 - Crocs/boots.fmdl",
        ]);
        let roster = wearing(&[(5, ("A", 430, 0))]);
        let equal = plan_with_bytes(
            &files,
            Some(&roster),
            Engine::Fox,
            &[
                ("Faces/XXX05 - A/skin.dds", b"same"),
                ("Boots/k0430 - Crocs/skin.dds", b"same"),
            ],
        );
        assert!(
            equal
                .copies
                .iter()
                .any(|(_, to)| to == "Players/05 - A/boots.fmdl")
        );
        assert_eq!(
            equal
                .copies
                .iter()
                .filter(|(_, to)| to == "Players/05 - A/skin.dds")
                .count(),
            1,
            "the equal path is kept once"
        );
        let differing = plan_with_bytes(
            &files,
            Some(&roster),
            Engine::Fox,
            &[
                ("Faces/XXX05 - A/skin.dds", b"mine"),
                ("Boots/k0430 - Crocs/skin.dds", b"theirs"),
            ],
        );
        let kept = differing
            .findings
            .iter()
            .find(|m| m.code.code == "merge_kept_shared")
            .expect("merge_kept_shared expected");
        assert_eq!(kept.severity, Severity::Info);
        assert_eq!(
            kept.context,
            vec![("path".to_owned(), "skin.dds".to_owned())]
        );
        assert_eq!(
            kept.scope,
            Scope::Folder {
                export_id: INPUT,
                path: ScopePath::new("Boots/k0430 - Crocs").unwrap()
            }
        );
        assert!(
            differing
                .copies
                .iter()
                .any(|(_, to)| to == "Boots/Crocs/skin.dds")
        );
        assert!(
            differing
                .markers
                .contains(&"Players/05 - A/Crocs.boots".to_owned())
        );
    }

    #[test]
    fn a_folder_nobody_wears_is_leftovers_and_a_second_folder_of_one_id_is_refused() {
        // EU-PLY-06's shape: no rostered wearer → the folder's files are
        // leftovers; a second folder of the same ID follows the first by
        // name and stays unmigrated.
        let files = files(&[
            "Boots/k0430 - A/boots.fmdl",
            "Boots/k0430 - B/boots.fmdl",
            "Gloves/g0432 - Mitts/glove_l.fmdl",
        ]);
        let roster = wearing(&[(5, ("A", 430, 0))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        let folders: Vec<String> = plan
            .findings
            .iter()
            .filter(|m| m.code.code == "unmigrated_content")
            .filter_map(|m| match &m.scope {
                Scope::Folder { path, .. } => Some(path.as_str().to_owned()),
                _ => None,
            })
            .collect();
        assert_eq!(folders, ["Boots/k0430 - B".to_owned()]);
        let second = plan
            .findings
            .iter()
            .find(|m| m.code.code == "unmigrated_content")
            .expect("the second folder of the ID is reported");
        assert_eq!(
            second.context,
            [
                ("reason".to_owned(), "id_taken".to_owned()),
                ("other".to_owned(), "Boots/k0430 - A".to_owned()),
            ]
        );
        assert!(
            plan.leftovers.contains("Gloves/g0432 - Mitts/glove_l.fmdl"),
            "{:?}",
            plan.leftovers
        );
        assert!(!plan.consumed.contains("Gloves/g0432 - Mitts/glove_l.fmdl"));
        assert!(
            !plan
                .copies
                .iter()
                .any(|(_, to)| to.contains("g0432") || to.contains("Mitts/"))
        );
    }

    #[test]
    fn a_file_directly_in_faces_belongs_to_no_plan() {
        // A referee export's folders are `referees.rs`'s: `run` does not
        // call `player_folders` for one.
        let files = files(&["Faces/loose.fmdl", "Faces/XXX15 - A/face.fmdl"]);
        let plan = plan_folders(&files);
        assert!(!plan.consumed.contains("Faces/loose.fmdl"));
    }

    #[test]
    fn a_case_folded_clash_keeps_the_item_folder_shared() {
        // `Skin.dds` in the face folder and `skin.dds` in the boots folder:
        // paths compare folded, so of different bytes they clash.
        let files = files(&[
            "Faces/XXX05 - A/Skin.dds",
            "Boots/k0430 - Crocs/skin.dds",
            "Boots/k0430 - Crocs/boots.fmdl",
        ]);
        let roster = wearing(&[(5, ("A", 430, 0))]);
        let differing = plan_with_bytes(
            &files,
            Some(&roster),
            Engine::Fox,
            &[
                ("Faces/XXX05 - A/Skin.dds", b"face"),
                ("Boots/k0430 - Crocs/skin.dds", b"boots"),
            ],
        );
        let kept = differing
            .findings
            .iter()
            .find(|m| m.code.code == "merge_kept_shared")
            .expect("merge_kept_shared expected");
        // The item folder's spelling of the clashing path.
        assert_eq!(
            kept.context,
            vec![("path".to_owned(), "skin.dds".to_owned())]
        );
        assert!(
            differing
                .copies
                .iter()
                .any(|(_, to)| to == "Boots/Crocs/skin.dds")
        );
        // Equal bytes merge once, the player folder's own spelling kept.
        let equal = plan_with_bytes(
            &files,
            Some(&roster),
            Engine::Fox,
            &[
                ("Faces/XXX05 - A/Skin.dds", b"same"),
                ("Boots/k0430 - Crocs/skin.dds", b"same"),
            ],
        );
        assert!(
            equal
                .copies
                .iter()
                .any(|(_, to)| to == "Players/05 - A/Skin.dds")
        );
        assert!(
            equal
                .copies
                .iter()
                .any(|(_, to)| to == "Players/05 - A/boots.fmdl")
        );
        assert!(
            !equal
                .copies
                .iter()
                .any(|(_, to)| to == "Players/05 - A/skin.dds")
        );
    }

    #[test]
    fn two_shared_folders_matching_only_by_fold_resolve_by_lowest_id() {
        // `Éclair` and `éclair` fold to one name where `to_ascii_lowercase`
        // kept them apart.
        let files = files(&[
            "Boots/k0430 - Éclair/boots.fmdl",
            "Boots/k0431 - éclair/boots.fmdl",
        ]);
        let roster = wearing(&[
            (1, ("A", 430, 0)),
            (2, ("B", 430, 0)),
            (3, ("C", 431, 0)),
            (4, ("D", 431, 0)),
        ]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        assert!(
            plan.copies
                .iter()
                .any(|(_, to)| to == "Boots/Éclair/boots.fmdl")
        );
        assert!(
            plan.copies
                .iter()
                .any(|(_, to)| to == "Boots/k0431/boots.fmdl")
        );
    }

    #[test]
    fn identical_shared_folders_merge_into_the_lowest_ids() {
        // `k0430 - Crocs` and `k0431 - Crocs`, the same files byte for byte:
        // one `Boots/Crocs/`, every wearer of either linking it.
        let files = files(&[
            "Boots/k0430 - Crocs/boots.fmdl",
            "Boots/k0430 - Crocs/boots.skl",
            "Boots/k0431 - Crocs/boots.fmdl",
            "Boots/k0431 - Crocs/boots.skl",
        ]);
        let roster = wearing(&[
            (1, ("A", 430, 0)),
            (2, ("B", 430, 0)),
            (3, ("C", 431, 0)),
            (4, ("D", 431, 0)),
        ]);
        let plan = plan_with_bytes(
            &files,
            Some(&roster),
            Engine::Fox,
            &[
                ("Boots/k0430 - Crocs/boots.fmdl", b"model"),
                ("Boots/k0431 - Crocs/boots.fmdl", b"model"),
                ("Boots/k0430 - Crocs/boots.skl", b"skel"),
                ("Boots/k0431 - Crocs/boots.skl", b"skel"),
            ],
        );
        // Two files, from `k0430 - Crocs` alone.
        assert_eq!(
            plan.copies,
            [
                (
                    "Boots/k0430 - Crocs/boots.fmdl".to_owned(),
                    "Boots/Crocs/boots.fmdl".to_owned()
                ),
                (
                    "Boots/k0430 - Crocs/boots.skl".to_owned(),
                    "Boots/Crocs/boots.skl".to_owned()
                ),
            ]
        );
        for player in ["01 - Crocs", "02 - Crocs", "03 - Crocs", "04 - Crocs"] {
            assert!(
                plan.markers
                    .contains(&format!("Players/{player}/Crocs.boots")),
                "{player}"
            );
        }
        // Its files are not copied but stay consumed — nothing is lost and
        // nothing is reported.
        for file in ["boots.fmdl", "boots.skl"] {
            assert!(
                plan.consumed
                    .contains(&format!("Boots/k0431 - Crocs/{file}")),
                "{file}"
            );
        }
        assert!(
            plan.findings
                .iter()
                .all(|m| m.code.code.as_ref() != "unmigrated_content")
        );
    }

    #[test]
    fn a_difference_in_one_file_keeps_the_id_spelling() {
        // `boots.skl` differing in one byte: two shared folders of one name
        // keep it for the lowest ID, and the other takes its ID, `Boots/k0431/`.
        let files = files(&[
            "Boots/k0430 - Crocs/boots.fmdl",
            "Boots/k0430 - Crocs/boots.skl",
            "Boots/k0431 - Crocs/boots.fmdl",
            "Boots/k0431 - Crocs/boots.skl",
        ]);
        let roster = wearing(&[
            (1, ("A", 430, 0)),
            (2, ("B", 430, 0)),
            (3, ("C", 431, 0)),
            (4, ("D", 431, 0)),
        ]);
        let plan = plan_with_bytes(
            &files,
            Some(&roster),
            Engine::Fox,
            &[
                ("Boots/k0430 - Crocs/boots.fmdl", b"model"),
                ("Boots/k0431 - Crocs/boots.fmdl", b"model"),
                ("Boots/k0430 - Crocs/boots.skl", b"one"),
                ("Boots/k0431 - Crocs/boots.skl", b"two"),
            ],
        );
        assert!(
            plan.copies
                .iter()
                .any(|(_, to)| to == "Boots/k0431/boots.skl")
        );
        assert!(
            plan.markers
                .contains(&"Players/03 - Crocs/k0431.boots".to_owned())
        );
        assert!(
            plan.markers
                .contains(&"Players/04 - Crocs/k0431.boots".to_owned())
        );
    }

    #[test]
    fn an_extra_file_means_no_merge() {
        // `k0431 - Crocs` holding one more file than `k0430 - Crocs`.
        let files = files(&[
            "Boots/k0430 - Crocs/boots.fmdl",
            "Boots/k0430 - Crocs/boots.skl",
            "Boots/k0431 - Crocs/boots.fmdl",
            "Boots/k0431 - Crocs/boots.skl",
            "Boots/k0431 - Crocs/extra.dds",
        ]);
        let roster = wearing(&[
            (1, ("A", 430, 0)),
            (2, ("B", 430, 0)),
            (3, ("C", 431, 0)),
            (4, ("D", 431, 0)),
        ]);
        let plan = plan_with_bytes(
            &files,
            Some(&roster),
            Engine::Fox,
            &[
                ("Boots/k0430 - Crocs/boots.fmdl", b"model"),
                ("Boots/k0431 - Crocs/boots.fmdl", b"model"),
                ("Boots/k0430 - Crocs/boots.skl", b"skel"),
                ("Boots/k0431 - Crocs/boots.skl", b"skel"),
                ("Boots/k0431 - Crocs/extra.dds", b"extra"),
            ],
        );
        assert!(
            plan.copies
                .iter()
                .any(|(_, to)| to == "Boots/k0431/extra.dds")
        );
        assert!(
            plan.markers
                .contains(&"Players/03 - Crocs/k0431.boots".to_owned())
        );
    }

    #[test]
    fn a_case_folded_file_set_merges_too() {
        // `Boots.fmdl` in `k0431 - crocs` is `boots.fmdl` in `k0430 - Crocs`,
        // paths folded: equal bytes merge them.
        let files = files(&[
            "Boots/k0430 - Crocs/boots.fmdl",
            "Boots/k0431 - crocs/Boots.fmdl",
        ]);
        let roster = wearing(&[
            (1, ("A", 430, 0)),
            (2, ("B", 430, 0)),
            (3, ("C", 431, 0)),
            (4, ("D", 431, 0)),
        ]);
        let plan = plan_with_bytes(
            &files,
            Some(&roster),
            Engine::Fox,
            &[
                ("Boots/k0430 - Crocs/boots.fmdl", b"model"),
                ("Boots/k0431 - crocs/Boots.fmdl", b"model"),
            ],
        );
        assert_eq!(
            plan.copies,
            [(
                "Boots/k0430 - Crocs/boots.fmdl".to_owned(),
                "Boots/Crocs/boots.fmdl".to_owned()
            )]
        );
        assert!(
            plan.markers
                .contains(&"Players/03 - crocs/Crocs.boots".to_owned())
        );
    }

    #[test]
    fn among_three_folders_only_the_identical_pair_merges() {
        // `k0431 - Crocs` is `k0430 - Crocs` byte for byte, `k0432 - Crocs`
        // is not: the middle merges, the third keeps its ID spelling.
        let files = files(&[
            "Boots/k0430 - Crocs/boots.fmdl",
            "Boots/k0431 - Crocs/boots.fmdl",
            "Boots/k0432 - Crocs/boots.fmdl",
        ]);
        let roster = wearing(&[
            (1, ("A", 430, 0)),
            (2, ("B", 430, 0)),
            (3, ("C", 431, 0)),
            (4, ("D", 431, 0)),
            (5, ("E", 432, 0)),
            (6, ("F", 432, 0)),
        ]);
        let plan = plan_with_bytes(
            &files,
            Some(&roster),
            Engine::Fox,
            &[
                ("Boots/k0430 - Crocs/boots.fmdl", b"model"),
                ("Boots/k0431 - Crocs/boots.fmdl", b"model"),
                ("Boots/k0432 - Crocs/boots.fmdl", b"other"),
            ],
        );
        assert_eq!(
            plan.copies,
            [
                (
                    "Boots/k0430 - Crocs/boots.fmdl".to_owned(),
                    "Boots/Crocs/boots.fmdl".to_owned()
                ),
                (
                    "Boots/k0432 - Crocs/boots.fmdl".to_owned(),
                    "Boots/k0432/boots.fmdl".to_owned()
                ),
            ]
        );
        for marker in [
            "Players/01 - Crocs/Crocs.boots",
            "Players/03 - Crocs/Crocs.boots",
            "Players/05 - Crocs/k0432.boots",
            "Players/06 - Crocs/k0432.boots",
        ] {
            assert!(plan.markers.contains(&marker.to_owned()), "{marker}");
        }
    }

    #[test]
    fn loose_mode_pairs_each_id_by_its_position_in_the_block() {
        // Step 11: the lowest boots ID 0128 infers the 126 block, so 0128 is
        // slot 03 and 0130 slot 05; 0149 and 0155 (positions 24 and 30) name
        // nobody and are unmigrated as whole folders.
        let files = files(&[
            "Faces/XXX03 - Ann/face.fmdl",
            "Boots/k0128 - Rex/boots.fmdl",
            "Boots/k0130/boots.fmdl",
            "Boots/k0149/boots.fmdl",
            "Boots/k0155/boots.fmdl",
        ]);
        let plan = plan_with_bytes(&files, None, Engine::Fox, &[]);
        for (from, to) in [
            (
                "Boots/k0128 - Rex/boots.fmdl",
                "Players/03 - Ann/boots.fmdl",
            ),
            ("Boots/k0130/boots.fmdl", "Players/05/boots.fmdl"),
        ] {
            assert!(
                plan.copies.contains(&(from.to_owned(), to.to_owned())),
                "{from} → {to}: {:?}",
                plan.copies
            );
        }
        for refused in ["k0149", "k0155"] {
            assert!(
                !plan.copies.iter().any(|(from, _)| from.contains(refused)),
                "{refused}"
            );
        }
        // Slot 05 has no face folder: the bare marker; 03 - Ann's own face
        // means none.
        assert!(
            plan.markers
                .contains(&"Players/05/ingame_face.toml".to_owned())
        );
        assert!(
            !plan
                .markers
                .contains(&"Players/03 - Ann/ingame_face.toml".to_owned())
        );
        let unmigrated: Vec<&Message> = plan
            .findings
            .iter()
            .filter(|m| m.code.code == "unmigrated_content")
            .collect();
        assert_eq!(unmigrated.len(), 2, "{:?}", plan.findings);
        for (finding, folder) in unmigrated.iter().zip(["Boots/k0149", "Boots/k0155"]) {
            assert_eq!(finding.disposition, Disposition::DropFolder);
            assert_eq!(
                finding.scope,
                Scope::Folder {
                    export_id: INPUT,
                    path: ScopePath::new(folder).unwrap()
                }
            );
            assert_eq!(
                finding.context,
                [("reason".to_owned(), "loose_unplaceable".to_owned())]
            );
        }
        // Each assumed pairing is reported on its item folder.
        let assumed: Vec<&Message> = plan
            .findings
            .iter()
            .filter(|m| m.code.code == "wearer_assumed")
            .collect();
        assert_eq!(assumed.len(), 2, "{:?}", plan.findings);
        for (finding, (folder, context)) in assumed.iter().zip([
            (
                "Boots/k0128 - Rex",
                vec![
                    ("id".to_owned(), "0128".to_owned()),
                    ("slot".to_owned(), "03".to_owned()),
                ],
            ),
            (
                "Boots/k0130",
                vec![
                    ("id".to_owned(), "0130".to_owned()),
                    ("slot".to_owned(), "05".to_owned()),
                ],
            ),
        ]) {
            assert_eq!(finding.severity, Severity::Info);
            assert_eq!(finding.disposition, Disposition::Keep);
            assert_eq!(
                finding.scope,
                Scope::Folder {
                    export_id: INPUT,
                    path: ScopePath::new(folder).unwrap()
                }
            );
            assert_eq!(finding.context, context);
        }
        assert!(plan.folders[&3].named);
        assert!(!plan.folders[&5].named);
    }

    #[test]
    fn loose_mode_infers_each_kinds_block_from_its_own_lowest_id() {
        // Each kind's block comes from its own lowest ID: g0160's minimum
        // infers the 151 block, so it is player 10; sharing the boots' 126
        // block would put it past the block.
        let files = files(&["Boots/k0128/boots.fmdl", "Gloves/g0160/glove_l.fmdl"]);
        let plan = plan_with_bytes(&files, None, Engine::Fox, &[]);
        assert!(
            plan.copies.contains(&(
                "Boots/k0128/boots.fmdl".to_owned(),
                "Players/03/boots.fmdl".to_owned()
            )),
            "{:?}",
            plan.copies
        );
        assert!(
            plan.copies.contains(&(
                "Gloves/g0160/glove_l.fmdl".to_owned(),
                "Players/10/glove_l.fmdl".to_owned()
            )),
            "{:?}",
            plan.copies
        );
    }

    #[test]
    fn loose_mode_block_bounds() {
        // The lowest ID 0101 infers the 101 block: positions 1 to 23 pair,
        // 24 and 25 are the block's spare positions, and 0126 falls past it.
        let files = files(&[
            "Boots/k0101/boots.fmdl",
            "Boots/k0123/boots.fmdl",
            "Boots/k0124/boots.fmdl",
            "Boots/k0125/boots.fmdl",
            "Boots/k0126/boots.fmdl",
        ]);
        let plan = plan_with_bytes(&files, None, Engine::Fox, &[]);
        for (from, to) in [
            ("Boots/k0101/boots.fmdl", "Players/01/boots.fmdl"),
            ("Boots/k0123/boots.fmdl", "Players/23/boots.fmdl"),
        ] {
            assert!(
                plan.copies.contains(&(from.to_owned(), to.to_owned())),
                "{from} → {to}: {:?}",
                plan.copies
            );
        }
        let unmigrated: Vec<&Message> = plan
            .findings
            .iter()
            .filter(|m| m.code.code == "unmigrated_content")
            .collect();
        let folders: Vec<String> = unmigrated
            .iter()
            .filter_map(|m| match &m.scope {
                Scope::Folder { path, .. } => Some(path.as_str().to_owned()),
                _ => None,
            })
            .collect();
        assert_eq!(
            folders,
            [
                "Boots/k0124".to_owned(),
                "Boots/k0125".to_owned(),
                "Boots/k0126".to_owned()
            ]
        );
        for finding in unmigrated {
            assert_eq!(
                finding.context,
                [("reason".to_owned(), "loose_unplaceable".to_owned())]
            );
        }
    }

    #[test]
    fn loose_mode_keeps_a_pre_fox_folder_shared_with_a_link() {
        // Pre-Fox never merges: the one assumed wearer links the shared
        // folder, named by its name part.
        let files = files(&[
            "Faces/XXX03 - Ann/face.xml",
            "Boots/k0128 - Rex/boots.model",
        ]);
        let plan = plan_with_bytes(&files, None, Engine::PreFox, &[]);
        assert!(
            plan.copies.contains(&(
                "Boots/k0128 - Rex/boots.model".to_owned(),
                "Boots/Rex/boots.model".to_owned()
            )),
            "{:?}",
            plan.copies
        );
        assert!(
            plan.markers
                .contains(&"Players/03 - Ann/Rex.boots".to_owned())
        );
    }

    #[test]
    fn the_stray_file_rule() {
        let fox_boots = Some((ItemKind::Boots, Engine::Fox));
        let fox_gloves = Some((ItemKind::Gloves, Engine::Fox));
        // A file `classify` reads as no known kind and that is no `.txt`
        // note is a stray wherever it sits.
        for name in ["face.mtl.backup", "materials.mtl.bak", "junk.doc"] {
            assert!(is_stray(name, None), "{name}");
            assert!(is_stray(name, fox_boots), "{name}");
        }
        // A `.txt` note is no stray; neither is a file of a known kind.
        for name in ["notes.txt", "face.mtl", "skin.dds"] {
            assert!(!is_stray(name, None), "{name}");
            assert!(!is_stray(name, fox_boots), "{name}");
        }
        // In a Fox boots or gloves folder an `.fmdl` under no name the
        // game loads is a stray; the names alone and after a prefix and
        // `_` are the folder's own.
        assert!(is_stray("boots - Copy.fmdl", fox_boots));
        assert!(is_stray("glove_l - Copy.fmdl", fox_gloves));
        for (name, item) in [
            ("boots.fmdl", fox_boots),
            ("kit_boots.fmdl", fox_boots),
            ("Boots.FMDL", fox_boots),
            ("glove_l.fmdl", fox_gloves),
            ("team_glove_r.fmdl", fox_gloves),
        ] {
            assert!(!is_stray(name, item), "{name}");
        }
        // The name rule is Fox-only: pre-Fox keeps a copy-named `.model`,
        // as today.
        assert!(!is_stray(
            "boots - Copy.model",
            Some((ItemKind::Boots, Engine::PreFox))
        ));
    }

    #[test]
    fn a_stray_in_a_face_or_item_folder_is_a_leftover() {
        let files = files(&[
            "Faces/XXX05 - A/face.mtl",
            "Faces/XXX05 - A/face.mtl.backup",
            "Boots/k0430 - Crocs/boots.fmdl",
            "Boots/k0430 - Crocs/boots - Copy.fmdl",
        ]);
        let roster = wearing(&[(5, ("A", 430, 0))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        for stray in [
            "Faces/XXX05 - A/face.mtl.backup",
            "Boots/k0430 - Crocs/boots - Copy.fmdl",
        ] {
            assert!(plan.leftovers.contains(stray), "{stray}");
            assert!(!plan.consumed.contains(stray), "{stray}");
        }
        for kept in ["face.mtl", "boots.fmdl"] {
            assert!(
                plan.copies
                    .iter()
                    .any(|(_, to)| to == &format!("Players/05 - A/{kept}")),
                "{kept}"
            );
        }
    }

    #[test]
    fn an_empty_face_folder_forms_with_no_marker_and_no_name_change() {
        // Listed as a folder but holding no file: `has_face` is set (the
        // wearer keeps his own face), no `ingame_face`, the folder's name
        // part still names him.
        let plan = player_folders(
            &files(&["Boots/k0430 - Crocs/boots.fmdl"]),
            &files(&["Faces", "Faces/XXX05 - Bob", "Boots", "Boots/k0430 - Crocs"]),
            Some(&wearing(&[(5, ("SaveName", 430, 0))])),
            Engine::Fox,
            false,
            |_, _| false,
        );
        assert!(plan.folders.contains_key(&5), "{:?}", plan.folders.keys());
        assert_eq!(plan.folders[&5].path, "Players/05 - Bob");
        assert!(
            !plan
                .markers
                .contains(&"Players/05 - Bob/ingame_face.toml".to_owned())
        );
        // The merged boots still land in it.
        assert!(
            plan.copies
                .iter()
                .any(|(_, to)| to == "Players/05 - Bob/boots.fmdl")
        );
    }

    #[test]
    fn a_modelless_item_folder_marks_its_wearers_fpc_on() {
        // A folder holding no model (an empty one included) hides the
        // player's boots or hands: `fpc_on`, the category claimed, its
        // files leftovers, its name part naming nobody.
        let folders = files(&[
            "Boots/k0430 - EMPTY",
            "Gloves/g0432 - EMPTY",
            "Faces/XXX03 - Ann",
        ]);
        let plan = player_folders(
            &files(&["Boots/k0430 - EMPTY/boots.skl"]),
            &folders,
            Some(&wearing(&[
                (3, ("Save", 430, 432)),
                (4, ("NoFace", 430, 0)),
            ])),
            Engine::Fox,
            false,
            |_, _| false,
        );
        // One `fpc_on` each; the empty face folder leaves 03 no
        // `ingame_face`, while 04, no face folder, gets it.
        for (folder, markers) in [
            ("Players/03 - Ann", vec!["fpc_on"]),
            ("Players/04 - NoFace", vec!["fpc_on", "ingame_face.toml"]),
        ] {
            for marker in ["fpc_on", "ingame_face.toml"] {
                let expected = markers.contains(&marker);
                assert_eq!(
                    plan.markers.contains(&format!("{folder}/{marker}")),
                    expected,
                    "{folder}/{marker}"
                );
            }
        }
        assert!(plan.leftovers.contains("Boots/k0430 - EMPTY/boots.skl"));
        assert_eq!(plan.folders[&3].boots_folder, Some(ItemFolder::Modelless));
        assert_eq!(plan.folders[&3].gloves_folder, Some(ItemFolder::Modelless));
        assert_eq!(plan.folders[&4].boots_folder, Some(ItemFolder::Modelless));
        // `EMPTY` names nobody: 04's name fell to the save's.
        assert_eq!(plan.folders[&4].path, "Players/04 - NoFace");
    }

    #[test]
    fn a_game_path_face_folder_moves_its_innermost_folders_files() {
        let files = files(&[
            "Faces/71401/common/character0/model/character/face/real/71401/face.xml",
            "Faces/71401/common/character0/model/character/face/real/71401/face.mtl",
        ]);
        let plan = plan_folders(&files);
        for name in ["face.xml", "face.mtl"] {
            assert!(
                plan.copies
                    .iter()
                    .any(|(_, to)| to == &format!("Players/01/{name}")),
                "{name}: {:?}",
                plan.copies
            );
        }
    }

    #[test]
    fn a_game_path_in_another_letter_case_is_the_game_path() {
        // Windows paths ignore case, so an extraction may spell the
        // segments either way: `Common/Character0/...` is the same path.
        let files = files(&[
            "Faces/71401/Common/Character0/Model/character/Face/REAL/71401/face.xml",
            "Faces/71401/common/character0/model/Character/face/real/71401/face.mtl",
        ]);
        let plan = plan_folders(&files);
        for name in ["face.xml", "face.mtl"] {
            assert!(
                plan.copies
                    .iter()
                    .any(|(_, to)| to == &format!("Players/01/{name}")),
                "{name}: {:?}",
                plan.copies
            );
        }
    }

    #[test]
    fn a_face_folder_with_one_file_outside_the_game_path_keeps_its_paths() {
        // One file not below `real/<one folder>/` is no extracted game
        // path: the folder's files keep their places.
        let files = files(&[
            "Faces/71401/common/character0/model/character/face/real/71401/face.xml",
            "Faces/71401/face.mtl",
        ]);
        let plan = plan_folders(&files);
        for (from, to) in [
            (
                "Faces/71401/common/character0/model/character/face/real/71401/face.xml",
                "Players/01/common/character0/model/character/face/real/71401/face.xml",
            ),
            ("Faces/71401/face.mtl", "Players/01/face.mtl"),
        ] {
            assert!(
                plan.copies.contains(&(from.to_owned(), to.to_owned())),
                "{to}: {:?}",
                plan.copies
            );
        }
    }

    #[test]
    fn two_innermost_game_path_folders_keep_their_paths() {
        // Two different folders below `real/` are no game's own path to a
        // face: `game_path` asks every file for the same innermost folder,
        // so the files keep the paths they sit at.
        let files = files(&[
            "Faces/71401/common/character0/model/character/face/real/71401/face.xml",
            "Faces/71401/common/character0/model/character/face/real/71402/face.mtl",
        ]);
        let plan = plan_folders(&files);
        for (from, to) in [
            (
                "Faces/71401/common/character0/model/character/face/real/71401/face.xml",
                "Players/01/common/character0/model/character/face/real/71401/face.xml",
            ),
            (
                "Faces/71401/common/character0/model/character/face/real/71402/face.mtl",
                "Players/01/common/character0/model/character/face/real/71402/face.mtl",
            ),
        ] {
            assert!(
                plan.copies.contains(&(from.to_owned(), to.to_owned())),
                "{to}: {:?}",
                plan.copies
            );
        }
    }

    #[test]
    fn the_wearer_name_fold_keeps_letters_and_digits_lowercased() {
        for (name, expected) in [
            ("BANISHED", "banished"),
            ("81514- LET ME DEFEND", "81514letmedefend"),
            ("k2952 - BANISHED", "k2952banished"),
            ("XXX05 - Bob", "xxx05bob"),
            ("Änn-É", "änné"),
            ("   ", ""),
            ("!?.", ""),
        ] {
            assert_eq!(fold_wearer_name(name), expected, "{name:?}");
        }
    }

    #[test]
    fn an_unworn_folder_named_for_a_player_pairs_his_unclaimed_id() {
        // A rostered player's worn custom ID (above 100) no folder of the
        // kind carries: the one unworn folder named after him is his, the
        // warning naming the folder, the slot and the save's ID.
        let files = files(&[
            "Faces/XXX05 - BANISHED/face.fmdl",
            "Boots/k2952 - BANISHED/boots.fmdl",
        ]);
        let roster = wearing(&[(5, ("SaveName", 2552, 0))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        assert!(
            plan.copies
                .iter()
                .any(|(_, to)| to == "Players/05 - BANISHED/boots.fmdl"),
            "{:?}",
            plan.copies
        );
        assert!(!plan.leftovers.contains("Boots/k2952 - BANISHED/boots.fmdl"));
        let finding = plan
            .findings
            .iter()
            .find(|m| m.code.code == "wearer_by_name")
            .expect("wearer_by_name expected");
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.disposition, Disposition::Keep);
        assert_eq!(
            finding.scope,
            Scope::Folder {
                export_id: INPUT,
                path: ScopePath::new("Boots/k2952 - BANISHED").unwrap()
            }
        );
        assert_eq!(
            finding.context,
            [
                ("slot".to_owned(), "05".to_owned()),
                ("id".to_owned(), "2552".to_owned()),
            ]
        );
    }

    #[test]
    fn the_name_rule_reads_all_three_wearer_names() {
        // One missing boots ID beside two unworn boots folders: the counts
        // differ, so the order rule cannot pair and only the name rule can —
        // each case through a different one of its names (the face folder's
        // whole name, its name part, the save name).
        for (paths, roster, folder, slot, id, leftover) in [
            (
                vec![
                    "Faces/81514- LET ME DEFEND/face.fmdl",
                    "Boots/k2964 - 81514- LET ME DEFEND/boots.fmdl",
                    "Boots/k2965 - Other/boots.fmdl",
                ],
                wearing(&[(14, ("SaveA", 2564, 0))]),
                "Boots/k2964 - 81514- LET ME DEFEND",
                "14",
                "2564",
                "Boots/k2965 - Other/boots.fmdl",
            ),
            (
                vec![
                    "Faces/XXX05 - Ann/face.fmdl",
                    "Boots/k2999 - Ann/boots.fmdl",
                    "Boots/k2998 - Other/boots.fmdl",
                ],
                wearing(&[(5, ("SaveA", 2500, 0))]),
                "Boots/k2999 - Ann",
                "05",
                "2500",
                "Boots/k2998 - Other/boots.fmdl",
            ),
            (
                vec![
                    "Boots/k2999 - Bob/boots.fmdl",
                    "Boots/k2998 - Other/boots.fmdl",
                ],
                wearing(&[(5, ("Bob", 2500, 0))]),
                "Boots/k2999 - Bob",
                "05",
                "2500",
                "Boots/k2998 - Other/boots.fmdl",
            ),
        ] {
            let plan = plan_with_bytes(&files(&paths), Some(&roster), Engine::Fox, &[]);
            assert_eq!(
                finding_at(&plan, "wearer_by_name", folder).context,
                [
                    ("slot".to_owned(), slot.to_owned()),
                    ("id".to_owned(), id.to_owned()),
                ],
                "{folder}"
            );
            assert!(
                plan.leftovers.contains(leftover),
                "{leftover}: {:?}",
                plan.leftovers
            );
            assert!(
                plan.findings
                    .iter()
                    .all(|m| m.code.code != "custom_model_unpaired"),
                "{folder}: {:?}",
                plan.findings
            );
        }
    }

    #[test]
    fn two_unworn_folders_of_one_name_refuse_the_missing_id() {
        // Two candidates for one player: nothing pairs by name, and the
        // missing ID left beside unworn folders is `custom_model_unpaired`.
        let files = files(&[
            "Faces/XXX05 - BANISHED/face.fmdl",
            "Boots/k2952 - BANISHED/boots.fmdl",
            "Boots/k2953 - banished/boots.fmdl",
        ]);
        let roster = wearing(&[(5, ("SaveName", 2552, 0))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        for folder in ["k2952 - BANISHED", "k2953 - banished"] {
            assert!(
                plan.leftovers
                    .contains(&format!("Boots/{folder}/boots.fmdl")),
                "{folder}"
            );
        }
        assert!(
            plan.findings
                .iter()
                .all(|m| m.code.code != "wearer_by_name")
        );
        let refused = plan
            .findings
            .iter()
            .find(|m| m.code.code == "custom_model_unpaired")
            .expect("custom_model_unpaired expected");
        assert_eq!(refused.severity, Severity::Error);
        assert_eq!(
            refused.context,
            [
                ("slot".to_owned(), "05".to_owned()),
                ("key".to_owned(), "boots_id".to_owned()),
                ("id".to_owned(), "2552".to_owned()),
            ]
        );
        assert!(
            !plan.copies.iter().any(|(_, to)| to.ends_with("boots.fmdl")),
            "{:?}",
            plan.copies
        );
    }

    #[test]
    fn the_number_rule_runs_across_kinds_in_either_direction() {
        // A folder paired in one kind lets an unworn same-numbered folder
        // of the other kind take its ID, whether the first pairing was
        // boots or gloves: `k2701` follows `g2701` (2700 missing in boots)
        // and `g2802` follows `k2802` (2800 missing in gloves).
        let files = files(&[
            "Faces/XXX05 - Alpha/face.fmdl",
            "Faces/XXX06 - Beta/face.fmdl",
            "Gloves/g2701 - Alpha/glove_l.fmdl",
            "Boots/k2701 - Other/boots.fmdl",
            "Boots/k2802 - Beta/boots.fmdl",
            "Gloves/g2802 - Other/glove_l.fmdl",
        ]);
        let roster = wearing(&[(5, ("SaveA", 2700, 2700)), (6, ("SaveB", 2800, 2800))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        for (folder, code, context) in [
            (
                "Gloves/g2701 - Alpha",
                "wearer_by_name",
                vec![
                    ("slot".to_owned(), "05".to_owned()),
                    ("id".to_owned(), "2700".to_owned()),
                ],
            ),
            (
                "Boots/k2802 - Beta",
                "wearer_by_name",
                vec![
                    ("slot".to_owned(), "06".to_owned()),
                    ("id".to_owned(), "2800".to_owned()),
                ],
            ),
            (
                "Boots/k2701 - Other",
                "wearer_by_number",
                vec![
                    ("slot".to_owned(), "05".to_owned()),
                    ("id".to_owned(), "2700".to_owned()),
                    ("from".to_owned(), "Gloves/g2701 - Alpha".to_owned()),
                ],
            ),
            (
                "Gloves/g2802 - Other",
                "wearer_by_number",
                vec![
                    ("slot".to_owned(), "06".to_owned()),
                    ("id".to_owned(), "2800".to_owned()),
                    ("from".to_owned(), "Boots/k2802 - Beta".to_owned()),
                ],
            ),
        ] {
            assert_eq!(finding_at(&plan, code, folder).context, context, "{folder}");
        }
        for (folder, names) in [
            ("Players/05 - Alpha", vec!["glove_l.fmdl", "boots.fmdl"]),
            ("Players/06 - Beta", vec!["boots.fmdl", "glove_l.fmdl"]),
        ] {
            for name in names {
                assert!(
                    plan.copies
                        .iter()
                        .any(|(_, to)| to == &format!("{folder}/{name}")),
                    "{folder}/{name}: {:?}",
                    plan.copies
                );
            }
        }
    }

    #[test]
    fn the_order_rule_waits_for_the_number_rule() {
        // `k2999 - Ann` pairs by name; `g2999` follows it by number while
        // its kind still holds two missing IDs and two unworn folders —
        // the order rule must not fire first, or `g2999` would take 2601
        // by rank instead of 2500 by `from`.
        let files = files(&[
            "Faces/XXX05 - Ann/face.fmdl",
            "Faces/XXX06 - Bob/face.fmdl",
            "Boots/k2999 - Ann/boots.fmdl",
            "Gloves/g2999 - Other/glove_l.fmdl",
            "Gloves/g2998 - Other/glove_l.fmdl",
        ]);
        let roster = wearing(&[(5, ("SaveA", 2500, 2500)), (6, ("SaveB", 0, 2601))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        assert_eq!(
            finding_at(&plan, "wearer_by_number", "Gloves/g2999 - Other").context,
            [
                ("slot".to_owned(), "05".to_owned()),
                ("id".to_owned(), "2500".to_owned()),
                ("from".to_owned(), "Boots/k2999 - Ann".to_owned()),
            ]
        );
        assert_eq!(
            finding_at(&plan, "wearer_by_order", "Gloves/g2998 - Other").context,
            [
                ("slot".to_owned(), "06".to_owned()),
                ("id".to_owned(), "2601".to_owned()),
            ]
        );
        assert!(
            plan.findings
                .iter()
                .all(|m| m.code.code != "custom_model_unpaired"),
            "{:?}",
            plan.findings
        );
    }

    #[test]
    fn the_pairing_rounds_repeat_until_neither_kind_pairs() {
        // Round 1 pairs `g2701` by order — the gloves' one missing ID to
        // their one unworn folder — while the boots, one missing ID
        // beside two unworn folders, pair nothing; round 2's number rule
        // then has `k2701` follow `g2701`. A single pass would leave the
        // boots' ID beside `k2702` and refuse.
        let files = files(&[
            "Faces/XXX05 - Ann/face.fmdl",
            "Gloves/g2701 - Other/glove_l.fmdl",
            "Boots/k2701 - Other/boots.fmdl",
            "Boots/k2702 - Other/boots.fmdl",
        ]);
        let roster = wearing(&[(5, ("SaveA", 2500, 2500))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        assert_eq!(
            finding_at(&plan, "wearer_by_order", "Gloves/g2701 - Other").context,
            [
                ("slot".to_owned(), "05".to_owned()),
                ("id".to_owned(), "2500".to_owned()),
            ]
        );
        assert_eq!(
            finding_at(&plan, "wearer_by_number", "Boots/k2701 - Other").context,
            [
                ("slot".to_owned(), "05".to_owned()),
                ("id".to_owned(), "2500".to_owned()),
                ("from".to_owned(), "Gloves/g2701 - Other".to_owned()),
            ]
        );
        assert!(
            plan.findings
                .iter()
                .all(|m| m.code.code != "custom_model_unpaired"),
            "{:?}",
            plan.findings
        );
        assert!(plan.leftovers.contains("Boots/k2702 - Other/boots.fmdl"));
    }

    #[test]
    fn a_missing_id_the_other_kind_numbers_is_not_worn_and_outside_the_order() {
        // Slot 05's gloves 2780 is a boots folder's number: no custom
        // gloves, claimed as `not_worn` for `custom_model_not_worn`, and it
        // counts toward neither side of the order rule — which still pairs
        // 3000 to `g2999`.
        let files = files(&[
            "Faces/XXX05 - Ann/face.fmdl",
            "Faces/XXX06 - Bob/face.fmdl",
            "Boots/k2780 - A/boots.fmdl",
            "Gloves/g2999 - Other/glove_l.fmdl",
        ]);
        let roster = wearing(&[(5, ("SaveA", 0, 2780)), (6, ("SaveB", 0, 3000))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        assert!(plan.not_worn.contains(&(ItemKind::Gloves, 2780)));
        let finding = plan
            .findings
            .iter()
            .find(|m| m.code.code == "wearer_by_order")
            .expect("wearer_by_order expected");
        assert_eq!(
            finding.context,
            [
                ("slot".to_owned(), "06".to_owned()),
                ("id".to_owned(), "3000".to_owned()),
            ]
        );
        assert!(
            plan.findings
                .iter()
                .all(|m| m.code.code != "custom_model_unpaired"),
            "{:?}",
            plan.findings
        );
        // `k2780` itself is worn by nobody: a leftover as ever.
        assert!(plan.leftovers.contains("Boots/k2780 - A/boots.fmdl"));
    }

    #[test]
    fn the_order_rule_pairs_id_rank_to_folder_number_rank() {
        // `g567` lists after `g0999 - X` but numbers before it: the missing
        // IDs' sorted order meets the folder numbers', not the listing's —
        // 2800 to 567 (slot 06), 2900 to 999 (slot 05).
        let files = files(&["Gloves/g567/glove_l.fmdl", "Gloves/g0999 - X/glove_l.fmdl"]);
        let roster = wearing(&[(5, ("SaveA", 0, 2900)), (6, ("SaveB", 0, 2800))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        for (folder, slot, id) in [
            ("Gloves/g567", "06", "2800"),
            ("Gloves/g0999 - X", "05", "2900"),
        ] {
            assert_eq!(
                finding_at(&plan, "wearer_by_order", folder).context,
                [
                    ("slot".to_owned(), slot.to_owned()),
                    ("id".to_owned(), id.to_owned()),
                ],
                "{folder}"
            );
        }
        assert!(
            plan.findings
                .iter()
                .all(|m| m.code.code != "custom_model_unpaired"),
            "{:?}",
            plan.findings
        );
    }

    #[test]
    fn the_order_rule_counts_missing_ids_not_their_wearers() {
        // Two wearers of one missing ID beside one unworn folder: one ID to
        // one folder, so the order rule pairs it and both wear it shared.
        let files = files(&["Boots/k2701 - Other/boots.fmdl"]);
        let roster = wearing(&[(5, ("SaveA", 2500, 0)), (6, ("SaveB", 2500, 0))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        let mut by_order: Vec<Vec<(String, String)>> = plan
            .findings
            .iter()
            .filter(|m| {
                m.code.code == "wearer_by_order"
                    && matches!(
                        &m.scope,
                        Scope::Folder { path, .. } if path.as_str() == "Boots/k2701 - Other"
                    )
            })
            .map(|m| m.context.clone())
            .collect();
        by_order.sort();
        assert_eq!(
            by_order,
            [
                [
                    ("slot".to_owned(), "05".to_owned()),
                    ("id".to_owned(), "2500".to_owned()),
                ],
                [
                    ("slot".to_owned(), "06".to_owned()),
                    ("id".to_owned(), "2500".to_owned()),
                ],
            ]
        );
        assert!(
            plan.findings
                .iter()
                .all(|m| m.code.code != "custom_model_unpaired"),
            "{:?}",
            plan.findings
        );
        // Both wear it: the shared `Boots/Other/`, a link in each folder.
        assert!(plan.copies.contains(&(
            "Boots/k2701 - Other/boots.fmdl".to_owned(),
            "Boots/Other/boots.fmdl".to_owned()
        )));
        for marker in [
            "Players/05 - Other/Other.boots",
            "Players/06 - Other/Other.boots",
        ] {
            assert!(plan.markers.contains(&marker.to_owned()), "{marker}");
        }
    }

    #[test]
    fn the_name_rule_pairs_before_the_not_worn_check() {
        // The missing boots 2780 is a gloves folder's number — the "no
        // custom item" claim — but `k2999 - Ann` pairs it by name first, so
        // the player wears the folder instead.
        let files = files(&[
            "Faces/XXX05 - Ann/face.fmdl",
            "Boots/k2999 - Ann/boots.fmdl",
            "Gloves/g2780 - Other/glove_l.fmdl",
        ]);
        let roster = wearing(&[(5, ("SaveA", 2780, 0))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        assert_eq!(
            finding_at(&plan, "wearer_by_name", "Boots/k2999 - Ann").context,
            [
                ("slot".to_owned(), "05".to_owned()),
                ("id".to_owned(), "2780".to_owned()),
            ]
        );
        assert!(!plan.not_worn.contains(&(ItemKind::Boots, 2780)));
    }

    #[test]
    fn the_order_rule_needs_as_many_missing_ids_as_unworn_folders() {
        // Two missing boots IDs beside one unworn boots folder: the counts
        // differ so the order rule does not fire, each wearer is refused
        // his own ID, and the folder drops as a leftover.
        let files = files(&["Boots/k2701 - Other/boots.fmdl"]);
        let roster = wearing(&[(5, ("SaveA", 2500, 0)), (6, ("SaveB", 2600, 0))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        assert!(
            plan.findings
                .iter()
                .all(|m| m.code.code != "wearer_by_order"),
            "{:?}",
            plan.findings
        );
        let mut refused: Vec<Vec<(String, String)>> = plan
            .findings
            .iter()
            .filter(|m| m.code.code == "custom_model_unpaired")
            .map(|m| m.context.clone())
            .collect();
        refused.sort();
        assert_eq!(
            refused,
            [
                [
                    ("slot".to_owned(), "05".to_owned()),
                    ("key".to_owned(), "boots_id".to_owned()),
                    ("id".to_owned(), "2500".to_owned()),
                ],
                [
                    ("slot".to_owned(), "06".to_owned()),
                    ("key".to_owned(), "boots_id".to_owned()),
                    ("id".to_owned(), "2600".to_owned()),
                ],
            ]
        );
        assert!(plan.leftovers.contains("Boots/k2701 - Other/boots.fmdl"));
    }

    #[test]
    fn the_number_rule_needs_the_id_missing_of_its_own_kind() {
        // `k2452 - Ann` pairs the missing boots 2600 by name; `g2452`
        // shares its number, but no gloves ID is missing, so the number
        // rule does not fire and the folder drops as a leftover.
        let files = files(&[
            "Faces/XXX05 - Ann/face.fmdl",
            "Boots/k2452 - Ann/boots.fmdl",
            "Gloves/g2452 - Other/glove_l.fmdl",
        ]);
        let roster = wearing(&[(5, ("SaveA", 2600, 3))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        assert_eq!(
            finding_at(&plan, "wearer_by_name", "Boots/k2452 - Ann").context,
            [
                ("slot".to_owned(), "05".to_owned()),
                ("id".to_owned(), "2600".to_owned()),
            ]
        );
        assert!(
            plan.findings
                .iter()
                .all(|m| m.code.code != "wearer_by_number"),
            "{:?}",
            plan.findings
        );
        assert!(plan.leftovers.contains("Gloves/g2452 - Other/glove_l.fmdl"));
        assert!(
            plan.findings
                .iter()
                .all(|m| m.code.code != "custom_model_unpaired"),
            "{:?}",
            plan.findings
        );
    }

    #[test]
    fn a_stock_item_is_never_missing() {
        // Stock IDs (at or under 100) are never missing: the unworn folder
        // beside them pairs nothing and drops as a leftover. Boots 100, the
        // last stock ID, so the boundary itself is held.
        let files = files(&["Boots/k2701 - Other/boots.fmdl"]);
        let roster = wearing(&[(5, ("SaveA", 100, 3))]);
        let plan = plan_with_bytes(&files, Some(&roster), Engine::Fox, &[]);
        assert!(
            plan.findings
                .iter()
                .all(|m| !m.code.code.starts_with("wearer_by_")),
            "{:?}",
            plan.findings
        );
        assert!(
            plan.findings
                .iter()
                .all(|m| m.code.code != "custom_model_unpaired"),
            "{:?}",
            plan.findings
        );
        assert!(plan.leftovers.contains("Boots/k2701 - Other/boots.fmdl"));
    }
}
