# Unlimited Tool Works — Match tracker plan: Match files

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

Part of the [Match tracker plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Match files

The binary `.sen` format is dropped entirely — its version-locked layout and
dual-direction template reader were most of the old file-handling code, and existing
archives remain readable in old SEN:P-AI. Matches save as **plain JSON**
(`.match.json`):

- A separately versioned match-file envelope:
  `"match_schema": {"major": 1, "minor": 0}`, plus `producer_version`,
  `game_version`, `game_profile_id`, `match_id`, lifecycle state, and
  `history_complete`. In a file whose match has not reached `Live`, a true
  `history_complete` value is provisional ("not yet disproven"), not confirmed complete;
  readers and exporters interpret it together with `has_reached_live`. Feed-wire and
  match-file schemas use separate version spaces.
  Major versions are compatibility boundaries. Minor versions may add only optional or
  defaulted fields whose omission does not change existing semantics; adding or changing
  a state-affecting enum variant is a major-version change unless an explicit forward-
  compatible opaque representation is defined. Readers must not silently skip unknown
  state-changing variants.
- Match info: team names and IDs, full rosters, and the current `Score`, which is frozen
  when lifecycle state is `Finalized` but is not necessarily fully classified. The
  regulation and shootout components are authoritative for goals already assigned to
  those domains. When `classification_complete` is false, the overall result is incomplete
  because additional goals remain unclassified; the score must not be used as a final
  result or canonical footballbox score until classification is complete. The file also
  carries detected match length and optional provenance-tagged timestamps:
  `created_at`, `first_observed_at`, optional `kickoff_at` when kickoff was actually
  observed, and optional `finalized_at`. Attaching mid-match must not label
  `first_observed_at` as kickoff. Match state also stores `has_reached_live`, the latest
  optional `Clock`, optional `ClockMotion` including `stop_epoch`, and the saved
  `score_authority`. Recovery files additionally store the feed-binding state needed to
  determine whether the saved `MatchSnapshot` is eligible for `restore_retained`.
  Ordinary read-only Open treats recorded authority as historical data and never installs
  feed authority.
- The event list and the clock-timeline entries they reference, plus a distinct
  `recovered_aggregate` section for initial counters whose order and minutes are unknown.
  Canonical `unclassified_goals` records preserve every unresolved goal's stable match-
  scoped ID, scored/own-goal attribution, optional original clock, observed segment
  evidence, optional observed ordering, and `Pending`/`Permanent` state across save/restore.
  The persisted `next_unclassified_goal_id` cursor prevents reuse after resolved records
  leave this collection. A classified goal is stored once as its
  canonical event with the final known assist. Feed-level
  `GoalAmended` messages are not separate scoring events in this timeline; an optional
  audit log may retain an amendment and its `goal_sequence` reference. Exporters always
  materialize one goal.
- Latest stats and the full per-player stat-change history; the latest stats are final
  only for a finalized match. Stat records use stable catalog IDs plus display names,
  value type, and segment count; names alone are not schema keys. `catalog_id` is unique
  within a verified Unlimited Tool Works `game_profile_id`. Fork array positions and `visualIndex`
  values are retained only as optional provenance metadata and are never used directly as
  schema keys. Unidentified scalar fields retain their stable catalog IDs and are stored
  with field class `UnknownScalar` — their history is the raw material for identifying
  them, one of the reasons `.sen` files were kept at all. Stat records preserve the profile
  field class separately from their binary value type and segment count. Grid/blob fields
  follow the separate latest-grid rules below instead of entering generic stat history.
- **Candidate heatmap grids**: the grid layout claims and their unverified status are in
  `reading_the_game.md` "Game structures". Once a grid signature and its semantics are verified, its
  latest per-player/team snapshot is updated in memory at a throttled cadence and written
  on save. Profiles mark whether a grid is cumulative or instantaneous; the viewer labels
  and interprets it accordingly. A time series is not stored unless a later
  schema adds an explicitly downsampled optional timeline; grid changes are not duplicated
  into generic scalar stat history. Grids remain a separate optional section so match
  files from versions without a verified heatmap signature stay valid; the PES17
  verification task below determines whether that grid can be enabled.

Autosave carries over: on finalization and/or on a timer, with `%HOME`/`%AWAY` and
strftime placeholders in the filename pattern. Saves use write-temp → flush → atomic
replace. A suspension creates at most one separate recovery file per suspension episode
and never overwrites a user save. A timed interval resets only after a successful write;
filename collisions receive a deterministic suffix.

On startup, the tracker scans the autosave directory for unclosed recovery files and
offers **Restore**, **Open read-only**, or **Discard**. An `offline_only` draft is instead
offered **Open offline**, **Finalize offline** (available only if the draft reached
`Live`), or **Discard**, but never feed **Restore**;
it cannot call `restore_retained` or publish later events under the released `match_id`.
Its saved reliable cursor is audit metadata only. Recovery files additionally carry
`reliable_through_sequence`, the last reliable feed sequence reflected in the saved
canonical state; eligible feed restoration uses it to continue sequence allocation
strictly above the saved cursor. Restore preserves the saved `match_id`,
`has_reached_live`, `score_authority`, latest clock, `history_complete` flag, feed-binding
state, `unclassified_goals` records with their states, and
`next_unclassified_goal_id`. It normally preserves lifecycle and clock motion as well,
except that a saved `Live` draft is first materialized as a
recovery revision with `LifecycleState::Suspended { reason: SourceLoss }` and
`clock_motion = Some(ClockMotion::Unknown)` unless continuity can still be proved.
`restore_retained` validates
and installs that exact normalized `MatchSnapshot` with the saved cursor through the atomic
operation described above. It does not resume automation until live memory is reacquired
and a `MatchResumed` snapshot has reconciled the restored state; a provisional draft that
has never reached live uses `MatchLive` instead.

Finalization is staged for every path — explicit Finish or a verified session boundary. A
feed-bound finalization builds its prospective `Finalized` revision and takes the
complete broker batch (including replacement `TeamsDetected` for a session boundary)
through the staged prepare/reserve → durable write → infallible commit pattern defined
under `match_feed.md` "The match feed". During the durable-write step, if no current user save or recovery file
exists, the tracker atomically
creates a finalized safety/recovery file even when **Autosave on match end** is off.
An offline draft instead commits its finalized state locally without
feed publication. Finalization never removes the last durable copy: only after the
finalized write or conversion succeeds may an active recovery marker be cleared or
archived. If an explicit
Finish write fails, the prior draft remains open in memory, existing recovery markers
remain intact, and the UI reports that durability was not established so the user can
retry or use Save As. If a boundary-triggered write fails, the prior lifecycle remains
authoritative, the UI marks the tab as finalization-blocked, acquisition does not apply the
new session to the old match, and the detected replacement-team state is retained for
retry. It does not publish `MatchFinalized`, discard the prior recovery marker, or create a
new feed-bound draft until durability succeeds. Abandonment may delete the recovery copy
only after explicit confirmation.

**Wiki footballbox export.** A match can be exported as canonical `{{Football box}}`
wikitext only when it is `Finalized` and `score.classification_complete` is true. If
unclassified goals remain, canonical export stays unavailable in the first implementation;
confirmation alone never silently assigns them to regulation score. A future tracker-side
classification/correction workflow may resolve them. The new, tested exporter is informed
by `WikiFootballbox.h`'s generic parameter serializer and the fork's partial match adapter;
the wikitext field structure is a factual specification, not code to adapt. The original never
implemented
its declared match adapter; the fork's adapter handles teams, score, goals, and own goals
but omits cards, date, and time. The suite's exporter pulls scorers with minutes (incl.
injury time and penalty/own-goal markers), cards (single yellow, or `{{sent off}}` for two
yellows / a straight red), and score/teams from the match file. Date/time uses a verified
`kickoff_at` when available; otherwise the exporter requires user confirmation or omits
those fields. If `history_complete` is false or event minutes are missing, it presents a
validation summary and omits incomplete details unless the user explicitly supplies or
confirms them; it never invents minutes. No extra memory reading is involved. This
completes the feature the original declared but never wired up.

---
