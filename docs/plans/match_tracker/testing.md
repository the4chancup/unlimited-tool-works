# Unlimited Tool Works — Match tracker plan: Testing

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

Part of the [Match tracker plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Testing

- **Recorded-table replay**: a debug command dumps `RawTableSnapshot` fixtures from live
  games; the memory-source trait decodes them to `DecodedGameSnapshot` input through the
  same path as live reads. Event derivation, clock reconstruction, and phase
  transitions are asserted on recorded real matches (including extra time and
  penalty matches).
- **Acquisition validation fixtures**: a team-table candidate with 22 valid entries on only
  one side is rejected; 11 side-correct entries on each side are accepted; wrong-side
  validity markers, implausible team names, and player IDs inconsistent with the side's
  team ID are rejected.
- **Clock calibration unit tests**: synthetic tick sequences with known match lengths,
  stoppages, and injury time; stop/start debounce at the minimum and maximum supported
  poll rates, satisfying both the two-sample and profile elapsed-time thresholds.
- **Feed contract tests**: late-subscriber `SnapshotReplay` initialization without
  one-shot automation; an existing subscriber receiving
  `ScoreReconciled { reason: UnclassifiedGoalObserved }` and updating its uncertain score
  without automation; subscription immediately after that event receiving the updated
  unclassified score and record in its replay snapshot without triggering automation; a
  recovered aggregate unknown-domain goal appearing only in the next `MatchLive` or
  `MatchResumed` snapshot without a standalone `UnclassifiedGoalObserved` event; an
  automation-eligible resolving `Goal` or `OwnGoal` carrying the exact
  `UnclassifiedGoalId`, moving that record from uncertain to classified score without
  double-counting, while a `None` resolution ID adds a newly classified observation;
  `sequence: None` snapshots always processed
  and their cursor honored; rejection of
  publisher-originated `Snapshot`/`SnapshotReplay` and invalid
  provenance/event combinations; rejection of inconsistent `has_reached_live`,
  `score_authority`, lifecycle, and clock-motion combinations; rejection of snapshots
  whose unclassified score projection or `classification_complete` value disagrees with
  `unclassified_goals`; rejection of duplicate/out-of-cursor `UnclassifiedGoalId` values or
  a decreasing `next_unclassified_goal_id`; rejection of `PlayerKey` entries absent from the
matching same-side
  roster and participation entries whose `roster_index` identifies a different player;
  rejection of event or attribution `PlayerRef` values whose side, ID, or roster index
  disagrees with the enclosing side or snapshot roster; atomic snapshot cursor, immutable captured
`ClockWatch`, and both-
  lane registration under concurrent
  publish/watch replacement; initialization/reliable draining before the latest coalesced
  watch; atomic batch validation and sequence assignment, ordered transition snapshots,
  subscription exclusion during the batch, all-or-none enqueue, cursor advance, optional
  retained-snapshot install/clear, and required watch clearing; prepared-batch validation
  and match-ID-associated sequence reservation before file mutation, exclusion of publish,
  subscription, restore, and watch replacement while reserved, old-match cursor selection in
  a cross-match boundary batch, abort without sequence consumption on file failure, and
  infallible commit after durability; single-event wrapper parity;
  new-match sequence reset to 1, provisional same-match
  continuation, and rejection of illegal prior-state or `match_id` transitions; rejection
  of mismatched embedded and retained snapshots/scores; abandonment and `FeedReleased`
  observed by existing subscribers while later subscribers start `Idle`; successful
  recovery restore plus cursor continuation above the restored value, including a saved
  `Live` snapshot normalized to source-loss `Suspended` before a legal `MatchResumed`;
  reliable events
  never dropped; `TeamsDetected` performing setup without score authority, `MatchLive`
  acquiring it, and mid-match reacquisition seeding the score; source-loss suspension
  retaining consumer state and eligible pending metadata; user-stop canceling pending
  automation; user-stop, abandonment, finalization, and feed release returning to manual
  score accounting; `MatchResumed` reconciliation accepted from a suspended snapshot that
  has reached live and rejected from one that has not, with the latter using `MatchLive`
  for its first valid live snapshot; recovered historical events not triggering live
  automation; a recovered substitution appearing
  once in history and never being applied twice to participation state; `match_id`/
  reliable-sequence deduplication after reacquisition; the sequence returned by a successful
  `Goal` publish being used by its later `GoalAmended`; assist-only and kind-only
  `GoalAmended` payloads retaining the original goal clock and complete optional-assist/kind
  state while updating history without scoring or automation; a subscriber attaching between the
  goal and amendment using the complete amendment payload without triggering automation;
  `ScoreReconciled { reason: ClassificationResolved }` updating state without one-shot
  automation for post-boundary live records, recovered/restored records, and resolved
  attribution/domain combinations with no automation-eligible feed event; a late
  subscriber receiving scorer, disciplinary, participation, and team-loading state; a provisional
  correction or new match replacing slots loaded automatically for an earlier feed team
  while retaining a manual load and reporting its mismatch; Undo, per-team Reset, and team
  load/reload leaving authoritative score, scorer, discipline, and participation state
  unchanged; late subscription during source-loss suspension retaining paused feed authority
  and during user-stop suspension restoring manual score accounting from snapshot state
  alone; Autopilot `Off` suppressing playback/highlights without restoring manual scoring;
  `ClockStopped`
  propagating its `stop_epoch` to an existing subscriber and preserving pending automation
  through suspension/resume only when `MatchResumed` reports that same stopped epoch; a
  restart/stop cycle hidden inside a source-loss interval changing the stop epoch or
  reconciling clock motion to `Unknown` and canceling the pending horn; announcement
  deadlines using the original monotonic observation anchor; `WireClock` conversion
  including `observation_age_ms` exactly when that anchor exists and never serializing
  `observation_instant`; a disk-restored
  `UnclassifiedGoal` in `Pending` state with no monotonic anchor using reconciliation only
  and never triggering automation; batch preparation failure leaving reducer, broker, and
  authoritative file state unchanged, consuming no sequence, and exposing no part of the
  batch; lifecycle/result-state mismatches
  being rejected atomically; stale same-match restore rejection, exact idempotent same-match
  restore, `u64::MAX` cursor rejection, and concurrent restore/subscription; finalized
  matches releasing score authority without stopping audio or triggering victory
  automation; `FeedReleased` releasing authority and clearing broker state without
  abandoning the offline draft; `ClockWatch` initial replay, wakeups, coalescing, match-ID
  rejection, clearing, and independent revisions under load.
- **Match lifecycle tests**: same-team rematch detection through both the readiness chain
  and the scan-only table-loss/reallocation fallback; a validated different-team boundary
  after `Live` preparing/reserving the broker batch, durably writing the prospective final
  revision, infallibly publishing the old `MatchFinalized`, clearing the old watch in the
  new-match transition, and publishing
  `TeamsDetected` with a new `match_id`; a boundary write failure publishing neither
  `MatchFinalized` nor replacement `TeamsDetected`, retaining the prior lifecycle and
  recovery marker plus the detected replacement-team state, then a successful retry
  publishing the ordered atomic boundary batch; source progression during the blocked
  interval being excluded from the old match and causing the new match's recovered state
  and `history_complete` to reflect anything missed before retry; a team-identity change
  while provisional replacing the existing `TeamsDetected` draft without finalizing an
  empty match; a provisional `TeamsDetected` draft disabling Finish; discarding that feed-
  bound draft publishing `MatchAbandoned`, clearing retained snapshot/watch state, and
  suppressing immediate recreation from the same source session; transient read failure
  vs. finalized match; Finish and Discard while the
  source session remains live (the terminal `match_id` receives no more deltas and the same
  session cannot create another draft);
  PES-version change publishing `FeedReleased` and detaching an incompatible suspended
  draft, retaining its Offline draft tab while a new feed-bound Current Match becomes
  active, followed by offline save/finalize/discard without further feed publication;
  unknown score-domain goals remaining visible in `Score.unclassified` through snapshot
  replay without triggering automation, then moving atomically by stable record ID when
  classified; normal and own-goal attributions projecting onto the benefiting/scoring side,
  with an unknown-domain own goal preserving its conceding player and emitting `OwnGoal`
  only after regulation-domain classification; multiple `unclassified_goals` records
  retaining distinct IDs, attribution, scorer/clock evidence, and state through save/restore;
  classification before the post-goal boundary publishing an ordinary
  `Goal`, versus classification after a closing boundary while the record remains
  `Pending` publishing only `ScoreReconciled`; one pending record classified into
  regulation and another into shootout with only the regulation classification updating
  ordinary `scorer_counts`; an early `Pending` → `Permanent` transition publishing
  `ScoreReconciled { reason: ClassificationBecamePermanent }`; unresolved `Pending`
  records becoming `Permanent` through a reconciliation in the same atomic batch
  immediately before `MatchFinalized`; legitimate gauge/state decreases and
  `UnknownScalar` changes being accepted while only verified monotonic-counter decreases
  invoke invalid-read/reset handling; two goals or goal+assist/goal-kind evidence spanning
  adjacent polls; late assist or kind evidence amending before kickoff and being rejected
  after a phase or other amendment-closing boundary; closure of an initial goal holdback
  flushing every confirmed known-domain goal or materializing an unknown-domain record
  without discarding its delta; yellow-first
  and red-first second-yellow
  correlation, timeout, and phase/suspension boundary closure, all retaining the first
  delta's clock and never dropping a confirmed card; penalty and shootout score separation;
  clock stop/start debounce.
- **File format tests**: malformed, future-version, and oversized match files; canonical
  amended-goal materialization; profile field-class, `has_reached_live`, latest clock,
  optional timed `display_minute` versus shootout `None`, observed/reconstructed optional
  `Clock.seconds` and `InjuryTime.seconds` versus recorded-minute `None`, non-serialized
  `observation_instant`, and `ClockMotion`/`stop_epoch` round-trips; `score_authority`
  round-trips in ordinary and
  recovery files; ordinary read-only Open treating recorded authority as historical data
  without installing feed authority; recovery-only feed-binding-state round-trips;
  `PlayerKey`-keyed scorer, discipline, and participation maps round-tripping through
  ordinary and recovery files; multiple `unclassified_goals` records with stable IDs,
  scored/own-goal attribution, `Pending`/`Permanent` state, unclassified totals,
  `classification_complete`, and a non-reusing `next_unclassified_goal_id` cursor surviving
  save/restore; a restored
  `UnclassifiedGoal` in `Pending` state lacking a monotonic anchor and remaining ineligible
  for one-shot automation regardless of UTC or apparent stopped-window state;
  recovery-file discovery, cursor-preserving restore, and marker cleanup; prepared broker
  reservation followed by atomic conversion of an active recovery marker to `offline_only`
  and infallible profile-change `FeedReleased` commit, including preparation/conversion
  failure, crash/interruption cases, and rejection of feed Restore for the released
  `match_id`; Finish with no prior timed save, suspension,
  recovery file, or user save creating a durable finalized safety copy; write interruption
  retaining the draft and existing marker; atomic autosave interruption; optional 140-cell
  heatmap round-trip and absence, dimension/size rejection, and confirmation that heatmap
  changes do not create a hidden generic-stat grid timeline; worst-case JSON size and
  autosave latency for a full 120-minute match.
- **Cross-platform compilation**: Windows native plus non-Windows/WASM file-viewer
  build (Start tracking disabled, file viewing enabled).
- **Manual**: a full simulated stream with the music player on autopilot (shared with
  the music player's test plan).

### Verifying source-derived findings

The suite's first implementation registers PES 2019 first, then PES 2021, then the other
versions, PES 2017 among them (README "Priority"), so verification is prioritized
accordingly. PES 2019 and PES 2021 register on their measured layouts (README "Independent
measurements"); the unofficial fork's PES21 claims (pointer chains, substitution journal,
direct clock read) stay **unregistered** until verified and are recorded here as the
starting point for that work. Unresolved PES17 claims from either source gate registering
PES 2017, not the first release. A claim that fails verification is dropped, and the
affected plan text reverts.

**PES17 verification required before registering PES 2017:**

Each task must reach a recorded pass/fail decision before PES 2017 is registered. Failure
disables the affected optional capability or fast path but does not block the scan-based
tracker itself. Phase verification gates a version's release only if ET/shootout
classification and automation remain promised for it; buckets 6 and 7 are unobserved on
every version, so that gate is the same on PES 2019 and PES 2021.

- **PES17 phase buckets**: the original confirms buckets 0–2 as the first half and 3–5
  as the second half, but only hypothesizes `1st ET: [6-7?], 2nd ET: [7?-8?]` and leaves
  segment 8 as a TODO (`MatchReader.cpp:171`). Capture and label all nine segments across
  regulation, both extra-time periods, and a shootout. Confirm the 6/7/8 mapping and
  transition behavior before enabling shootout classification or ET/penalty-phase
  automation; until then those buckets remain unregistered and produce `Phase::Unknown`.
  *Measured on PES 2017, 2019 and 2021 (README "Independent measurements"):* the nine
  buckets are 15-minute periods of the game clock. 0 holds minutes 0–15, 1 holds 15–30,
  2 holds 30–45 and the first half's stoppage time, 3, 4 and 5 the same for the second
  half (one match per version sampled every 3 s against the HUD clock). A PES 2021
  shoot-out writes its goals and kicks to bucket 8. Buckets 6 and 7 (extra time) were not
  observed on any version, so their mapping and PES 2017's bucket 8 stay open.
- **PES17 heatmap grid**: the original maps the "Play Area Grid" at entry+0x1034
  (`StatsInfo.cpp:150`, 140×u32) and processes it only through the generic stat path as a
  summed scalar column, hidden by default; it never interprets or renders the 10x14 grid as
  a heatmap. Confirm that the underlying 140 cells — not merely their generic aggregate —
  populate with stable spatial semantics during a live PES17 match (values update as
  players move), determine 10×14 orientation and whether sides/halves require flipping,
  and whether values are cumulative or instantaneous. If it is confirmed to populate with
  stable semantics, PES17 heatmap recording and the viewer ship in the first release;
  otherwise the viewer remains hidden for PES17 and the signature is not registered.
  *Measured on one PES 2017 match (README "Independent measurements"):* the 140 cells
  populate and are cumulative (no cell decreased over 327 samples); each full-match player
  totals about 630, one count per about 48 game ticks. They read as 14 rows of 10,
  row-major, row 0 at the player's own goal for both sides in both halves: each
  goalkeeper stays in rows 0–2 for the whole match, so the game flips at half time and
  the viewer does not. Which touchline column 0 lies on was not determined. Unlike PES
  2021's grid, all 140 values are cells.
- **PES17 pointer-chain locator (fast path)**: the fork's profile defines two chains
  from the same root: `root = *(moduleBase + 0x21A9E68)`. The **readiness chain** is
  `container = *(root + 0x124)`, `flag = *(u32 *)(container + 0x1168)`; `flag == 0` claims
  that a match is live. The **stats-base chain** is `subObj = *(root + 0x2890)`,
  `stats-table base = subObj + 0x317FC`; the last step is arithmetic, not a dereference.
  The operational fork does not use this profile in its PES17 `findStats()` path.
  Confirm that applying the profile's `+0x3804` player-array offset to the resolved base
  yields the same home stat-entry address found by the original scan, that the derived
  away address also agrees, and that the readiness flag distinguishes live/not-live. If
  it holds, PES17 gains the ASLR-proof fast path described under "Acquisition and
  polling"; if not, the scan-only path stands.
  *Checked on PES 2017 1.0.0.0 (the README's executable):* the claim fails there. Before
  kickoff the root reads 0; during the match the readiness flag reads 1, not 0, and
  `subObj` reads 0x8000, so the derived home entry (0x3D000) holds no player, while the
  scan finds the home block in its 0x90000 region as the original does. The profile was
  likely made against another executable; on this one the scan-only path stands.

**PES21: the unofficial fork's claims, unregistered until verified (the measured layout
registers without them):**

- **Pointer-chain stats-table locator (PES21)**: same two-chain structure. The readiness
  chain is `root = *(moduleBase + 0x36F4178)`, `container = *(root + 0)`,
  `flag = *(u32 *)(container + 0)`; `flag == 0` claims that a match is live. The stats-base
  chain is `subObj = *(root + 0xBD08)`, `stats-table base = subObj + 0x750D0`. Confirm both
  on a live PES 2021 (1.01).
- **PES21 stat layout**: confirm entry size 8500 and indicator validation (0xFFFD/0xFFFE
  at +0, ID at +4). Normalize the fork's declared 269-slot catalog into unique Studio
  `catalog_id` values: account for all 263 explicit initializers and six default slots,
  bounds-check every offset/type/count against the entry size, and explicitly resolve or
  reject duplicate IDs, overlapping aliases, and conflicting definitions — including the
  duplicate `visualIndex` ranges 110–134 and 176–185 — before enabling them. Confirm the
  physical 40-slot layout and capture a roster using slots 32–39. The
  suite must use dynamically sized vectors or the profile maximum of 40 throughout; the
  fork's 32-slot `teamInfo` structures are not reusable. Cross-check a sample of named
  stats (Goals, Cards, Saves, Minutes, Rating) against observed match events.
  *Measured (README "Independent measurements"):* entry size 8500 confirmed. The marker
  is the u32 before each entry's player ID (0xFFFD home, 0xFFFE away) on both PES 2021
  and PES 2019, so the fork's "+0 marker, +4 ID" is the same layout counted from four
  bytes earlier. The team table's physical layout has 40 player slots
  per side on both versions; no roster using slots 32–39 was captured. Instead of
  normalizing the fork's catalog, the rebuild maps its own catalog from the game's pages
  (719 entries on PES 2021, 711 on PES 2019): Goals, Assists, Cards, Shots, Passes,
  Saves, Minutes on Pitch and Player Rating are checked against Team Stats, Match Events
  and Player Ratings in two matches, and an out-of-sample match per version reproduces
  every goal, card, substitution and Team Stats total. The team table sits at the same
  offset in its region across two launches on PES 2021; on PES 2019 it started at five
  different offsets in five launches (0x32A58, 0x38168, 0x2B5B8, 0x2F218, 0x2F388), so a
  profile locates it by its pairs signature, not by offset; the rebuild's reader now does so
  and attached in the last three of those launches, with the testbot teams and with the
  betting rotation's.
- **Substitution journal (unverified)**: verify the fork's claimed `u8` count at
  `table_base + 0x10B4D4` (maximum 12) and 40-byte record `i` at
  `table_base + 0x10B2F4 + 40*i`, with `type` at `+0`, `team` at `+4`, `period` at `+8`,
  `minute_in_period` at `+12`, `inIdx` at `+20`, and `outIdx` at `+24`. Confirm that the
  indices match the actual substitution, the recorded period/minute is exact, and the
  restart-replay behavior holds across a tracker restart mid-match.
- **Direct clock read (unverified)**: verify the fork's claimed chain
  `manager = *(u64 *)(moduleBase + 0x3705E10)`,
  `clock_object = *(u64 *)(manager + 0x50)`, the `u32` segment at
  `clock_object + 0x3FCD4`, and the `f32` ticks at `clock_object + 0x3FCD8`. Confirm that
  segment/ticks reproduce the scoreboard minute + injury time across all phases and into
  extra time. Also verify tick stagnation and resumption across ordinary stoppages, goal
  celebrations/replays, halftime, and menus, and confirm that a unique post-goal restart
  can be derived before enabling PES21 autopilot.
- **PES21 heatmaps**: confirm the 140×u32 player grid at entry+0x1938 and the 8 per-team
  grids populate during a match. Determine the meaning of all 8 grids, validate the
  fork's five-segment accumulation and odd-segment flipping, and establish whether values
  are cumulative before exposing or serializing them.
  *Measured on one PES 2021 match (README "Independent measurements"):* the player grid
  at entry+0x1938 populates and is cumulative (no cell ever decreased over 364 samples).
  Its first 139 u32 are cells of a 14-row by 10-column map, row-major, with row 0 at the
  player's own goal for both sides and both halves (both goalkeepers stay in rows 0–2 all
  match, so the game has already flipped it at half time). The 140th u32 is not a cell: it
  equals the sum of the other 139 (±1 for a sample in flight), a count of the samples
  taken, one per about 54 game ticks. Which edge column 0 is, and the 8 per-team grids,
  were not determined.
