//! The icon glyphs the shell and the tools draw: sidebar rail entries, the Help and
//! Settings buttons, the status bar's condition and dismiss icons. Each constant is the
//! glyph's one-character string, at its codepoint in `resources/fonts/icons.ttf`, the
//! suite's icon font, which `scripts/make_icons.py` builds from one SVG per icon in
//! `resources/icons/`. A tool's icon says what the tool does, with one mark in its
//! bottom-right corner for the tool's kind: a pencil for an editor, an open box for a
//! compiler, a plus for a creator or a generator.

/// The Team compiler's sidebar glyph: a running figure dropping into an open box.
pub const TEAM_COMPILER: &str = "\u{E68E}";
/// The Save editor's sidebar glyph: a floppy disk with the editors' pencil.
pub const SAVE_EDITOR: &str = "\u{E34C}";
/// The Stadium compiler's sidebar glyph: a row of buildings.
pub const STADIUM_COMPILER: &str = "\u{E102}";
/// The Export upgrader's sidebar glyph: upward arrows on a folder.
pub const EXPORT_UPGRADER: &str = "\u{E52E}";
/// The Music player's sidebar glyph: two music notes.
pub const MUSIC_PLAYER: &str = "\u{E33C}";
/// The Music export editor's sidebar glyph: two music notes with the pencil.
pub const MUSIC_EXPORT_EDITOR: &str = "\u{E340}";
/// The Match tracker's sidebar glyph: a live feed on a monitor.
pub const MATCH_TRACKER: &str = "\u{E1D6}";
/// The Kit config editor's sidebar glyph: a shirt with the pencil.
pub const KIT_CONFIG_EDITOR: &str = "\u{E670}";
/// The Refs arranger's sidebar glyph: a referee's striped shirt with a refresh sign.
pub const REFS_ARRANGER: &str = "\u{E244}";
/// The Balls compiler's sidebar glyph: a ball dropping into the open box.
pub const BALLS_COMPILER: &str = "\u{E716}";
/// The Player aesthetics editor's sidebar glyph: the running figure with the pencil.
pub const PLAYER_AESTHETICS_EDITOR: &str = "\u{E436}";
/// The Team creator's sidebar glyph: three figures with the plus.
pub const TEAM_CREATOR: &str = "\u{E3D6}";
/// The Ruleset editor's sidebar glyph: a clipboard with the pencil.
pub const RULESET_EDITOR: &str = "\u{EB7A}";
/// The DB generator's sidebar glyph: a database with the plus.
pub const DB_GENERATOR: &str = "\u{E1DE}";
/// The sidebar's Settings button: a gear.
pub const GEAR: &str = "\u{E270}";
/// The sidebar's Help button: a question mark in a circle.
pub const QUESTION: &str = "\u{E3E8}";
/// The status bar notice's dismiss button: an X.
pub const X: &str = "\u{E4F6}";
/// The status bar's read-only-data-folder condition: a warning triangle.
pub const WARNING: &str = "\u{E4E0}";
/// The status bar's update-available condition: an arrow down onto a tray.
pub const DOWNLOAD_SIMPLE: &str = "\u{E20C}";
