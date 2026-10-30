//! The Export upgrader's message catalog (`spec/export_upgrader.md` "Message
//! catalog"): each code the command reports, its severity, and the `Message` it
//! is sent as. The report names two exports: the input (export 1) for what is
//! found in it, the output (export 2) for the shared validation's findings on
//! what was written. Each row also carries the member-facing `when`/`effect`
//! the help's generated Messages topic shows; the console prints the code with
//! its context fields.

use std::borrow::Cow;

use aesthetics_export::{IssueScope, ValidationIssue};
use studio_core::{CatalogRow, Disposition, ExportId, Message, MessageCode, Scope, Severity};

use crate::source::SourceFailure;

/// The tool's id: its CLI subcommand and the owner of its message codes.
pub(crate) const TOOL_ID: &str = "export-upgrader";

/// The input export's id in this run's events.
pub(crate) const INPUT: ExportId = ExportId(1);
/// The output export's.
pub(crate) const OUTPUT: ExportId = ExportId(2);

/// The codes the tool reports itself. `team_name_unknown` and
/// `source_read_failed` are shared with the structure pass's `ISSUE_CODES`:
/// the catalog reuses the compiler's codes for them. So they live in
/// `CATALOG`'s issue rows and are emitted by string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Code {
    /// The input holds nothing an old export holds; nothing is written.
    NoOldExport,
    /// A `refs` export without a `refs.txt` holds no roster to migrate:
    /// nothing is written (step 9).
    RefsTxtMissing,
    /// The archive does not open or its listing is refused (the compiler's
    /// code); nothing is written (`path`, `error`).
    ExportExtractFailed,
    /// An entry inside the old format's folders that no step migrates
    /// (step 10), entry by entry; a leftover outside them is
    /// `leftover_dropped`. Its context opens with `reason`, the cause
    /// from `UnmigratedReason`'s closed set, then `other`, the other
    /// candidate's input path, where two candidates claim one place.
    UnmigratedContent,
    /// The last line: nothing above Info, nothing dropped but a leftover.
    MigrationClean,
    /// The last line otherwise; nothing is written ("Verification").
    MigrationNotClean,
    /// Content that never reached a player in game (step 10): outside the
    /// old format's layout — a top-level folder the AET does not define, a
    /// root file other than the format's own, a file under `Other/` — or
    /// inside it — a stray file, an item folder no save record names, a
    /// file directly in `Faces/`, `Boots/` or `Gloves/`. Dropped, and the
    /// run stays clean since the Studio export needs none of it.
    LeftoverDropped,
    /// An old `.mtl` held text between its elements: dropped as the file is
    /// written back, since the compiler's reader refuses it (step 7).
    MtlStrayTextDropped,
    /// A face folder whose slot the save's roster leaves empty: it still
    /// becomes its player folder, with no `settings.toml` (`slot`).
    RosterSlotEmpty,
    /// A path a player folder and a merging item folder share with different
    /// bytes: the item folder is kept shared with a link instead (`path`, the
    /// first clashing path relative to the player folder).
    MergeKeptShared,
    /// Loose mode's assumed wearer of an item folder (step 11): the ID's
    /// position in its kind's old block of 25, reported so the pairing can be
    /// checked (`id`, the folder's four-digit ID; `slot`, the player's).
    WearerAssumed,
    /// A kit slot a Note kit entry gives and no `Kit Configs` config does:
    /// the kit still forms, a placeholder the compiler fills (`slot`).
    KitWithoutConfig,
    /// A kit slot a `Kit Configs` config gives and no Note kit entry does:
    /// its `config.toml` is written all the same, the kit offered as the old
    /// export never offered it (`slot`).
    ConfigWithoutKitEntry,
    /// A kit's config field named another texture: the named texture was
    /// copied into the kit under the field's generic name (`kit`, `stem` the
    /// field's, `from` the source's slot and stem).
    KitTextureCopied,
    /// A texture every kit folder held byte-identically: it moved to
    /// `Kits/all/` (`stem`, the first kit's spelling).
    KitTextureHoisted,
    /// A `Portraits/` file and its slot's face-folder portrait differ in
    /// bytes: the compiler's deep pass would refuse the export (`slot`).
    PortraitConflict,
    /// The old `_r` small logo is a different drawing, kept as
    /// `logo_small.png` (`share`, the differing pixels' percentage).
    LogoSmallCarried,
    /// The old `_r` small logo is a downscale of the large one, dropped:
    /// the compiler regenerates it (`share`).
    LogoSmallDropped,
    /// No usable `_r_ll` member: the largest usable one became `logo.png`.
    LogoFallback,
    /// An `.mtl`/`.fmdl` reference into the team's `Common/` kept as written
    /// but its `???`, respelled the team's ID: the file is not in the
    /// input's `Common/`, a link would clash (the folder already holds a
    /// texture of that stem, which is the one the model will wear — the
    /// compiler looks a game path's file name up in the folder first), or a
    /// subfolder `.mtl` names a Common subfolder (`reference`, the path as
    /// written).
    CommonReferenceKept,
    /// A rostered player's save record wears a custom boots or gloves ID
    /// (above 100) no migrated folder claims: the content is missing (step
    /// 8; `slot`, `name`, `key` the ID's own, `id`). Checked for every
    /// rostered player of a Full output but in a Midcup one only for a
    /// player the output gives a folder.
    CustomModelMissing,
    /// A stored value no `settings.toml` key can hold, left out of the
    /// player's file (`slot`, `name`, `key` the dotted key, `value` the
    /// stored value).
    SettingsValueRefused,
    /// A player's `settings.toml` could not be generated at all (`slot`,
    /// `name`, `error` the failure's text).
    SettingsNotGenerated,
    /// Model-folder mode: where an output folder goes once dropped into a
    /// Studio export (`into`: `Players`, `Boots`, `Gloves`, or `root` for
    /// `Common/`).
    ModelFolderDestination,
    /// A team export's coverage tag, reported on every `upgrade` so the
    /// guess is never silent (`tag`, `source`;
    /// `spec/export_upgrader.md` "Message catalog").
    CoverageTagChosen,
    /// An earlier run's staging folder for this output, removed before the
    /// write (`path`; `spec/export_upgrader.md` "Message catalog").
    StagingFolderRemoved,
}

impl Code {
    /// The code as the console prints it and the help window lists it.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Code::NoOldExport => "no_old_export",
            Code::RefsTxtMissing => "refs_txt_missing",
            Code::ExportExtractFailed => "export_extract_failed",
            Code::UnmigratedContent => "unmigrated_content",
            Code::MigrationClean => "migration_clean",
            Code::MigrationNotClean => "migration_not_clean",
            Code::LeftoverDropped => "leftover_dropped",
            Code::MtlStrayTextDropped => "mtl_stray_text_dropped",
            Code::RosterSlotEmpty => "roster_slot_empty",
            Code::MergeKeptShared => "merge_kept_shared",
            Code::WearerAssumed => "wearer_assumed",
            Code::KitWithoutConfig => "kit_without_config",
            Code::ConfigWithoutKitEntry => "config_without_kit_entry",
            Code::KitTextureCopied => "kit_texture_copied",
            Code::KitTextureHoisted => "kit_texture_hoisted",
            Code::PortraitConflict => "portrait_conflict",
            Code::LogoSmallCarried => "logo_small_carried",
            Code::LogoSmallDropped => "logo_small_dropped",
            Code::LogoFallback => "logo_fallback",
            Code::CommonReferenceKept => "common_reference_kept",
            Code::CustomModelMissing => "custom_model_missing",
            Code::SettingsValueRefused => "settings_value_refused",
            Code::SettingsNotGenerated => "settings_not_generated",
            Code::ModelFolderDestination => "model_folder_destination",
            Code::CoverageTagChosen => "coverage_tag_chosen",
            Code::StagingFolderRemoved => "staging_folder_removed",
        }
    }

    /// Every own code, for the catalog test.
    #[cfg(test)]
    const ALL: &'static [Code] = &[
        Code::NoOldExport,
        Code::RefsTxtMissing,
        Code::ExportExtractFailed,
        Code::UnmigratedContent,
        Code::MigrationClean,
        Code::MigrationNotClean,
        Code::LeftoverDropped,
        Code::MtlStrayTextDropped,
        Code::RosterSlotEmpty,
        Code::MergeKeptShared,
        Code::WearerAssumed,
        Code::KitWithoutConfig,
        Code::ConfigWithoutKitEntry,
        Code::KitTextureCopied,
        Code::KitTextureHoisted,
        Code::PortraitConflict,
        Code::LogoSmallCarried,
        Code::LogoSmallDropped,
        Code::LogoFallback,
        Code::CommonReferenceKept,
        Code::CustomModelMissing,
        Code::SettingsValueRefused,
        Code::SettingsNotGenerated,
        Code::ModelFolderDestination,
        Code::CoverageTagChosen,
        Code::StagingFolderRemoved,
    ];
}

/// The cause an `unmigrated_content` finding names in its `reason`
/// context (`spec/export_upgrader.md` "What it does", step 10): one
/// variant per cause — two sites reporting the same cause
/// share the variant. Where two candidates claim one place the finding
/// also carries `other`, the other candidate's input path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnmigratedReason {
    /// A face folder's name gives no slot 01-23 (`XXX24`, `ab15x`): the
    /// whole folder is refused.
    SlotInvalid,
    /// A second face folder claims a slot already taken (`other` the
    /// first folder's input path).
    SlotTaken,
    /// Two portraits claim one player: a second `Portraits/` file of one
    /// slot, or a second `*portrait.dds` inside one face folder (`other`
    /// the other candidate's input path).
    PortraitTaken,
    /// An item folder's name is no `k`/`g` + four digits (`K0430`,
    /// `k430`): the whole folder is refused.
    IdInvalid,
    /// A second boots or gloves folder carries one ID (`other` the first
    /// folder's input path).
    IdTaken,
    /// Loose mode cannot place the folder's ID: it sits on a spare
    /// position of its kind's inferred block, or past it (step 11).
    LooseUnplaceable,
    /// A `Kit Configs` file's name is no config name
    /// (`???_DEF_<ordinal>_realUni.bin`).
    ConfigNameInvalid,
    /// A `Kit Configs` file sits deeper than the one subfolder the old
    /// layout flattened.
    ConfigTooDeep,
    /// A second config of one kit slot, its bytes different (`other` the
    /// kept config's input path).
    ConfigTaken,
    /// A kit config does not decode (`error` the failure).
    ConfigUndecodable,
    /// A Note player kit entry past the ninth has no Studio kit slot
    /// (`line` the entry).
    KitEntryPastNinth,
    /// A Note color entry's colors do not read, a team-color entry's or
    /// a kit entry's (`line` the entry).
    ColorsUnreadable,
    /// An image file does not decode: a kit `.ftex` that fails its
    /// conversion or a `Logo/` member's PNG (`error` the failure).
    TextureUndecodable,
    /// A second kit texture lands on one output name (`other` the kept
    /// texture's input path).
    TextureNameTaken,
    /// A `Logo/` entry that is no member: a file in a subfolder, or a
    /// name ending in no `_r_ll`/`_r_l`/`_r` role.
    LogoMemberInvalid,
    /// A second `Logo/` member claims one role (`other` the kept
    /// member's input path).
    LogoRoleTaken,
    /// An `.mtl` does not parse even dropping stray text (`error` the
    /// reader's failure).
    MtlUnparsable,
    /// A referee folder's `common/` model no `face.xml` of the referee
    /// names (step 12).
    ModelUnreferenced,
    /// Two locations' files land on one name in a flattened referee
    /// folder, the bytes different (`other` the other file's input path).
    NameTaken,
    /// Two locations' texture files land on one stem in a flattened
    /// referee folder (`other` the other file's input path).
    StemTaken,
    /// A referee folder nests the old layout inside itself
    /// (`face/face/`): the fold would re-create the layout the compiler
    /// refuses.
    NestedLayout,
    /// A referee export's `refs.txt` is not UTF-8: no `players.txt` can
    /// be written.
    RosterNotUtf8,
    /// A second Note-named file sits at the export's root (`other` the
    /// kept Note's path).
    NoteTaken,
    /// No step's folder claims the file: one directly in `Players/` or
    /// below a top-level folder this export kind does not read, or one at
    /// a model-folder input's top level.
    Unclaimed,
    /// Two inputs land on one output path once renamed or respelled
    /// (`other` the kept input's path).
    PathTaken,
    /// Model-folder mode: a top-level folder's name cleans to nothing it
    /// can be published under.
    FolderNameInvalid,
    /// Model-folder mode: a top-level folder holds no file directly in
    /// it (empty, or the converters' two-level layout).
    NoDirectFiles,
    /// Model-folder mode: two top-level folders publish under one name
    /// (`other` the kept folder's input name).
    FolderNameTaken,
}

impl UnmigratedReason {
    /// The `reason` context's value, as the console prints it.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            UnmigratedReason::SlotInvalid => "slot_invalid",
            UnmigratedReason::SlotTaken => "slot_taken",
            UnmigratedReason::PortraitTaken => "portrait_taken",
            UnmigratedReason::IdInvalid => "id_invalid",
            UnmigratedReason::IdTaken => "id_taken",
            UnmigratedReason::LooseUnplaceable => "loose_unplaceable",
            UnmigratedReason::ConfigNameInvalid => "config_name_invalid",
            UnmigratedReason::ConfigTooDeep => "config_too_deep",
            UnmigratedReason::ConfigTaken => "config_taken",
            UnmigratedReason::ConfigUndecodable => "config_undecodable",
            UnmigratedReason::KitEntryPastNinth => "kit_entry_past_ninth",
            UnmigratedReason::ColorsUnreadable => "colors_unreadable",
            UnmigratedReason::TextureUndecodable => "texture_undecodable",
            UnmigratedReason::TextureNameTaken => "texture_name_taken",
            UnmigratedReason::LogoMemberInvalid => "logo_member_invalid",
            UnmigratedReason::LogoRoleTaken => "logo_role_taken",
            UnmigratedReason::MtlUnparsable => "mtl_unparsable",
            UnmigratedReason::ModelUnreferenced => "model_unreferenced",
            UnmigratedReason::NameTaken => "name_taken",
            UnmigratedReason::StemTaken => "stem_taken",
            UnmigratedReason::NestedLayout => "nested_layout",
            UnmigratedReason::RosterNotUtf8 => "roster_not_utf8",
            UnmigratedReason::NoteTaken => "note_taken",
            UnmigratedReason::Unclaimed => "unclaimed",
            UnmigratedReason::PathTaken => "path_taken",
            UnmigratedReason::FolderNameInvalid => "folder_name_invalid",
            UnmigratedReason::NoDirectFiles => "no_direct_files",
            UnmigratedReason::FolderNameTaken => "folder_name_taken",
        }
    }

    /// Every reason, for the uniqueness test.
    #[cfg(test)]
    const ALL: &'static [UnmigratedReason] = &[
        UnmigratedReason::SlotInvalid,
        UnmigratedReason::SlotTaken,
        UnmigratedReason::PortraitTaken,
        UnmigratedReason::IdInvalid,
        UnmigratedReason::IdTaken,
        UnmigratedReason::LooseUnplaceable,
        UnmigratedReason::ConfigNameInvalid,
        UnmigratedReason::ConfigTooDeep,
        UnmigratedReason::ConfigTaken,
        UnmigratedReason::ConfigUndecodable,
        UnmigratedReason::KitEntryPastNinth,
        UnmigratedReason::ColorsUnreadable,
        UnmigratedReason::TextureUndecodable,
        UnmigratedReason::TextureNameTaken,
        UnmigratedReason::LogoMemberInvalid,
        UnmigratedReason::LogoRoleTaken,
        UnmigratedReason::MtlUnparsable,
        UnmigratedReason::ModelUnreferenced,
        UnmigratedReason::NameTaken,
        UnmigratedReason::StemTaken,
        UnmigratedReason::NestedLayout,
        UnmigratedReason::RosterNotUtf8,
        UnmigratedReason::NoteTaken,
        UnmigratedReason::Unclaimed,
        UnmigratedReason::PathTaken,
        UnmigratedReason::FolderNameInvalid,
        UnmigratedReason::NoDirectFiles,
        UnmigratedReason::FolderNameTaken,
    ];
}

/// Every code this tool emits, with its severity and its member-facing
/// `when`/`effect`: the tool's own, then the structure pass's
/// (`aesthetics_export::ISSUE_CODES`) in `ISSUE_CODES` order. A structure-pass
/// row's `when` is the Team compiler's cell word for word (the same condition on
/// the upgraded export), unless this tool's own table describes the row
/// (`team_name_unknown`, `source_read_failed`), and its `effect` the upgrade's.
/// The issue severities are the Team compiler's catalog's, each run-dependent
/// row taken as for a compile that deploys with pass-through and the strict
/// file-type check on (the output's check runs with it), and an E/F row as an
/// Error (`spec/export_upgrader.md` "Message catalog").
const CATALOG: &[(&str, Severity, &str, &str)] = &[
    // The tool's own codes.
    (
        "no_old_export",
        Severity::Error,
        r#"The input holds nothing an old export holds: no Team Note, none of the old format's folders, no `refs.txt` or `refs list.txt`."#,
        r#"Nothing is written."#,
    ),
    (
        "refs_txt_missing",
        Severity::Error,
        r#"A `refs` export has neither `refs.txt` nor `refs list.txt`, so it holds no roster to migrate."#,
        r#"Nothing is written."#,
    ),
    (
        "export_extract_failed",
        Severity::Error,
        r#"The archive (`.zip` or `.7z`) cannot be opened, or a path inside it is unusable: it leads outside the archive, or two names differ only in letter case or Unicode form."#,
        r#"Nothing is written. Repack the archive, or unpack it into a folder first."#,
    ),
    (
        "unmigrated_content",
        Severity::Error,
        r#"Content of the input that reached the game, or was clearly meant for it, but that the migration does not carry into the output: two files or folders claiming one place (a second face folder of one slot, two portraits of one), a whole folder under a name the old format refuses, a Team Note entry whose colors do not read, a kit config that does not decode. The line names the cause, and, where another file or folder took the place, that one's path."#,
        r#"It is not migrated, and the run is not clean, so nothing is written. Fix it in the input and upgrade again."#,
    ),
    (
        "migration_clean",
        Severity::Info,
        r#"The report's last line, when there was no Error finding and no finding dropped anything but a leftover."#,
        r#"The upgraded export is written. The exit code is 0."#,
    ),
    (
        "migration_not_clean",
        Severity::Info,
        r#"The report's last line, otherwise."#,
        r#"Nothing is written. The exit code is 1 after an Error finding, or after a finding that dropped more than a leftover."#,
    ),
    (
        "leftover_dropped",
        Severity::Warning,
        r#"Content that never reached a player in game, outside the old format's layout or inside it (a refused portrait, a stray file, a second goalkeeper kit, a file directly in `Faces/`, `Boots/` or `Gloves/`). For a dropped part of a Team Note entry, the line names the entry's line in the Note."#,
        r#"It is not written, but the run stays clean."#,
    ),
    (
        "mtl_stray_text_dropped",
        Severity::Info,
        r#"An old `.mtl` held text between its elements, which the compiler cannot read."#,
        r#"The text is dropped as the file is written back."#,
    ),
    (
        "roster_slot_empty",
        Severity::Warning,
        r#"A face folder's slot is empty in the save's roster (the line names the slot)."#,
        r#"It still becomes its player folder, with no `settings.toml`."#,
    ),
    (
        "merge_kept_shared",
        Severity::Info,
        r#"A player folder and a boots or gloves folder that would merge into it share a file path with different bytes."#,
        r#"The boots or gloves folder is kept shared with a link instead."#,
    ),
    (
        "wearer_assumed",
        Severity::Info,
        r#"`--loose` paired a boots or gloves folder with the player at its ID's position in the team's old block of 25 (the line names the folder's four-digit ID and the slot)."#,
        r#"The pairing is reported so it can be checked. If it is wrong, rename the folder, or upgrade with `--savefile`."#,
    ),
    (
        "kit_without_config",
        Severity::Warning,
        r#"A Team Note kit entry gives a slot no `Kit Configs` config does."#,
        r#"The kit still forms. In a `Full` export it is a placeholder the compiler fills, and in a `Midcup` one it updates only the installed kit's colors."#,
    ),
    (
        "config_without_kit_entry",
        Severity::Warning,
        r#"A `Kit Configs` config gives a slot no Team Note kit entry does."#,
        r#"Its `config.toml` is written all the same, so the kit is offered as the old export never offered it. Delete the kit's folder in the upgraded export if the kit should not be offered."#,
    ),
    (
        "kit_texture_copied",
        Severity::Info,
        r#"A kit's config names another texture for a number or name (the line names the kit, the stem and where it came from)."#,
        r#"The named texture was copied into the kit under the stem the line names."#,
    ),
    (
        "kit_texture_hoisted",
        Severity::Info,
        r#"A texture every kit folder held with identical bytes."#,
        r#"It was moved to `Kits/all/`, and the kits compile the same without the copies."#,
    ),
    (
        "portrait_conflict",
        Severity::Error,
        r#"A `Portraits/` file and its slot's face-folder portrait differ in bytes, which the compiler would refuse."#,
        r#"The run is not clean, so nothing is written. Keep one portrait in the old export and upgrade again."#,
    ),
    (
        "logo_small_carried",
        Severity::Info,
        r#"The old `_r` small logo is a different drawing (the line gives how much it differs)."#,
        r#"It is kept as `logo_small.png`."#,
    ),
    (
        "logo_small_dropped",
        Severity::Info,
        r#"The old `_r` small logo is a downscale of the large one."#,
        r#"It is dropped, because the compiler regenerates it. Carry it by hand as `logo_small.png` if it was a variant."#,
    ),
    (
        "logo_fallback",
        Severity::Warning,
        r#"The `Logo/` folder holds no usable `_r_ll` file."#,
        r#"The largest usable image became `logo.png`, and no `logo_small.png` is written."#,
    ),
    (
        "common_reference_kept",
        Severity::Warning,
        r#"An `.mtl` or `.fmdl` reference into the team's `Common/` whose file the input's `Common/` does not hold, or which no link can spell (the model's folder already holds a texture or link of that stem, or a `.mtl` in a subfolder names a Common subfolder)."#,
        r#"It is kept as written, but its three-character Common segment is respelled to the team's ID. An installed CPK may still supply the file."#,
    ),
    (
        "custom_model_missing",
        Severity::Error,
        r#"In the save, a rostered player wears a custom boots or gloves ID (above 100) that no migrated folder claims. Every rostered player of a `Full` output is checked, but in a `Midcup` output only the players it gives a folder."#,
        r#"The content is missing. The run is not clean, so nothing is written. Add the folder to the old export, or take the ID off the player in the save."#,
    ),
    (
        "settings_value_refused",
        Severity::Warning,
        r#"A stored value that no key of the player's `settings.toml` or `ingame_face.toml` can hold (the line names the slot, the player, the key and the value)."#,
        r#"It is left out of the file the key belongs to."#,
    ),
    (
        "settings_not_generated",
        Severity::Error,
        r#"A player's `settings.toml` could not be generated at all (the line names the error)."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "model_folder_destination",
        Severity::Info,
        r#"In `upgrade-models`, where an output folder goes once dropped into an export (`Players`, `Boots`, `Gloves`, or the export root for `Common/`)."#,
        r#"Nothing to do."#,
    ),
    (
        "coverage_tag_chosen",
        Severity::Info,
        r#"The coverage tag the output's name took, and what decided it (the line names them): `flag` for `--coverage`, `name` for a tag word the name already carried, `midcup` or `additions` for text the name holds, `default` when it holds neither."#,
        r#"Nothing to do. Check the tag is what the export should be, and pass `--coverage` if it is not."#,
    ),
    (
        "staging_folder_removed",
        Severity::Info,
        r#"An earlier run's incomplete output folder, the output's name with `.upgrading` appended, was still there when this run started (the line names it)."#,
        r#"It was removed before this run wrote its output."#,
    ),
    // The structure pass's codes, in `ISSUE_CODES` order.
    (
        "nested_folders_fixed",
        Severity::Warning,
        r#"The export's content sits one level too deep, inside a single folder."#,
        r#"It is reported on the upgraded export, which is still written unless the finding drops part of it."#,
    ),
    (
        "nested_root_ambiguous",
        Severity::Error,
        r#"Several folders inside the export each look like its root."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "nested_root_conflict",
        Severity::Error,
        r#"A file at the export's root and one inside the nested folder resolve to the same place."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "players_txt_invalid",
        Severity::Error,
        r#"`players.txt` cannot be read as a roster (an encoding failure or a roster-wide ambiguity), or a referee roster is left with no valid assignment."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "refs_txt_ignored",
        Severity::Warning,
        r#"A referee export holds both `players.txt` and the old `refs.txt` name."#,
        r#"It is reported on the upgraded export, which is still written unless the finding drops part of it."#,
    ),
    (
        "source_read_failed",
        Severity::Error,
        r#"A file of the input could not be read after its listing did."#,
        r#"It is not migrated, and the run is not clean, so nothing is written. An unreadable Team Note (it decides the team) or a referee export's `refs.txt` or `refs list.txt` (it decides the roster) stops the run at once."#,
    ),
    (
        "export_empty",
        Severity::Error,
        r#"No usable content was found at the export's root."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "team_name_unknown",
        Severity::Error,
        r#"The export's team is not in the teams list: the Team Note's `Team:` line when there is one, else the export's name (the line names the value)."#,
        r#"Nothing is written. Check the name, or add the team to the list."#,
    ),
    (
        "export_tag_missing",
        Severity::Error,
        r#"A team export's name has neither `Full` nor `Midcup` as its second word."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "export_layout_old",
        Severity::Error,
        r#"The export is in the old layout, which the compiler does not read: a root folder named `Kit Configs`, `Kit Textures` or `Other` (kits live in `Kits/`)."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "players_txt_missing",
        Severity::Error,
        r#"A referee export has no `players.txt` (nor the old `refs.txt` name)."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "players_txt_line_invalid",
        Severity::Error,
        r#"A nonblank line of `players.txt` does not give a slot number and a folder name."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "players_txt_slot_invalid",
        Severity::Error,
        r#"A `players.txt` line's slot is outside 01-23 (teams) or 01-35 (refs)."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "players_txt_slot_duplicate",
        Severity::Error,
        r#"`players.txt` assigns the same slot more than once, so the roster is ambiguous."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "players_txt_target_missing",
        Severity::Error,
        r#"A `players.txt` line names a folder that does not exist."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "player_unlisted",
        Severity::Warning,
        r#"`players.txt` is present but does not list this player folder, whatever number its name carries."#,
        r#"It is reported on the upgraded export, which is still written unless the finding drops part of it."#,
    ),
    (
        "player_folder_number_invalid",
        Severity::Error,
        r#"Without `players.txt`, a player folder's number is not in 01-23."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "player_number_duplicate",
        Severity::Error,
        r#"Without `players.txt`, two player folder names claim the same slot."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "file_type_disallowed",
        Severity::Error,
        r#"A file sits where its kind is not allowed, anywhere in a player or shared folder, in a kit folder, or in `Collars/`: its extension is disallowed, or, below a subfolder, it is a marker, a `settings.toml` or a per-player file."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "common_file_disallowed",
        Severity::Error,
        r#"A file sits in `Common/` that is not model content (a model, `.mtl`, `.skl`, `.fclo`, `.xml`, `.bin`, texture or `.txt` note would be)."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "player_layout_proto",
        Severity::Error,
        r#"A referee export's player folder has a `face/`, `boots/`, `gloves/` or `common/` subfolder directly inside, the layout of older referee exports (in a team export those are ordinary subfolders)."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "shared_link_duplicate",
        Severity::Error,
        r#"A player folder has more than one shared link for a part (face, boots or gloves), counted over the whole folder, subfolders included. Two links naming one folder count once."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "link_target_missing",
        Severity::Error,
        r#"A link file names a shared folder that does not exist."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "common_link_missing",
        Severity::Error,
        r#"A `.common` link names a file missing from `Common/` (a `kitN` name or folder stands for its set). In PES 2018 to 2021, a texture link is satisfied when a CPK listed before this run's own holds it at the team's Common path."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "link_target_dropped",
        Severity::Error,
        r#"A shared-folder or `.common` link names a target that was left out for its own finding (the line names the link, the target and the finding)."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "shared_folder_orphaned",
        Severity::Warning,
        r#"A shared `Faces/`, `Boots/` or `Gloves/` folder is linked by no player."#,
        r#"It is reported on the upgraded export, which is still written unless the finding drops part of it."#,
    ),
    (
        "fpc_conflict",
        Severity::Error,
        r#"A player folder holds both `fpc_on` and `fpc_off`."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "ingame_face_marker_renamed",
        Severity::Error,
        r#"A player folder holds `ingame_face` or `ingame_face.txt`, the in-game face marker's old name. The marker is now `ingame_face.toml`, which also holds the in-game face's keys."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "ingame_face_explicit_face_model",
        Severity::Error,
        r#"A player folder holds `ingame_face.toml` beside an explicit face model (`face_high`, `hair_high` or `oral`, in any format), a `.common` link to one, or a `Faces/` link, and any of them needs the face folder the marker suppresses."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "texture_stem_conflict",
        Severity::Error,
        r#"Two image files share one stem in the same place, whatever their extensions (`hair.dds` beside `hair.png`, or a texture link `hair.png.common`; `hair.dds` beside `jessie/hair.dds` is no conflict). The place is a model folder, a kit folder, `Kits/all/`, a folder of `Common/`, or `Portraits/` (two files of one slot)."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "fmdl_name_invalid",
        Severity::Error,
        r#"In PES 2018 to 2021, a model anywhere in a shared `Boots/` or `Gloves/` folder, or one a `.common` link there stands for, has a name that does not resolve to that category's allowed names."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "model_name_invalid",
        Severity::Error,
        r#"In PES 2015 to 2017, a model anywhere in a shared `Boots/` or `Gloves/` folder lacks that category's suffix."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "edithair_unsupported",
        Severity::Error,
        r#"In PES 2015 to 2017, a `face_edithair.xml` or `hair.xml` sits anywhere in a player or shared folder."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "kit_folder_invalid",
        Severity::Error,
        r#"A `Kits/` subfolder's name is not `<slot>` or `<slot> - <label>` with the slot in `p1`-`p9`, `g1` or `all`."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "kit_slot_duplicate",
        Severity::Error,
        r#"Two `Kits/` subfolders resolve to the same slot (`p1/` and `p1 - Lakers/`)."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "kit_texture_name_invalid",
        Severity::Error,
        r#"A kit texture's name does not carry the `kit` prefix (`kit.dds`, `kit_mask.dds`)."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "kit_layout_conflict",
        Severity::Error,
        r#"A kit folder holds both `pre-fox` and `fox` markers."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "kit_icon_invalid",
        Severity::Warning,
        r#"A kit's `icon_<N>` marker is numbered above 23, or the kit holds two or more (each is reported)."#,
        r#"It is reported on the upgraded export, which is still written unless the finding drops part of it."#,
    ),
    (
        "kit_all_file_ignored",
        Severity::Warning,
        r#"`Kits/all/` holds something other than kit textures (`config.toml`, `colors.txt`, an `icon_<N>` marker, anything else)."#,
        r#"It is reported on the upgraded export, which is still written unless the finding drops part of it."#,
    ),
    (
        "kit_all_unused",
        Severity::Warning,
        r#"`Kits/all/` is present but no kit folder inherits from it."#,
        r#"It is reported on the upgraded export, which is still written unless the finding drops part of it."#,
    ),
    (
        "kit_textures_inherited",
        Severity::Info,
        r#"A kit lacks texture names that `Kits/all/` provides (the line lists them)."#,
        r#"It is reported on the upgraded export, which is still written unless the finding drops part of it."#,
    ),
    (
        "portrait_name_invalid",
        Severity::Error,
        r#"A `Portraits/` file is not a texture named `NN` with NN in 01-23 (the `NN` alone, or followed by ` - ` and any text)."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "logo_file_invalid",
        Severity::Error,
        r#"A root `logo*` file has an unknown tag in its name, is in a format the compiler does not read, or does not decode."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "logo_role_duplicate",
        Severity::Error,
        r#"Two files claim the same logo role (two `logo*`, or two `logo_small*`)."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "logo_small_without_main",
        Severity::Error,
        r#"A `logo_small*` image is present with no main `logo*`."#,
        r#"The run is not clean, so nothing is written."#,
    ),
    (
        "root_file_unexpected",
        Severity::Warning,
        r#"An unknown file or folder sits at the export's root (a stale `wrapper/` beside a usable root included), or a file sits directly inside `Players/`, `Kits/`, `Faces/`, `Boots/` or `Gloves/`, all of which hold only folders."#,
        r#"It is reported on the upgraded export, which is still written unless the finding drops part of it."#,
    ),
    (
        "notes_found",
        Severity::Info,
        r#"The export's root holds a non-empty `notes.txt`."#,
        r#"It is reported on the upgraded export, which is still written unless the finding drops part of it."#,
    ),
    (
        "notes_encoding_invalid",
        Severity::Error,
        r#"The root `notes.txt` is not valid UTF-8."#,
        r#"The run is not clean, so nothing is written."#,
    ),
];

/// The catalog rows the help's generated Messages topic is rendered from, in `CATALOG`
/// order (`lib.rs` `help`).
pub(crate) fn catalog() -> Vec<CatalogRow> {
    CATALOG
        .iter()
        .map(|row| CatalogRow {
            code: row.0,
            severity: row.1.name(),
            when: row.2,
            effect: row.3,
        })
        .collect()
}

/// The severity `code` is shown at.
fn severity(code: &str) -> Severity {
    CATALOG
        .iter()
        .find(|row| row.0 == code)
        .map(|row| row.1)
        .expect(
            "every code has a catalog row: the catalog test checks `Code::ALL` and `ISSUE_CODES`",
        )
}

/// The message for `code` with its scope, disposition and context fields.
fn message(
    code: &'static str,
    scope: Scope,
    disposition: Disposition,
    context: Vec<(&'static str, String)>,
) -> Message {
    Message {
        code: MessageCode {
            tool_id: TOOL_ID,
            code: Cow::Borrowed(code),
        },
        severity: severity(code),
        disposition,
        scope,
        context: context
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    }
}

/// A message for one of the tool's own codes.
pub(crate) fn tool_message(
    code: Code,
    scope: Scope,
    disposition: Disposition,
    context: Vec<(&'static str, String)>,
) -> Message {
    message(code.as_str(), scope, disposition, context)
}

/// `unmigrated_content` on `scope` with `disposition` (step 10): `reason`
/// names the cause first in the context; `other`, the other candidate's
/// input path, follows where two candidates claim one place; the site's
/// own context (`line`, `error`) last.
pub(crate) fn unmigrated(
    scope: Scope,
    disposition: Disposition,
    reason: UnmigratedReason,
    other: Option<String>,
    context: Vec<(&'static str, String)>,
) -> Message {
    let mut context = context;
    if let Some(other) = other {
        context.insert(0, ("other", other));
    }
    context.insert(0, ("reason", reason.as_str().to_owned()));
    tool_message(Code::UnmigratedContent, scope, disposition, context)
}

/// `export_extract_failed` on `export_id` for `failure`: the archive does not
/// open or its listing is refused, nothing is written (`path`, `error`).
pub(crate) fn extract_failed(export_id: ExportId, failure: SourceFailure) -> Message {
    tool_message(
        Code::ExportExtractFailed,
        Scope::Export { export_id },
        Disposition::DropExport,
        vec![("path", failure.path), ("error", failure.error)],
    )
}

/// `source_read_failed`: the file at `failure.path` did not read after the
/// listing did, `disposition` what becomes of it (`path`, `error`). A shared
/// code — one of `ISSUE_CODES` — so it has no `Code` variant and takes the
/// catalog's row.
pub(crate) fn read_failed(
    export_id: ExportId,
    failure: SourceFailure,
    disposition: Disposition,
) -> Message {
    let scope = Scope::File {
        export_id,
        path: vtree::ScopePath::new(&failure.path)
            .expect("a source path was checked when the export opened"),
    };
    message(
        "source_read_failed",
        scope,
        disposition,
        vec![("path", failure.path), ("error", failure.error)],
    )
}

/// `team_name_unknown`: the team resolves to no row of the list, the `Team:`
/// line's file or the export named (`name`, the value as written). A shared
/// code — one of `ISSUE_CODES` — so it has no `Code` variant and takes the
/// catalog's row.
pub(crate) fn team_name_unknown(scope: Scope, name: String) -> Message {
    message(
        "team_name_unknown",
        scope,
        Disposition::DropExport,
        vec![("name", name)],
    )
}

/// The message for one shared-validation issue of the output export
/// `export_id`: the issue's scope placed in that export, its effective
/// disposition, and its severity from the catalog.
pub(crate) fn issue_message(issue: &ValidationIssue, export_id: ExportId) -> Message {
    let scope = match &issue.scope {
        IssueScope::Export => Scope::Export { export_id },
        IssueScope::Folder(path) => Scope::Folder {
            export_id,
            path: path.clone(),
        },
        IssueScope::File(path) => Scope::File {
            export_id,
            path: path.clone(),
        },
        IssueScope::RosterEntry { file, line, slot } => Scope::RosterEntry {
            export_id,
            file: file.clone(),
            line: *line,
            slot: *slot,
        },
    };
    let disposition = match issue.disposition {
        aesthetics_export::Disposition::Keep => Disposition::Keep,
        aesthetics_export::Disposition::DropFile => Disposition::DropFile,
        aesthetics_export::Disposition::DropSlot => Disposition::DropSlot,
        aesthetics_export::Disposition::DropFolder => Disposition::DropFolder,
        aesthetics_export::Disposition::DropExport => Disposition::DropExport,
    };
    let context: Vec<(&'static str, String)> = issue
        .context
        .iter()
        .map(|(key, value)| (*key, value.clone()))
        .collect();
    message(issue.code, scope, disposition, context)
}

#[cfg(test)]
mod tests {
    use aesthetics_export::ISSUE_CODES;

    use super::*;

    #[test]
    fn every_issue_code_and_every_own_code_has_exactly_one_catalog_row() {
        for code in ISSUE_CODES {
            let rows = CATALOG.iter().filter(|(known, ..)| known == code).count();
            assert_eq!(rows, 1, "{code}");
        }
        for code in Code::ALL {
            let rows = CATALOG
                .iter()
                .filter(|(known, ..)| *known == code.as_str())
                .count();
            assert_eq!(rows, 1, "{}", code.as_str());
        }
        for (index, (code, ..)) in CATALOG.iter().enumerate() {
            assert!(
                CATALOG[index + 1..].iter().all(|(other, ..)| other != code),
                "duplicate row: {code}"
            );
        }
    }

    #[test]
    fn each_code_has_the_catalogs_severity() {
        for (code, expected) in [
            ("migration_clean", Severity::Info),
            ("migration_not_clean", Severity::Info),
            ("leftover_dropped", Severity::Warning),
            ("mtl_stray_text_dropped", Severity::Info),
            ("unmigrated_content", Severity::Error),
            ("no_old_export", Severity::Error),
            ("export_extract_failed", Severity::Error),
            ("root_file_unexpected", Severity::Warning),
            ("file_type_disallowed", Severity::Error),
            ("common_file_disallowed", Severity::Error),
            ("export_empty", Severity::Error),
            ("team_name_unknown", Severity::Error),
            ("nested_folders_fixed", Severity::Warning),
            ("kit_textures_inherited", Severity::Info),
        ] {
            assert_eq!(severity(code), expected, "{code}");
        }
    }

    #[test]
    fn no_own_code_is_also_an_issue_code() {
        for code in Code::ALL {
            assert!(!ISSUE_CODES.contains(&code.as_str()), "{}", code.as_str());
        }
    }

    #[test]
    fn read_failed_is_a_file_scoped_error_naming_the_path_and_reason() {
        let failure = crate::source::SourceFailure {
            path: "co Note.txt".to_owned(),
            error: "denied".to_owned(),
        };
        let message = read_failed(ExportId(1), failure, Disposition::DropExport);
        assert_eq!(message.code.tool_id, "export-upgrader");
        assert_eq!(message.code.code, "source_read_failed");
        assert_eq!(message.severity, Severity::Error);
        assert_eq!(message.disposition, Disposition::DropExport);
        assert_eq!(
            message.scope,
            Scope::File {
                export_id: ExportId(1),
                path: vtree::ScopePath::new("co Note.txt").unwrap()
            }
        );
        assert_eq!(
            message.context,
            vec![
                ("path".to_owned(), "co Note.txt".to_owned()),
                ("error".to_owned(), "denied".to_owned())
            ]
        );
    }

    #[test]
    fn a_message_carries_its_code_severity_scope_and_context() {
        let message = unmigrated(
            Scope::File {
                export_id: ExportId(1),
                path: vtree::ScopePath::new("Other/readme.txt").unwrap(),
            },
            Disposition::DropFile,
            UnmigratedReason::Unclaimed,
            None,
            vec![],
        );
        assert_eq!(message.code.tool_id, "export-upgrader");
        assert_eq!(message.code.code, "unmigrated_content");
        assert_eq!(message.severity, Severity::Error);
    }

    #[test]
    fn unmigrated_places_reason_then_other_then_the_sites_fields() {
        let message = unmigrated(
            Scope::File {
                export_id: ExportId(1),
                path: vtree::ScopePath::new("Kit Configs/714/XXX_DEF_1st_realUni.bin").unwrap(),
            },
            Disposition::DropFile,
            UnmigratedReason::ConfigTaken,
            Some("Kit Configs/XXX_DEF_1st_realUni.bin".to_owned()),
            vec![("error", "denied".to_owned())],
        );
        assert_eq!(
            message.context,
            vec![
                ("reason".to_owned(), "config_taken".to_owned()),
                (
                    "other".to_owned(),
                    "Kit Configs/XXX_DEF_1st_realUni.bin".to_owned()
                ),
                ("error".to_owned(), "denied".to_owned()),
            ]
        );
        // Without a second candidate `other` is absent, not empty.
        let message = unmigrated(
            Scope::Run,
            Disposition::DropFile,
            UnmigratedReason::ColorsUnreadable,
            None,
            vec![("line", "- 2nd: blue".to_owned())],
        );
        assert_eq!(
            message.context,
            vec![
                ("reason".to_owned(), "colors_unreadable".to_owned()),
                ("line".to_owned(), "- 2nd: blue".to_owned()),
            ]
        );
    }

    #[test]
    fn every_reason_names_one_cause_in_snake_case() {
        let mut seen = std::collections::BTreeSet::new();
        for reason in UnmigratedReason::ALL {
            let value = reason.as_str();
            assert!(
                !value.is_empty()
                    && value
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{value} is not snake_case"
            );
            assert!(seen.insert(value), "duplicate reason value: {value}");
        }
    }

    #[test]
    fn every_catalog_row_carries_its_member_facing_text() {
        for (code, _, when, effect) in CATALOG {
            assert!(!when.is_empty() && !effect.is_empty(), "{code}");
        }
    }

    #[test]
    fn the_help_chapter_lists_the_whole_catalog_in_order() {
        use studio_core::StudioTool;

        let help = crate::Tool::new().help();
        let codes: Vec<&str> = help.messages.iter().map(|row| row.code).collect();
        let catalog: Vec<&str> = CATALOG.iter().map(|row| row.0).collect();
        assert_eq!(codes, catalog);
    }

    #[test]
    fn a_rows_text_is_written_for_a_member() {
        let row = CATALOG.iter().find(|row| row.0 == "no_old_export").unwrap();
        assert_eq!(
            row.2,
            "The input holds nothing an old export holds: no Team Note, none of the old format's folders, no `refs.txt` or `refs list.txt`."
        );
        assert_eq!(row.3, "Nothing is written.");
        // A structure-pass row takes the Team compiler's `when` cell word for word
        // (pinned here: the same text as `team_compiler`'s catalog row of the code) and
        // the upgrader's own `effect`, unless this tool's own table describes the row
        // (`team_name_unknown`, `source_read_failed`).
        let row = CATALOG
            .iter()
            .find(|row| row.0 == "nested_folders_fixed")
            .unwrap();
        assert_eq!(
            row.2,
            "The export's content sits one level too deep, inside a single folder."
        );
        assert_eq!(
            row.3,
            "It is reported on the upgraded export, which is still written unless the finding drops part of it."
        );
        let row = CATALOG
            .iter()
            .find(|row| row.0 == "team_name_unknown")
            .unwrap();
        assert_eq!(
            row.3,
            "Nothing is written. Check the name, or add the team to the list."
        );
        // `file_type_disallowed` takes the compiler's `when` too; the effect is the
        // run's verdict, the strict check the upgrade's validation runs making it an Error.
        let row = CATALOG
            .iter()
            .find(|row| row.0 == "file_type_disallowed")
            .unwrap();
        assert_eq!(
            row.2,
            "A file sits where its kind is not allowed, anywhere in a player or shared folder, in a kit folder, or in `Collars/`: its extension is disallowed, or, below a subfolder, it is a marker, a `settings.toml` or a per-player file."
        );
        assert_eq!(row.3, "The run is not clean, so nothing is written.");
    }
}
