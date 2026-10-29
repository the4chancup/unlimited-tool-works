# Unlimited Tool Works — Aesthetics export plan: Object model

Part of the [Aesthetics export plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Object Model

The pipeline operates on typed objects, not a flat dict of path strings to bytes: `parse_listing`
canonicalizes a flat `CanonicalListing` into a `vtree::VirtualTree`, and `vtree::ScopePath` spells
every path and issue scope the model carries.

**The model and its validation live in `libs/aesthetics_export`, not in the Team compiler.** The export format is
shared knowledge: the Export upgrader writes it, the Kit config editor edits kit folders inside it,
and the Team creator generates it from scratch — so one crate owns the object model, the
folder conventions described under "Player folders (the export format)", and the check rules behind
the format-level messages in the catalog. That includes the **structure parser** — canonical
folder/file listing, names/sizes, supplied small metadata such as raw `players.txt`, and link files
→ `ParsedAestheticsExport`, with no model or texture contents. The consumer creates the canonical listing
from a directory walk, ZIP central directory, or 7z entry table and chooses a source provider;
`aesthetics_export` does not own filesystem/archive I/O. Lazy per-folder *content* loading,
source-provider implementation, and the memory budget remain pipeline concerns. The Load step's
eager structure loading builds on this parser, and the Refs arranger supplies a plain-filesystem
listing. The Team compiler is the crate's biggest consumer, and the compile-time behavior
(processing steps, packing, bins, GUI, CLI) stays here. The Team compiler itself is deliberately
*not* split into a companion lib crate — see "Lib crates" in the [core plan](../core/README.md).

`TeamName`, `TeamId` and the `teams_list.txt` parse/reconcile logic live in the leaf crate
`libs/teams_list` (see the [library crates plan](../libs/README.md)); this crate depends on it for identity
resolution and contributes only the export-format half — splitting the display name into the
token that `TeamName::new` folds. `studio_core` never parses the list or interprets team IDs.

The eager object model stores **structure descriptors, not loaded contents**: the coordinator
later resolves each descriptor through the pipeline's source access (direct file, ZIP entry, or
solid-7z buffer) and materializes the task's files under the permit its batch holds; loaded bytes
stay charged until the writer commits their output, keeping the memory charge aligned with byte
lifetime.
`ParsedAestheticsExport`, `ValidatedAestheticsExport`, `ResolvedAestheticsExport`, `PlayerFolder`, and the descriptor
types belong to `aesthetics_export`; processing types such as `ModelFolder` and their
conversion/packing methods belong to `team_compiler`, avoiding inherent implementations of
shared-lib types in the tool crate.

**Validation boundary.** Structural validation (folder tree, naming, roster, link-file references,
file-type allowlists) lives in `aesthetics_export` and returns a `ValidationReport` containing the parsed
draft, every issue, and an optional **sanitized** `ValidatedAestheticsExport`: a bad optional file or
folder is omitted while valid siblings remain, with every original issue preserved for reporting.
Foundational failures — unsafe virtual paths, roster-wide ambiguity, unparsable required metadata —
never produce a valid affected scope and cannot be restored by `pass_through`; isolated malformed
roster lines remain `RosterEntry`/`DropSlot` findings. Invalid drafts remain renderable and
repairable in the GUI; unreadable sources and unsafe virtual paths are hard parser errors. Deep
format validation (FMDL/XML/MTL/glTF/texture/kit-config content checks) lives in the respective
format crates and applies the same sanitized-scope rule before planning. The consuming tool maps
`ValidationIssue`s to `Message`s with its own catalog policy — so the Refs arranger and Kit config
editor can consume the same structural checks without inheriting Team-compiler-specific catalog
entries.

The compile progression is **`ParsedAestheticsExport` → `ValidatedAestheticsExport` (structural + deep
validation, sanitized) → `ResolvedAestheticsExport` (identity attached) → build manifest (run-level
planning)**. Parsing retains raw roster entries and issues so malformed input stays reportable;
planning resolves dependencies, IDs, destinations, and collisions serially before parallel work
starts.

### Validation semantics

These settle the Team compiler plan's pre-Phase-3 gates (identity boundary, sanitized versus
eligible, roster-entry scope and disposition; confirmed 2026-09-30).

- **Identity boundary.** The export kind follows from the canonical `team_name`: `/refs/` is a
  referee export, `/balls/` never reaches this crate (the consumer's discovery skips it), anything
  else is a normal team. `validate` derives the kind from `team_name` itself, so the referee rules
  (required `players.txt`, slots 01–35, repeated folders) need no input from the consumer.
  `resolve_identity` maps `/refs/` to `ExportIdentity::Referees` without reading the teams list,
  and any other name to `ExportIdentity::Team { id, name }` by `TeamsList` lookup, or to
  `IdentityError` (the Team compiler's `team_name_unknown`). No referee value is ever a `TeamId`;
  999 appears only where a game format writes it. An export stem with no token has no team name:
  it is reported as `team_name_unknown` with an empty name and dropped.
- **Coverage tag.** A team export's name says what the export covers, as its second word, right
  after the team name, in any letter case: `Full` (everything the team has: `co Full Spring
  26`) or `Midcup` (changes to add to what is installed: `co Midcup Day 5`). The compiler needs
  to know, because the game offers as many kits as the team's record counts and (Fox) reads a
  player's boots from his row: only a full export may reset what the team had before, and
  nothing in an export's content tells the two apart (a full export of a team with two kits
  looks like a midcup export bringing two kits). It is in the name, not in a marker file, so
  whoever handles the export sees it without opening it, and a name with neither word is an
  error that drops the export (the Team compiler's `export_tag_missing`) rather than a guess:
  a full export taken for a midcup one leaves the kits of a past cup on offer, and the
  reverse wipes a team's kits. Only the second word counts, so a later `Full` in a
  description changes nothing. A referee export carries no tag: it is always full.
- **Who decides a consequence.** This crate decides each structural issue's disposition, because
  it builds the sanitized export from them: a `ValidationIssue` carries its code, scope, context,
  effective disposition, and whether `pass_through` changed it. The consuming tool owns only
  severity, text and hint (the Team compiler's catalog in `messages.md`); the tool tests that
  every code in `ISSUE_CODES` has a catalog row, and its scenario tests observe the consequence
  each row states. Settings that change a
  consequence reach the crate through `ValidationContext`: the target `PesVersion` (Fox or pre-Fox
  allowed names), `strict_file_type_check`, `pass_through`, and the texture stems the installed
  CPKs hold in the team's Common output, which on PES 18-21 satisfy a texture `.common` link
  whose target is
  not in `Common/` (`spec/team_compiler/pipeline.md` "Resolved decisions", "A texture a model names
  must exist").
- **Sanitized versus eligible.** `ValidatedAestheticsExport` holds exactly the eligible content:
  every item no issue drops. `ValidationReport.parsed` still holds everything and `issues` every
  finding, so a dropped folder stays renderable and a broken roster repairable. `validated` is
  `None` exactly when some issue's effective disposition is `DropExport`. `pass_through` turns an
  eligible `DropFile`/`DropFolder` into `Keep`: the item stays in the validated export and its
  issue stays in the report, marked as passed through, so the tool reports it at its severity and
  finishes that scope as `DoneWithErrors`. `DropSlot`, `DropExport` and the codes the catalog
  marks not eligible are never kept. The deep pass applies the same rule ("Content findings").
- **Content findings.** This crate reads no file content, so the deep pass's findings come from
  the consumer, which checks the files of the first report's `validated` (what the structure
  pass dropped is not read). `ValidationReport::with_content_findings` takes them and returns
  the report validation would have made had they been its own. A `ContentFinding` carries the
  consumer's code (its own; the one `ISSUE_CODES` member a consumer also reports is
  `logo_file_invalid`, for a logo that does not decode), the scope, the context, the disposition asked
  for, and whether `pass_through` may keep it; an eligible `DropFile`/`DropFolder` becomes
  `Keep` under `pass_through`, marked as passed through, like the structure pass's own. In the
  report's issues they stand after the model folders' and `Common/`'s own findings and before
  the cascade (the kit, portrait and root-file findings follow the cascade), so a
  player or shared folder linking a shared folder or `Common/` file that a content finding
  drops gets
  `link_target_dropped` naming that finding, and a shared folder left with no linking player
  `shared_folder_orphaned`. What a scope drops: a `Folder` scope a player, shared or kit
  folder, whole (a finding on a file below a subfolder is its root folder's, whose output the
  subfolder's is merged into); a `File` scope that file alone, wherever it sits (a `Common/`, `Portraits/` or
  `Collars/` file, a logo file, the logo going as one unit, a player or shared folder's own
  file, `settings.toml` and `portrait.*` included), the folder keeping everything else; until
  4.y-fix20 a player or shared folder's own file stayed in the folder's file list after its
  `DropFile`, so a task still read it; `DropExport` on the `Export` scope leaves
  `validated: None`.
- **Roster entries.** A finding about one `players.txt` (or `refs.txt`) line is `RosterEntry`-scoped:
  the file, the 1-based line number, and the slot when it parsed. Line-local findings
  (`players_txt_line_invalid`, `players_txt_slot_invalid`, `players_txt_target_missing`) are
  `DropSlot`: that assignment goes and the other lines stand. Duplicate slots are found first,
  over every complete line (a slot and a folder name) whose slot is in range, before any target
  is looked up (a bare `03` is `players_txt_line_invalid` and claims nothing): a stale
  `03 OldName` above `03 NewName` is a duplicate even when `OldName` does not exist (both findings
  are reported). Roster-wide findings drop the
  export: a slot assigned twice (`players_txt_slot_duplicate`, on each later line), a file that is
  not UTF-8, and a referee roster left with no valid assignment (`players_txt_invalid`,
  `File`-scoped). A folder named by a line counts as listed even when that line is dropped, so it
  gets no `player_unlisted` on top; a line whose slot does not parse names no folder. A folder no
  remaining assignment maps is not compiled, and its links reference nothing: a shared folder
  counts as referenced only by an eligible, roster-mapped player (otherwise
  `shared_folder_orphaned`). Without `players.txt`, number findings are
  `Folder`-scoped `DropFolder`: `player_folder_number_invalid`, and `player_number_duplicate` on
  every folder claiming the slot.
- **Dropped link targets.** Sanitizing never leaves a reference to something it removed. A
  player or shared folder whose shared-folder or `.common` link names a target a pass
  dropped
  (a shared folder discarded for a disallowed file, a Common file discarded as disallowed) is
  dropped too, with `link_target_dropped` naming the link and the target's own finding. The
  trigger is the target's effective disposition: under `pass_through` a kept target keeps its
  dependants and no `link_target_dropped` is reported. A link a finding drops itself, with its
  target (the deep pass's `model_hidden_dropped` on a `.common` link to a Common model that
  draws nothing), is no link: nothing references the target, so no `link_target_dropped`, and
  the folder holding the link stands with its other files. A texture `.common` link naming a
  kit set (`kitN/armor_bsm.dds.common`) reaches every member of the set: a dropped member is an
  absent one, a gap the set's completion fills as it fills any other, so the link's folder is
  dropped only when every member is, `link_target_dropped` naming the first member, not when
  one of several goes.
- **Unread metadata.** The CLI's `check` and `compile` read every small metadata file, from a
  solid `.7z` too (the buffer that read decompresses is released before the next archive is
  admitted; compiling decompresses again); only the GUI's shallow live check leaves a solid archive's roster unread, and
  that state (a report that is neither valid nor refused) is designed with live validation in
  Phase 8. `SmallMetadata` distinguishes a file that is absent from one that was not read, so
  that addition changes no shape.
- **Refused listings.** A listing `vtree` refuses (a path escaping the root, two names that fold
  to one) is a `SourceError` from `parse_listing`: there is no draft to report on, and the
  consumer reports it as a source failure naming the path (the Team compiler's
  `export_extract_failed`).
- **File-type allowlist.** What a file may be depends on where it sits; a file the row does not
  admit gets the row's code. *Model content* is models (any format), textures, `.skl`, `.fclo`,
  `.xml`, `.mtl`, material tomls and `.bin`, and `.txt` notes, which no package reads
  (`file_not_used`): makers leave one beside a model, and dropping the folder for it would
  lose a working model to a comment (step 5.0e). A note is a `.txt` whose name without it is no
  known kind (`notes.txt`, `readme.txt`); `settings.toml.txt` or `skin.dds.txt`, a real file
  saved with a hidden extension, stays disallowed, so the mistake is named.

  | Where the file sits | Admits | Otherwise |
  |---|---|---|
  | directly in a player folder | model content, shared and `.common` links, the `fpc_on` and `fpc_off` markers, `settings.toml` and `ingame_face.toml` (the marker's old spellings `ingame_face` and `ingame_face.txt` are `ingame_face_marker_renamed`, not disallowed) | `file_type_disallowed` |
  | anywhere below a player folder's subfolder (a player folder of its own, `player_folders.md` "Subfolders") | model content but the per-player singletons (`face.xml`, the face diffs, `fcl_hair_sim.fclo`, which count in the root alone), links (a shared link read as the root's; a `.common` link stands for the `Common/` file at the link's own path below the player folder, `spec/model_format.md` "Link files") | `file_type_disallowed` |
  | anywhere in a shared folder (its subfolders are its own, `player_folders.md` "Subfolders") | model content (below a subfolder, but the singletons, as below a player folder's), `.common` links | `file_type_disallowed` |
  | directly in a kit folder | textures, `config.toml`, `colors.txt`, the `pre-fox`, `fox` and `icon_<N>` markers | `file_type_disallowed` |
  | anywhere in `Common/` | model content (a subfolder's is read on PES 18-21 when a link names it or a linked model or `.mtl` brings it, `spec/team_compiler/pipeline.md` "Common") | `common_file_disallowed` |
  | directly in `Collars/` | model files (any model format; the game draws a collar with the kit texture, so it has no textures or materials of its own, `spec/team_compiler/pipeline.md` "Collars"); their `collar_<ID>` name is the compiler's to check, since which IDs exist depends on the target version (`collar_id_invalid`) | `file_type_disallowed` |
  | anywhere in `Kits/all/` | textures directly in it | `kit_all_file_ignored` |
  | anywhere in `Portraits/` | textures named `NN` directly in it (`NN` a slot 01-23, alone or followed by ` - ` and any text, as a player folder is numbered) | `portrait_name_invalid` |
  | at the root | `players.txt`, `notes.txt`, `colors.txt`, `README.txt`, `logo*` textures; a referee export also `refs.txt`, `ref_lists.txt`, `ref_marker.dds` | `root_file_unexpected` |

  A file below a subfolder the table does not name takes the code of the folder holding that
  subfolder (`Kits/p1/extra/x.dds` is `file_type_disallowed`). The list is the same for
  every target: each model
  format is a source for either engine through conversion, so what a target does not emit is the
  deep pass's and processing's concern, not the allowlist's (Red's per-engine lists predate
  conversion). With `strict_file_type_check` off, `file_type_disallowed` and
  `common_file_disallowed` keep their item (`Keep`, which the consumer shows as Info) and the file
  stays in the folder's files; a task reads only the files its roles name, so a kept file no
  role takes is not emitted (a kit emits only its named textures; the Common tasks on PES 18-21 only the
  files directly in `Common/` and those a link names or brings), and that finding is the one line the
  member sees for it.
- **OS artifacts.** `Thumbs.db`, `desktop.ini` and `.DS_Store` (any case, at any depth) are
  written by file browsers, never by authors: `parse_listing` leaves them out of the draft and
  reports nothing. On 2026-10-01 the maintainer's team exports held 209 `Thumbs.db` and 16
  `desktop.ini` files inside model folders; as disallowed files, each would have dropped its folder
  under the default strict check.
- **Folder names.** A player folder name (without `players.txt`) and a kit folder name are
  `<head>[ - <label>]`: split at the first `-`, both sides trimmed. The head is the slot (decimal
  `01`–`23`, or a `KitSlot`, or `all`); the label is the player name or the kit label, and a
  player folder without one is named by its whole folder name, as every folder listed in
  `players.txt` is. `03 - Jean-Pierre` is player 03, Jean-Pierre.
- **Issue scope.** An issue's scope names the item its disposition acts on. `DropFile` issues are
  `File`-scoped; a disallowed file's `file_type_disallowed` is `Folder`-scoped in a player, shared
  or kit folder (the folder it drops) and `File`-scoped in `Collars/` (the file alone), one issue
  per file with the file in its context. Roster findings keep the scopes above.
- **Pass-through eligibility.** In the structure pass, exactly `link_target_missing`,
  `common_link_missing`, `file_type_disallowed` and `common_file_disallowed` are eligible; every
  other drop it decides belongs to one of the not-eligible classes in `team_compiler/messages.md`
  "Message structure".
- **Drop order.** A folder's own findings come first, then `link_target_dropped` for each player
  or shared folder whose link target was dropped, then `shared_folder_orphaned` over the players still eligible
  and roster-mapped: a shared folder linked only by a dropped player is orphaned.
- **Export findings of `validate`.** A stem with no token is `team_name_unknown` with an empty
  name, and an export holding no content-folder entry and no root `logo*` texture is
  `export_empty`, both `DropExport`. No finding of `validate` is reported beside
  `nested_root_ambiguous` or `nested_root_conflict`, which leave the root undecided (its
  folders are the candidate roots and its files are not yet the export's root files) and skip
  the export, as `export_layout_old` does: the export's name and root files are judged once
  its root is. A name the teams list lacks is
  `resolve_identity`'s `IdentityError`.
- **Texture stems.** A lookup namespace (`team_compiler/messages.md` `texture_stem_conflict`)
  holds its textures plus, in a player folder or, on PES 18-21, a shared one (a shared folder's
  links stay unread on PES 15-17), each texture `.common` link under its linked name
  (`hair.png.common` is `hair`), as a link counts as the linked file being local
  (`spec/model_format.md` "Rules"). Each directory of `Common/` is a namespace of its own, as each
  folder of a player's tree is: its conflicting files drop each other (`File`, `DropFile`),
  and the cascade then drops the players linking them.
- **Kits, portraits, logo.** A `kit_folder_invalid` folder gets no other finding: which
  allowlist its contents answer to is unknown (it may be a misspelled `all`), and it is dropped
  whole anyway. `all` counts as a slot for `kit_slot_duplicate`. `all/` textures
  follow a kit's `kit` prefix rule (`kit_texture_name_invalid`). `kit_all_unused` (an `all/` no kit inherits —
  a `Full` export's mandatory empty `p1`/`g1` count as kits) changes nothing (`Keep`). `kit_textures_inherited` lists the inherited stems without
  their `kit_` prefix, alphabetically (`back, leg, name`). A kit's icon is the number in its one
  `icon_<N>` marker's name: one numbered above 23 is `kit_icon_invalid`, and two or more are
  each `kit_icon_invalid` whatever their numbers (`File`, `DropFile`; the kit keeps the default
  icon).
  `Portraits/NN.*` maps to slot NN, the name read as a player folder's (`NN`,
  alone or followed by ` - ` and any text); two files of one slot there are
  `texture_stem_conflict` (the context's `stem` the slot's `NN`), dropping both
  files. Any other name is `portrait_name_invalid`, `player_NN` included: the folder already
  says what the file is, and the `player_` prefix was the old format's, which the upgrader
  rewrites. The rule is the player folder's (the stem split at its first `-`, both sides
  trimmed, the head all digits making a slot), so `05-Bob.png` is slot 05 as a folder `05-Bob`
  is and `081.dds` is no slot 08: one rule for the two places a member types a slot.
  `portrait_conflict` compares contents, so it
  belongs to the deep pass. Every root file whose stem starts with `logo` is a logo
  candidate; `logo_file_invalid`, `logo_role_duplicate` and `logo_small_without_main` each drop
  the logo as one unit (`File`-scoped on the offending file, `DropFile`, `logo: None`).
  `team_colors_missing` belongs to the bins stage.

### `aesthetics_export` crate layout

Five consumers (Team compiler, Export upgrader, Kit config editor, Refs arranger, future Team
Creator) read this crate, so its layout is organized by the **progression stage** a consumer can
stop at, and each stage is produced by the previous one's entry point:

```
crates/libs/aesthetics_export/src/
├── lib.rs              # re-exports; the progression's entry points
├── listing.rs          # CanonicalListing, SmallMetadata, ValidationContext: what the consumer supplies
├── slots.rs            # PlayerSlot, RefSlot
├── conventions/        # the export format as data, no logic
│   ├── mod.rs          #   content-folder names, small-metadata names, OS artifacts, the `<head>[ - <label>]` split
│   ├── player_folder.rs#   the model-name suffix table (ModelSuffix) and the per-player singletons
│   ├── kit_token.rs    #   the kitN / kit1–kit9 / kitF kit tokens in file stems (kit-dependent assets)
│   └── file_types.rs   #   FileKind and the name → kind classification, the `.txt` tolerance
├── parse/              # listing → ParsedAestheticsExport (raw, everything retained)
│   ├── mod.rs          #   canonicalization and root normalization
│   ├── identity.rs     #   team name (first token) and coverage tag (second) from the display stem
│   ├── roster.rs       #   players.txt (a referee export's refs.txt alias) → RawRoster (malformed lines retained)
│   └── draft.rs        #   AestheticsExportDraft, FolderDraft, FileDescriptor (structure only)
├── validate/           # ParsedAestheticsExport → ValidationReport { parsed, validated, issues }
│   ├── mod.rs          #   the sanitized-scope rule; foundational vs isolated failures
│   ├── roster.rs       #   ValidatedRoster: numbers, duplicates, DropSlot semantics
│   ├── links.rs        #   link files (Name.boots, .common) resolved against shared folders and Common, and the cascade
│   ├── folders.rs      #   FolderDraft → PlayerFolder / SharedModelFolder; the player, shared, Common and Collars checks
│   ├── kits.rs         #   the kit folders' own findings and the sanitized KitsFolder (all/ inheritance)
│   ├── root.rs         #   Portraits/, the logo* grammar, the root-file allowlist, the small root artifacts
│   ├── issues.rs       #   ValidationIssue, ISSUE_CODES (stable code, scope, context) — no message text
│   └── tests.rs        #   the validation tests (cfg(test))
├── resolve.rs          # ValidatedAestheticsExport + teams_list::TeamsList → ResolvedAestheticsExport (ExportIdentity, TeamId)
├── colors_txt.rs       # colors.txt reader: the "Colors" grammar, bytes in, colors and refused lines out
└── testing.rs          # shared listing/metadata builders for the crate's tests (cfg(test))
```

Placement rules:

- **No I/O.** Consumers supply a `CanonicalListing` (from a directory walk, ZIP central directory,
  or 7z entry table) and, where content is needed for deep checks, the bytes; this crate never opens
  a file or archive.
- **No message text.** `validate/issues.rs` yields stable codes with scope and context; each tool
  maps them to its own catalog. This is what lets the Refs arranger and Kit config editor share the
  checks without inheriting Team compiler wording.
- **Structure, not contents.** Descriptor types carry names and sizes; processing model/texture
  types (`ModelFolder`, …) and everything that converts or packs them live in `team_compiler`.
- **Conventions are data.** A new allowed model name or kit file is a table entry in
  `conventions/`, and every consumer picks it up; no consumer keeps its own list.
- **Deep format validation is not here.** FMDL/XML/MTL/glTF/texture/kit-config content checks live
  in their format crates; `validate/` only orchestrates the structural layer and the sanitized-scope
  rule that deep checks then reuse.
- `wasm32`-checkable and GUI-free (it is the Studio Web cheap tier's export knowledge).

### Core types

```rust
pub struct ParsedAestheticsExport {
    pub draft: AestheticsExportDraft,
    pub raw_roster: Option<RawRoster>,   // None: no roster file (numbered folder names instead)
    pub metadata: SmallMetadata,         // as supplied; validate reads notes.txt from it
    pub issues: Vec<ValidationIssue>,    // the parse's own (root normalization, the roster read)
}

pub struct ValidationReport {
    pub parsed: ParsedAestheticsExport,
    pub validated: Option<ValidatedAestheticsExport>,
    pub issues: Vec<ValidationIssue>,    // every issue: `parsed.issues`, then validation's
}

pub struct ValidatedAestheticsExport {
    pub export_display_name: String,     // complete archive/folder stem; presentation/source identity
    pub team_name: TeamName,               // canonical /xx/ name derived from the first word
    pub coverage: ExportCoverage,        // from the name's second word; Full for a referee export
    pub players: Vec<PlayerFolder>,      // sanitized main reference point for each eligible player
    pub roster: ValidatedRoster,         // normalized strong slots → player indices
    pub faces: Vec<SharedModelFolder>,   // shared folders, referenced by name from player folders
    pub boots: Vec<SharedModelFolder>,   //   (no IDs — IDs are assigned by run planning)
    pub gloves: Vec<SharedModelFolder>,
    pub kits: KitsFolder,                // per-kit subfolders (config.toml + colors.txt + textures)
    pub portraits: BTreeMap<PlayerSlot, FileDescriptor>, // `Portraits/NN.*`
    pub logo: Option<LogoFiles>,         // root `logo*` (+ optional `logo_small*`), each with its fit mode (see "Root files")
    pub collars: Vec<FileDescriptor>,    // `Collars/`, passed through
    pub common: Vec<FileDescriptor>,     // `Common/`, the targets of `.common` links
    pub root: RootArtifacts,             // sanitized root colors/notes/referee marker
}

pub struct LogoFiles {
    pub main: LogoFile,
    pub small: Option<LogoFile>,
}
pub struct LogoFile {
    pub file: FileDescriptor,
    pub fit: Option<LogoFit>,            // the stem's tag; None: untagged (`fit` if not square)
}
pub enum LogoFit { Crop, Stretch, Fit }

pub struct RootArtifacts {
    pub team_colors: Option<FileDescriptor>,
    pub notes: Option<FileDescriptor>,
    pub referee_marker: Option<FileDescriptor>,
}
// Invalid optional root artifacts are absent here but remain in ValidationReport.

pub struct SharedModelFolder {
    pub path: vtree::ScopePath,        // `Boots/Crocs`: the scope its issues name
    pub folder_name: String,           // the path's last segment — the link lookup key
    pub files: Vec<FileDescriptor>,    // every file below it
}

pub struct KitsFolder {
    pub kits: BTreeMap<KitSlot, KitFolder>,  // one per kit folder; `all/` is not a kit; a Full team export
                                             // with no surviving player kit gets an empty `Kits/p1`, one with
                                             // no surviving `g1` an empty `Kits/g1` (every team needs one of
                                             // each: `spec/team_compiler/pipeline.md` "Bins accumulation")
    pub shared: Vec<FileDescriptor>,         // `all/` textures as found (each also appears, as Shared, in every kit lacking that stem)
}

pub struct KitFolder {
    pub path: ScopePath,                     // `Kits/p1` or `Kits/p1 - Lakers`, as the export spells it:
                                             // the scope a consumer's kit findings name
    pub label: Option<String>,               // the free part after ` - `; GUI/editor display only
    pub config: Option<FileDescriptor>,      // config.toml (absent → generated at compile time)
    pub colors: Option<FileDescriptor>,      // colors.txt (grammar: colors_txt.rs)
    pub icon: Option<u8>,                    // the icon_<N> marker's number, 0–23; None: absent or kit_icon_invalid (the default 3 applies)
    pub layout: Option<KitLayout>,           // `fox` / `pre-fox` marker file; None = drawn for the target engine
    pub textures: Vec<KitTexture>,           // the *effective* set: own files, plus `all/` files for stems the kit lacks
}

pub enum KitLayout { PreFox, Fox }          // which engine's kit UV layout `kit` and its mask/srm are drawn for

pub struct KitTexture {
    pub stem: String,                        // `kit`, `kit_back`, … (lowercased)
    pub file: FileDescriptor,
    pub source: KitTextureSource,            // Own | Shared — provenance for the editor and `kit_textures_inherited`
}
pub enum KitTextureSource { Own, Shared }
// Inheritance is resolved here, once, so the compiler, the Kit config editor and the upgrader
// see the same effective set and derive the same texture-name fields from it.

pub struct PlayerIndex(pub usize);

pub enum ValidatedRoster {
    Team(BTreeMap<PlayerSlot, PlayerIndex>),
    Referees(BTreeMap<RefSlot, PlayerIndex>),
}
// Parsing retains raw roster entries and issues so duplicate slots remain
// reportable. A referee roster may map several RefSlots to the same PlayerIndex.

pub enum ExportIdentity {
    Team { id: TeamId, name: TeamName },   // TeamId is strictly 701–920
    Referees,                            // rendered as fixed game ID 999 only at format boundaries
}

pub struct ResolvedAestheticsExport {
    pub export: ValidatedAestheticsExport,
    pub identity: ExportIdentity,
}
```

The compiler supports **only the Studio export format**. The old format (separate Faces/Boots/Gloves
item folders with embedded IDs, Kit Configs/Kit Textures folders, the Other folder) is not supported
— old exports are migrated with the Export upgrader tool (see the [Export upgrader
plan](../export_upgrader.md)). An old-layout export that reaches the compiler anyway is one
finding, `export_layout_old`, and is skipped: a root folder named `Kit Configs`, `Kit Textures`
or `Other`, which the Studio format never has, tells the two layouts apart with no other
old-format knowledge, and the dozens of structure findings the old folders would otherwise
earn (the converge census of 2026-10-09 ran 55 old exports: each a wall of
`export_tag_missing`, `shared_folder_orphaned` and `portrait_name_invalid`) would only say
the same thing badly. Consequences:

- No `KitConfigFolder`/`KitTextureFolder`: kits live in the `Kits/` folder, one subfolder per kit.
- No `Other` folder support.
- `Faces`/`Boots`/`Gloves` folders still exist in the Studio format for models **shared between
  players**, but they are identified by **name only** (no embedded IDs). Player folders are the main
  reference point: a player folder references a shared folder with a link file (e.g. `Crocs.boots`).
  At compile time, each shared **boots/gloves** folder gets an auto-assigned ID, and every linked
  player gets that ID (Fox: his `BootsList`/`GloveList` row in the output CPK; pre-Fox: written to
  the savefile; only if the shared folder's output actually commits).
  Shared *face* folders take no ID: pre-Fox copies the shared face folder per linked player (local
  files layered on top), and Fox merges the parts into each player's own FMDL — see "A link plus
  local models combines" below.

```rust
// libs/aesthetics_export: parsing preserves invalid raw input for diagnostics and is
// source-neutral: the consumer supplies a canonical listing (from a directory walk,
// ZIP directory, or 7z entry table) plus small metadata; aesthetics_export owns no I/O.
pub fn parse_listing(
    listing: CanonicalListing,
    metadata: SmallMetadata,
) -> Result<ParsedAestheticsExport, SourceError>;
impl ParsedAestheticsExport {
    // Infallible: every structural failure is a ValidationIssue, a foundational one included
    // (DropExport, `validated: None`); what cannot be parsed at all is parse_listing's SourceError.
    pub fn validate(self, context: &ValidationContext) -> ValidationReport;
}
impl ValidatedAestheticsExport {
    pub fn resolve_identity(self, teams_list: &TeamsList)
        -> Result<ResolvedAestheticsExport, IdentityError>;
}

// tools/team_compiler: serial run-level planning over all resolved exports
// produces the immutable manifest; scoped failures drop tasks/exports while
// a manifest for valid siblings still proceeds (partial planning). `plan_run`,
// `process_task` and their types are the tool's own: their shapes live in
// `spec/team_compiler/pipeline.md` "Run driver shapes", not here.
// Model tasks carry immutable IDs assigned by plan_run; processing only consumes
// those assignments, never allocates. The writer commits successful batches and
// releases their permits.
```

### Structure pass types

The inputs, the draft, the issues and the small value types of the structure pass. The Core types
above are the sanitized output; everything here is what reaches it.

```rust
// ---- Input (listing.rs). The consumer lists one export source; parse_listing canonicalizes.
pub struct CanonicalListing {
    pub display_name: String,        // the source's stem: folder name, or archive name without extension
    pub entries: Vec<ListedEntry>,   // any order
}
pub struct ListedEntry {
    pub path: String,                // as the source spells it, relative to the source root
    pub kind: ListedKind,
}
pub enum ListedKind {
    File { size: u64 },
    Folder,                          // needed only for a folder with nothing below it (an empty kit)
}

/// The small files the structure pass reads, keyed by `ListedEntry::path`. The consumer reads
/// every listed file `is_small_metadata` accepts; a failed read carries its reason
/// (`source_read_failed`, with the disposition of what the file is: `DropExport` for a roster,
/// `DropFile` for `notes.txt`). A listed file missing from the map was not read:
/// Phase 3 treats it as a failed read; the GUI's shallow check gives it its own state in Phase 8.
pub struct SmallMetadata {
    pub files: BTreeMap<String, Result<Vec<u8>, String>>,
}
// conventions/mod.rs: `players.txt`, `refs.txt`, `notes.txt`, by name
// (case-insensitive) at any depth, so the consumer needs no root normalization of its own.
pub fn is_small_metadata(path: &str) -> bool;

pub struct ValidationContext {
    pub version: PesVersion,         // Fox or pre-Fox allowed names
    pub strict_file_type_check: bool,
    pub pass_through: bool,
    // Folded stems of the textures the installed CPKs loaded before the one being compiled
    // hold in the export's team's Common output: a texture `.common` link naming one is not
    // `common_link_missing` (empty when the lookup cannot be made)
    pub installed_common_textures: BTreeSet<String>,
}

// ---- Errors. A listing that cannot become a tree: the consumer's `export_extract_failed`.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("{path}: {error}")]
    Path { path: String, error: vtree::PathError },    // escapes the root, not canonical
    #[error("{path}: {error}")]
    Collision { path: String, error: vtree::InsertError }, // two names fold to one
}
/// The canonical team name has no teams-list row (`team_name_unknown`).
#[derive(Debug, thiserror::Error)]
#[error("team {team_name} is not in the teams list")]
pub struct IdentityError { pub team_name: TeamName }

/// What a team export covers, from the second word of its name ("Coverage tag").
pub enum ExportCoverage {
    Full,    // everything the team has: the compiler rebuilds the team's records from it alone
    Midcup,  // additions: the compiler replaces only what the export holds
}

// ---- Draft (parse/). Everything the normalized tree holds, grouped by content folder,
// nothing dropped; parse_listing first normalizes the root (`team_compiler/pipeline.md`
// "Per-export serial steps", step 1), reporting nested_folders_fixed, nested_root_ambiguous
// and nested_root_conflict.
pub struct AestheticsExportDraft {
    pub export_display_name: String,
    pub team_name: Option<TeamName>,          // None: the stem has no token (team_name_unknown, empty name)
    pub coverage: Option<ExportCoverage>,     // None: a team export's second word is neither tag (export_tag_missing)
    pub root_files: Vec<FileDescriptor>,      // files directly at the root
    pub root_folders: Vec<vtree::ScopePath>,  // root folders that are no content folder (`wrapper/`)
    pub stray_files: Vec<FileDescriptor>,     // files directly in Players/, Kits/, Faces/, Boots/, Gloves/
    pub players: Vec<FolderDraft>,
    pub faces: Vec<FolderDraft>,
    pub boots: Vec<FolderDraft>,
    pub gloves: Vec<FolderDraft>,
    pub kits: Vec<FolderDraft>,               // `all/` included
    pub portraits: Vec<FileDescriptor>,
    pub collars: Vec<FileDescriptor>,
    pub common: Vec<FileDescriptor>,
}
impl AestheticsExportDraft {
    pub fn kind(&self) -> ExportKind;         // Referees exactly when team_name is `/refs/`
}
pub struct FolderDraft {
    pub path: vtree::ScopePath,               // `Players/03 - A`, `Kits/p1 - Lakers`
    pub files: Vec<FileDescriptor>,           // every file below it, subfolders included
}
pub enum ExportKind { Team, Referees }

/// The authoritative roster file: `players.txt`, or a referee export's `refs.txt` alias when
/// it has no `players.txt`. A file that is not UTF-8 has no entries (players_txt_invalid).
pub struct RawRoster {
    pub file: vtree::ScopePath,
    pub entries: Vec<RawRosterEntry>,         // nonblank lines, in file order
}
pub struct RawRosterEntry {
    pub line: usize,                          // 1-based
    pub slot: Option<u16>,                    // the leading decimal, range unchecked; None: none parsed
    pub folder_name: Option<String>,          // the trimmed remainder; None: a bare slot
}

// ---- Issues (validate/issues.rs). The consumer maps each to its own Message.
pub struct ValidationIssue {
    pub code: &'static str,                   // stable; always one of ISSUE_CODES
    pub scope: IssueScope,
    pub context: Vec<(&'static str, String)>, // structured fields, in template order
    pub disposition: Disposition,             // effective: Keep when pass_through kept the item
    pub passed_through: bool,                 // pass_through turned a DropFile/DropFolder into Keep
}
pub enum IssueScope {                         // within the one export this crate sees
    Export,
    Folder(vtree::ScopePath),
    File(vtree::ScopePath),
    RosterEntry { file: vtree::ScopePath, line: usize, slot: Option<u16> },
}
pub enum Disposition { Keep, DropFile, DropSlot, DropFolder, DropExport }
pub const ISSUE_CODES: &[&str];               // every code the crate emits

// ---- File kinds (conventions/file_types.rs), from the name alone. A stray `.txt` after a
// link or marker name is tolerated. The allowlist is, per folder kind, the set of kinds it
// admits; `Other` is admitted nowhere, and strict_file_type_check changes only the disposition.
pub enum FileKind {
    Model(ModelFormat),
    Texture,                  // an accepted image extension
    Skl, Fclo, Xml, Mtl, MaterialsToml,
    Bin,                      // `.bin`: a game bin (`face_diff.bin`) or a glTF buffer; deep glTF parsing tells them apart
    SharedLink(SharedKind),   // `Crocs.boots`, `Longhair.face`, `Keeper gloves.gloves`
    CommonLink,               // `<name>.common`: stands in for `Common/<name>`
    Marker(Marker),
    Metadata(MetadataFile),
    Other,
}
pub enum ModelFormat { Fmdl, PesModel, Gltf }   // `.fmdl`, `.model`, `.glb`/`.gltf`
pub enum Marker { OldIngameFace, FpcOn, FpcOff, PreFox, Fox, Icon, WriteNames }
                                    // Icon: `icon_<N>`, the number in the name; OldIngameFace:
                                    // the in-game face marker's old spellings `ingame_face` and
                                    // `ingame_face.txt`, recognized only to be reported
                                    // (`ingame_face_marker_renamed`)
pub enum MetadataFile {
    PlayersTxt, RefsTxt, RefLists, NotesTxt, ColorsTxt, ConfigToml, SettingsToml, IngameFaceToml,
    Readme,
}

// ---- Slots.
pub struct PlayerSlot(u8);                    // 01–23: a normal team's roster slot
impl PlayerSlot {
    pub fn new(slot: u8) -> Option<PlayerSlot>;
    pub fn get(self) -> u8;
    pub fn player_id(self, team: TeamId) -> u32; // team * 100 + slot (92023 at most: past u16)
}
pub struct RefSlot(u8);                       // 01–35: a referee roster slot
impl RefSlot {
    pub fn new(slot: u8) -> Option<RefSlot>;
    pub fn get(self) -> u8;
}
```

Kits key on `kit_config::KitSlot` ("Design constraints"). A kit's `config.toml` is read and
written by `kit_config`'s own TOML form (`KitConfig::from_toml`, `to_toml`, the comment-preserving
`update_toml`), not by a bridge here: the format crate owns both forms of its format, and this
crate reads no file content.

### Player folders

The new per-player export format (see `player_folders.md` "Player folders (the Studio export format)") is represented as
its own type, which run planning turns into the folder's `ModelFolder` tasks, its files routed to
the player's packages by role (`plan/roles.rs`):

```rust
pub struct PlayerFolder {
    // No number field: numbering is a roster property, not folder content —
    // slots map to folders in ValidatedAestheticsExport.roster (a referee folder is identical
    // regardless of how many slots point at it). Folder→slots, when needed
    // (process once, pack per slot), is derived by grouping the roster.
    pub path: vtree::ScopePath,     // `Players/03 - A`: the scope its issues and events name
    pub player_name: String,        // the folder name without its `NN - ` number; under a roster
                                    //   file, the whole folder name
    pub files: Vec<FileDescriptor>, // names, sizes, kinds — contents load later, per task
    pub links: Vec<SharedLink>,     // shared-folder link files (e.g. "Crocs.boots" → shared Boots/Crocs/);
                                    //   `.common` links (models, material files) are `files` entries
                                    //   of link kind, resolved against the Common folder by the pipeline
    pub ingame_face: Option<FileDescriptor>,  // the folder's ingame_face.toml: its presence
                                            //   marks the in-game face, its keys set the face
    pub fpc: Option<FpcDirective>,  // normalized fpc_on/fpc_off directive
    pub portrait: Option<FileDescriptor>,
    pub settings: Option<FileDescriptor>,  // settings.toml
}
// Every source file has exactly one semantic role. `files` holds model-pipeline
// content — everything categorization routes and the model tasks consume
// (models, textures, material definition files, skeletons), however it reaches
// the output: emitted transformed, converted, or embedded by conversion. Typed
// fields hold savefile-stage inputs (settings, portrait, FPC markers) and
// folder-level references (links). Typed metadata and glTF dependencies are not
// also retained as pass-through output files. Files inside a subfolder (a
// player folder of its own — see "Subfolders") are ordinary `files` entries:
// their ScopePath keeps the subfolder, and categorization reads their roles
// from their names as it reads the root's.

pub struct SharedLink {
    pub kind: SharedKind,           // Face / Boots / Gloves
    pub name: String,               // "Crocs"
}
pub enum SharedKind { Face, Boots, Gloves }
pub enum FpcDirective { On, Off }

pub struct FileDescriptor {
    pub path: vtree::ScopePath,     // canonical virtual path within the (normalized) export
    pub source: vtree::ScopePath,   // the same file's path in the source, for reading it; differs
                                    //   from `path` only under a flattened layer
    pub size: u64,
    pub kind: FileKind,             // classified from the name alone ("Structure pass types")
    // glTF files may reference external .bin buffers and image files (any
    // accepted format: DDS, FTEX, PNG, JPEG, BMP, WebP, TGA, TIFF): those
    // dependencies are restricted to the source model folder, loaded once each,
    // and never doubled as pass-through output. Whether an image is a glTF
    // dependency is only known after deep glTF parsing; until then it is a
    // candidate, and unreferenced candidates fall back to the ordinary allowlist.
    // Material references (FMDL/MTL/materials.toml) name textures by stem; the
    // compiler resolves stems to actual files. Two image files with the same
    // stem but different extensions is a `texture_stem_conflict` error.
}
```

### Design constraints

- **No parent references.** Pass the resolved `ExportIdentity` and other parent context into child
  processing as parameters rather than storing parent references. Copy only presentation strings
  that a child result must own. This avoids Rust's ownership friction with parent-child references.
- **Strong domain types.** Use validated `TeamName`, `TeamId`, `ExportIdentity`, `PlayerSlot`,
  `RefSlot` and `CpkStem` types at boundaries rather than passing unvalidated strings and integers
  through processing code. `KitSlot` is `kit_config`'s (the texture and config names it derives are
  format knowledge); this crate parses kit folder names into it.
- **Shared charges.** Rayon's model-folder tasks are cross-thread by construction; nothing shares
  file bytes (the coordinator reads each task's files as owned `Vec<u8>`), what tasks share is the
  charge — a folder's group, or a `.7z` source's whole decompressed buffer, holds one `Arc<Permit>`
  across its tasks' batches, so shared bytes stay charged to the memory budget until their last
  consumer is written.
- **Enums over trait objects.** Use `enum`s with pattern matching (`FileKind` here, the compiler's
  `PlayerFile`), not `Box<dyn ...>`. Exhaustive matching catches missing cases at compile time.
- **Pass-through files stay simple.** A file the compiler moves without processing stays a
  `FileDescriptor` — no behavior-bearing wrapper even when the file is semantically rich (the
  Collars folder's contents are model files, but the compiler never parses them). Only files with
  processing logic get rich types.

---
