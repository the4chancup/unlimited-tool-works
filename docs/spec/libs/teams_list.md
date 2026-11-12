# Unlimited Tool Works — Library crates plan: teams_list

Part of the [Library crates plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## `libs/teams_list`

The 4cc team identity space and the file that records it, in one **leaf crate with no
dependencies beyond `thiserror`**. Three consumers need it and none is a natural owner: the Team
compiler resolves export names against it, the Save editor writes it from a savefile ("Export
teams list"), and the `studio` binary merges a new release's embedded list into the working copy
after an update. It used to be a module of `aesthetics_export`, which made the Save editor and the
updater depend on the whole export object model for one file format.

- `name.rs` — `TeamName`: the canonical `/xx/` form. `TeamName::new(token)` applies **the** fold
  (trim, strip one leading and one trailing `/`, Unicode `str::to_lowercase`, wrap in slashes;
  an empty token or one still holding `/` or whitespace rejected) — the one place the rule
  lives, so the export-name side and the list side cannot drift. `is_referees()` / `is_balls()` /
  `is_reserved()`. Splitting an export display name into its first token is export-format
  knowledge and stays in `aesthetics_export::identity`, which calls `TeamName::new`.
- `id.rs` — `TeamId`: validated 701–920. Referees' fixed 999 is not a `TeamId` (see the Team
  compiler plan, "Export identity resolution").
- `file.rs` — `TeamsList`: parse / write `teams_list.txt` per the contract in the Team compiler
  plan ("Export identity resolution" and the "`teams_list.txt` contract" open question):
  - tab-split, `ID` and `Name` columns located by header label in any order;
  - every other cell carried verbatim and written back in place;
  - a slash-wrapped Name cell (`/xx/`, the file's own convention) folded through
    `TeamName::new` on load;
  - every other row (`Backup 1`, a bare `Backup`, a half-wrapped `/co`) kept verbatim as an
    inert placeholder whose numeric id still counts in the duplicate check. The wrapping is
    checked on the raw cell because `TeamName::new` itself accepts bare tokens, being the export
    side's fold too;
  - lookup by `TeamName`;
  - the embedded upstream list as a `const`.
- `reconcile.rs` — the merge of an incoming list (embedded upstream, or a savefile's team table)
  into the working list:
  - rows only in the incoming list added;
  - rows only in the working list kept;
  - conflicts (same name, different ID) taken from the incoming list;
  - an incoming team whose ID a placeholder holds taking that slot (a new name replaces the
    placeholder row in place; an existing name keeps its own row and the placeholder row is
    removed);
  - an incoming placeholder carried over verbatim when no working row claims its numeric id and
    no working placeholder shows the same raw `Name` cell.

  Then uniqueness of IDs is validated on the *final* mapping over every row's claimed id,
  placeholders included, so two teams swapping IDs merge cleanly, and the merged file always
  parses again. Every incoming change behind a remaining collision is dropped and the rows
  recomputed from the working list (compute the changes once, then drop and recompute: no undo
  bookkeeping).

  It returns a `MergeSummary` (added / kept / overridden / unresolved / placeholders_added) for
  the caller to show before writing. It never writes: callers own filesystem I/O and the
  read-only handling (`teams_list_read_only`).

Consumers: `aesthetics_export` (identity resolution), `team_compiler` (ID cell write); the
later consumers are in `plans/libs/teams_list.md`. `wasm32`-clean.

The embedded list started as the tab-separated `teams_list.txt` of Red's unreleased development
branch (released Red's list separates its two columns, header `ID Name` included, by a space,
which this reader refuses naming the file; 220 rows, the `Backup N`, `VGL N` and
`Invitational N` placeholders). Its /vg/ League block (781–834) has since been replaced by each
league's own allocation (VGL27's 781–832 at 0.0.2-poc, 833–834 the placeholders `VGL 53` and
`VGL 54`).

The list changes with every allocation, so its tests check only what any version holds (one row
per ID 701–920, in order; an unchanged write-back byte for byte), never which team holds which
ID. The parser's cases (fold, placeholders, carried columns) are tested on lists of their own,
plus reconcile cases for each summary bucket.

---
