//! The Studio aesthetics export format as a source-neutral object model: the
//! consumer supplies a canonical listing of one export source (folder, ZIP or
//! 7z — this crate owns no I/O), and the crate parses it into a draft, decides
//! each structural issue's disposition, and projects the sanitized export the
//! consumers compile or repair from.
//!
//! Five consumers (Team compiler, Export upgrader, Kit config editor, Refs
//! arranger, Team Creator) read this crate, so its layout is organized by the
//! progression stage a consumer can stop at.

mod colors_txt;
mod conventions;
mod listing;
mod parse;
mod resolve;
mod slots;
#[cfg(test)]
mod testing;
mod validate;

pub use colors_txt::{ColorLineRefusal, ColorsTxt, RefusedColorLine, read_colors_txt};
pub use conventions::{
    FileKind, KitToken, LegacyKitToken, Marker, MetadataFile, ModelFormat, ModelSuffix, SharedKind,
    classify, common_link_name, common_link_target, ends_with_name, folder_player_name,
    is_os_artifact, is_player_singleton, is_small_metadata, is_txt_note, kit_folder,
    kit_path_token, kit_set_member, kit_token, kit_token_starts, legacy_kit_tokens, model_suffix,
    variant_path, variant_stem, without_kit_token,
};
pub use listing::{CanonicalListing, ListedEntry, ListedKind, SmallMetadata, ValidationContext};
pub use parse::{
    AestheticsExportDraft, ExportCoverage, ExportKind, FileDescriptor, FolderDraft,
    ParsedAestheticsExport, RawRoster, RawRosterEntry, SourceError, coverage, is_name_separator,
    parse_listing, team_name,
};
pub use resolve::{ExportIdentity, IdentityError, ResolvedAestheticsExport};
pub use slots::{PlayerSlot, RefSlot};
pub use validate::{
    ContentFinding, Disposition, FpcDirective, ISSUE_CODES, IssueScope, KitFolder, KitLayout,
    KitTexture, KitTextureSource, KitsFolder, LogoFile, LogoFiles, LogoFit, PlayerFolder,
    PlayerIndex, RootArtifacts, SharedLink, SharedModelFolder, ValidatedAestheticsExport,
    ValidatedRoster, ValidationIssue, ValidationReport, common_link_files,
};
