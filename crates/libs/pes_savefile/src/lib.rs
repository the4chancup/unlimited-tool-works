//! The PES EDIT savefile (`EDIT00000000`, `EDIT.bin` on PES 15): the per-version container
//! crypto for PES 15-21, the schema-driven player/team/tactics codec over the decrypted
//! payload, `EditFile` (the loaded file that rewrites itself byte-exact), and savefile
//! discovery.
//!
//! Layers, bytes upward: `container` (bytes <-> decrypted sections, knows nothing about
//! players), `schema` + `codec` (sections <-> fields, knows nothing about encryption),
//! `model` (what consumers edit, no I/O and no version), `discovery` (where the file lives),
//! and `file` (`EditFile`, the only module that composes container and codec).

/// Records ↔ fields: the one generic bit codec over a `&VersionSchema`.
pub(crate) mod codec;
/// Bytes ↔ decrypted sections: the PES 16-21 keyed container and the PES 15 LCG one.
pub mod container;
/// Savefile discovery under the user's Documents folder.
pub mod discovery;
/// The whole-file API: `EditFile` composes container and codec.
pub mod file;
/// The fixture payload plumbing shared by `test_support`, `fixture_save` and
/// `pes17_texport`.
#[cfg(any(test, feature = "test-support"))]
mod fixtures;
/// The interchange formats (`Texport` and the legacy ones) read and written.
pub mod interchange;
/// What consumers edit: version-independent player/team/tactics entries, no I/O.
pub mod model;
/// Save-to-save and preset operations over `model/` only, no bytes.
pub mod ops;
/// The Fox player tables (`PlayerAppearance.bin`, `BootsList.bin`,
/// `GloveList.bin`), read and written whole.
pub mod player_tables;
/// Per-version field tables and section layouts, data only, no logic.
pub mod schema;
/// The `PlayerSettings` model behind export `settings.toml` files.
pub mod settings_toml;
/// Fixture helpers shared by the crate's test modules.
#[cfg(test)]
mod test_support;

pub use codec::CodecError;
pub use file::{EditFile, SaveError};
/// The fixture saves and the PES 17 Texport as whole encrypted files, for
/// other crates' tests.
#[cfg(feature = "test-support")]
pub use fixtures::{fixture_save, pes17_texport};
