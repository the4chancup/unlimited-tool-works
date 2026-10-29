# Unlimited Tool Works — Match tracker plan: Match model

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

Part of the [Match tracker plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Match model and event derivation

The tracker is a deterministic reducer over
`(prior MatchState, current DecodedGameSnapshot)`, producing a new state and zero or
more events. Raw stat deltas are pairwise `DecodedGameSnapshot` comparisons; lifecycle,
clock, correlation, and recovery logic additionally use
explicit reducer state. This is the part `MatchReader` buried in widget code,
restated as plain logic:

**Match lifecycle.** A match transitions through explicit states. The complete lifecycle
state is represented in snapshots, including the reason for suspension:

```rust
pub enum LifecycleState {
    TeamsDetected,
    Live,
    Suspended { reason: SuspensionReason },
    Finalized,
}
```

No active draft is represented by tracker-local `Idle` and the absence of a retained
snapshot. Abandonment is represented by the reliable `MatchAbandoned` event, after which
the broker retains no snapshot. Neither is a retained `MatchSnapshot.lifecycle` value.
Short source failures debounce within a reacquisition grace period before suspension is
published; a recovery autosave fires at most once per suspension episode. Footballbox
export requires `Finalized` and a classification-complete score; ordinary Save can save a
draft from any state.

The "Retained snapshot?" column describes a match while it remains feed-bound. Feed
release is orthogonal to lifecycle: it clears the retained snapshot and watch while leaving
the released match's reliable cursor terminal until a new match replaces it; later offline
save, finalization, or discard does not restore the cleared state.

| State | Entry condition | Score authority | GUI tab | Retained snapshot? |
|-------|-----------------|-----------------|---------|--------------------|
| `TeamsDetected` | Team table located, rosters read | No | Current Match (provisional) | Yes |
| `Live` | Stats table acquired and at least one valid live clock observed; motion is `Running` or `Stopped` during normal observation and may be `Unknown` during reconciliation | Yes | Current Match | Yes |
| `Suspended { SourceLoss }` | Read failure past debounce grace, or saved `Live` draft normalized during recovery before source reacquisition | Yes (if previously Live) | Current Match (paused) | Yes |
| `Suspended { UserStopped }` | User pressed Stop tracking | No | Current Match (paused) | Yes |
| `Finalized` | Finish after reaching `Live` / different-team boundary / verified rematch boundary | No | Finished tab | Yes after standalone feed-bound Finish; transient and replaced by the new `TeamsDetected` snapshot in a session-boundary batch; otherwise no |
| (no draft) | Idle / searching | No | — | No |
| (abandoned) | `MatchAbandoned` published | No | — | No (cleared) |
| (offline draft) | `FeedReleased { ProfileChanged }` | No | Offline draft tab | No retained snapshot/watch; the released match's terminal reliable cursor remains until replacement |

Table loss triggers `Suspended`, not immediate finalization. A source/read-loss
suspension retains the music player's loaded teams, score, and pending one-shot metadata
for deduplication; on resumption, a pending horn proceeds only if its deadline has not
expired and the reconciled clock state proves the same post-goal stopped window (same
`stop_epoch`), otherwise it is canceled. A stale pending horn never starts merely because
the feed resumed. The authority table below summarizes which events acquire, release, or
retain authority and which cancel pending automation.

Feed score authority is carried in `MatchSnapshot` via explicit `has_reached_live` and
`score_authority` fields; consumers must not infer it from clock presence. Autopilot mode
controls playback and highlighting only; selecting `Off` does not transfer score
authority. `MatchResumed` reconciles from the tracker snapshot without merging ad hoc
manual score changes. Manual goalhorn presses during source-loss suspension remain
non-scoring soundboard actions.

| Event | Acquires authority? | Releases authority? | Cancels pending automation? | Retains snapshot/watch? |
|-------|--------------------|--------------------|-----------------------------|-------------------------|
| `TeamsDetected` | No | No | No | Snapshot retained/replaced; prior watch cleared |
| `MatchLive` | Yes | — | No | Retains |
| `MatchResumed` | Yes | — | Conditional — preserves only when the snapshot proves the same stopped `stop_epoch`; otherwise cancels | Retains |
| `MatchSuspended { SourceLoss }` | — | No (retains if was Live) | No (pauses) | Retains |
| `MatchSuspended { UserStopped }` | — | Yes | Yes | Retains |
| `MatchFinalized` | — | Yes | Yes | Retains terminal snapshot/watch unless followed atomically by new-match `TeamsDetected`, which replaces the snapshot and clears the old watch |
| `MatchAbandoned` | — | Yes | Yes | Clears |
| `FeedReleased` | — | Yes | Yes | Clears retained snapshot/watch; retains the terminal reliable cursor until replacement |

While the feed is score authority, no manual soundboard action may alter authoritative
score, scorer totals, discipline, or participation state. Undo, per-team Reset, and team
load/reload affect playback state, caches, and loaded audio only. Once authority is
released, those controls resume their ordinary manual match-state behavior. `Snapshot`,
`MatchLive`, `MatchResumed`, and `ScoreReconciled` are reconciliation messages and never
trigger one-shot automation.
Score correction while the feed is authoritative is not supported in the first
implementation. The user may stop or abandon an active match to return the Music player to
manual score accounting; correcting the tracker's canonical event history is future work.
`MatchFinalized` releases feed score authority: consumers return to manual score
accounting for any post-finalization interaction, while the finalized match's terminal
lifecycle and saved result are unchanged. `MatchFinalized` neither stops currently playing
audio nor triggers victory automation.

**Cross-plan alignment:** the Music player plan's Autopilot section mirrors the
authority, automation, and reconciliation rules in the tables above (uncertain-goal
resolution via `resolves_unclassified`, reconciliation-only snapshot handling, monotonic
announcement deadlines, `stop_epoch`-verified horn continuity, `SecondYellow` → `event
red`, shootout isolation, per-side auto team loading, and the `Off` mode's
no-authority-transfer semantics); the consumer-side rules live in that plan and in the
core plan's key-decision summary.

`Finalized` is terminal for that `match_id`, and `MatchAbandoned` clears that
`match_id`: no further stat or event deltas are applied to either. The acquisition worker
may continue only to detect the next positive session boundary; it suppresses the
still-running source session and cannot create another draft from the same table/readiness
cycle.

- **Stat deltas**: every stat of every player is compared per update; segmented stats
  are compared per segment. Profiles classify verified fields as monotonic counters,
  gauges, state values, or grids. Unidentified scalar fields use `UnknownScalar`: their
  changes are recorded, but they never generate match events and their decreases are not
  used as invalid-read, reset, or session-boundary evidence until their semantics are
  verified. Each accepted change lands in the per-player stat history, referencing a
  shared timeline entry (optional UTC wall-clock timestamp + reconstructed clock).
  Decreases of gauges and state values, such as stamina, rating, and position, are ordinary
  changes. Only positive increments of profile-declared event counters can create counter-
  derived player events. Verified first-minute participation transitions and substitution-
  journal additions may create `SubOn`; clock and lifecycle events are produced only by
  their dedicated state machines. Decreases of profile-declared monotonic counters never
  synthesize inverse events or silently alter
  canonical score or history. A broad reset starts a new match only when corroborated by
  the session-boundary rules above; otherwise the snapshot is rejected and acquisition
  suspends or reacquires pending confirmation. Isolated implausible monotonic-counter
  decreases are treated as invalid-read evidence. Rejected snapshots are recorded only in
  diagnostics, not canonical match history.
- **Grid fields**: grid/blob fields such as "Play Area Grid" are excluded from the generic
  scalar stat-delta history. They are sampled on the profile's heatmap throttle and stored
  only in the optional latest-grid section. A scalar aggregate may be computed for
  diagnostics or compatibility display, but it is not persisted as per-cell stat history.
- **Events**: goal, own goal, card, dismissal, and sub-on. The initial goal holdback
  waits at most two polls or 250 ms, whichever comes first, for both assist and goal-kind
  correlation evidence. If a suspension, generation/session boundary, phase boundary, or
  termination closes the holdback, every confirmed goal delta is flushed immediately with
  the evidence currently available when its score domain is known; otherwise it is
  materialized as an `UnclassifiedGoal`. Closing a boundary never discards a confirmed
  positive goal delta. A goal is otherwise emitted when the holdback expires, without an
  assist if none was correlated; `Goal.clock` is captured from the
  `DecodedGameSnapshot` containing the goal delta, not from the later timeout. Later assist
  or goal-kind evidence updates the stored canonical event and publishes a presentation-
  only `GoalAmended` carrying the complete current assist and kind. It never re-triggers
  scoring or automation. A goal's amendment record closes on the first subsequent kickoff
  (`ClockStarted`), another potentially conflicting goal, a suspension or generation
  change, a phase boundary, or match termination. Recovered aggregate
  assist deltas never amend live goals. Kickoff is therefore a closing boundary, not an
  extension of the initial assist-correlation timeout. Multiple simultaneous goals/assists
  that make pairing ambiguous are emitted without pairing. On PES17 the outgoing player is
  not identifiable from the stats (first-minute-on-pitch only reveals the incoming
  player; the original defined a slot for the outgoing player that never got filled,
  and the feed's `player_out` is optional there). **On PES21 the fork reports a
  substitution journal** *(unverified)* at a fixed offset from the stats-table base — a
  count byte plus 40-byte records carrying team, period, minute-in-period, and both
  `inIdx`/`outIdx` — so `player_out` becomes exact and the sub minute is the recorded
  value, not the live clock. The journal lives in the game's memory and survives a
  tracker restart, so a re-attach replays existing records stamped with their own
  recorded minute.
- **Recovered aggregate state**: when the stats table is first acquired mid-match,
  pre-existing counters seed the recovered aggregate and any score, scorer, and card
  state whose domain is known, but do **not** create ordinary timeline events because
  their minutes and ordering are unknown. Goals whose score domain is unknown become
  `UnclassifiedGoal` records and do not enter ordinary `scorer_counts` until classified;
  shootout goals never enter ordinary `scorer_counts`. The match file stores initial
  counters in a distinct `recovered_aggregate` section and sets `history_complete: false`.
  Journal records carrying their own recorded match minute may create
  `RecoveredHistorical` timeline events. Neither form triggers live one-shot automation
  (see `match_feed.md` "The match feed" — provenance). `history_complete` is provisionally true at match
  creation and is confirmed on first stats acquisition only when observation begins before
  live counters advance; that first acquisition sets it false if it reveals pre-existing
  counters. It becomes permanently false for that match if the tracker attaches late or
  reacquisition reveals counter changes whose ordering or time was not observed. A
  suspension with no intervening counter changes does not by itself make history
  incomplete. While `has_reached_live == false`, `history_complete == true` is provisional
  and means only "not yet disproven"; readers and exporters must not treat history
  completeness as confirmed until the match has reached `Live`. First stats acquisition
  either confirms the value or sets it permanently false.
- **Penalty and shootout classification**: a same-player "Penalties scored" delta
  correlated with a "Goals Scored" delta marks a regulation or extra-time penalty goal.
  Shootout attempts are classified only after the phase detector has positively entered
  the verified penalty phase. `OpenPlay`, `Penalty`, and regulation-phase `Unknown` goals
  update regulation score and ordinary scorer counts. `Shootout` updates only
  `shootout_score` and does not affect the music player's `goals`, `teamgoals`, `first`,
  or `mostgoals` condition state. `GoalKind::Unknown` is published only when the phase is
  positively known to be regulation and the remaining ambiguity is open play vs.
  penalty. If the score domain of a **live-observed** goal is unknown, the reducer retains
  the delta as an attributed `UnclassifiedGoal` in `Pending` state, updates
  `Score.unclassified` for the benefiting/scoring side, and publishes the snapshot-bearing,
  presentation-only `ScoreReconciled { reason: UnclassifiedGoalObserved }`. It publishes no
  automation-eligible `Goal` until the phase is classified. An own-goal delta observed while
  the score domain is unknown is retained as `GoalAttribution::OwnGoal`; once classified, it
  emits `OwnGoal` only for a regulation-domain resolution, while any other domain publishes
  `ScoreReconciled { reason: ClassificationResolved }`. `OwnGoal` therefore has no
  `GoalKind` field: it is emitted only
  after regulation-domain classification, so no kind discrimination is needed. A record
  materialized from recovered aggregate counters is incorporated into the next `MatchLive`
  or `MatchResumed` snapshot and publishes no standalone `UnclassifiedGoalObserved` event.
  Each live-observed record retains its stable match-scoped ID, attribution, original clock,
  observed segment evidence, and ordering needed for later classification; recovered
  aggregate records use the optional fields described in the feed model below.
  If classification remains impossible, its state becomes `Permanent`. When a record
  becomes `Permanent` before finalization, the reducer publishes the snapshot-bearing,
  presentation-only
  `ScoreReconciled { reason: ClassificationBecamePermanent }`. At feed-bound finalization,
  any required `Pending` → `Permanent` reconciliation is published in the same atomic
  batch immediately before `MatchFinalized`; offline finalization applies the same change
  only to its durable local revision and publishes nothing. Neither transition alone
  changes `history_complete`; that flag changes only under the attach/reacquisition and missing-
  order/time rules above. The reducer does not guess or trigger one-shot automation.
  `Pending` records remain eligible for later classification across kickoff, suspension,
  and other automation-closing boundaries; those boundaries determine only whether
  eventual resolution publishes an automation-eligible `Goal` or `OwnGoal`, or a
  presentation-only `ScoreReconciled`. A record becomes `Permanent` at finalization, or
  earlier only when the
  reducer can prove that no future observation in the match can resolve its score domain.
  `Permanent` means automatically unresolvable in the first implementation; a
  future explicit correction workflow may still resolve it. `Score.unclassified` is the
  per-side projection of the canonical `unclassified_goals` collection, not the only
  stored representation; `classification_complete` is true exactly when that collection
  is empty, independently of `history_complete`. Unclassified totals are displayed as
  uncertain audit state, serialized with their underlying records in snapshots and match
  files, and never contribute to regulation music conditions or one-shot automation. For
  a live-observed record, if classification completes while the original post-goal
  automation window is still open, the reducer publishes the ordinary `Goal` with its
  original clock and atomically moves the record into the appropriate score domain. An
  automation-eligible `Goal` that resolves an existing `UnclassifiedGoal` carries that
  record's stable match-scoped ID in `resolves_unclassified`. Consumers remove that exact
  uncertain record before applying the classified score/scorer update; `None` denotes a
  newly observed goal that was classified immediately. An automation-eligible `OwnGoal`
  that resolves an existing attributed `UnclassifiedGoal` likewise carries that record's ID
  and moves it out of uncertain score before applying the benefiting-side regulation score;
  `None` denotes an own goal classified immediately. If classification completes after
  kickoff, suspension, or another closing boundary, it instead publishes the snapshot-
  bearing, presentation-only `ScoreReconciled { reason: ClassificationResolved }`; that event
  updates the appropriate score domain and updates ordinary scorer state only for a
  classified non-shootout goal; it is never eligible for one-shot automation. A record
  materialized from recovered aggregate counters always uses
  `ScoreReconciled { reason: ClassificationResolved }` when later classified. The Music
  player plan needs the corresponding rule that a resolving `Goal` or `OwnGoal` moves the
  existing uncertain record rather than adding a second goal. The Music player treats
  shootout-horn playback as a separate setting (default off) and never processes a shootout
  success as a regulation goal.
- **Announcement deadline anchoring**: live observations retain their original in-process
  monotonic observation instant. The Music player computes `Goal` and `OwnGoal`
  announcement deadlines from that instant, not from message receipt or UTC wall time. A
  delayed-classification goal uses only the remaining delay; if the deadline has passed,
  it may play immediately only while the same stopped window is still proven open. A
  `UnclassifiedGoal` restored in `Pending` state from disk has no valid in-process
  monotonic observation anchor. Even if its score domain is classified before the
  apparent post-goal window closes,
  restoration may publish only presentation/state reconciliation (`ScoreReconciled`, or a
  recovered-history notification followed by reconciliation), never an automation-
  eligible `Goal`. UTC timestamps must not be used to reconstruct the deadline. The Music
  player plan needs the corresponding deadline-anchoring rule.
- **Cards and dismissals**: a same-player red delta correlated with the player's second
  yellow produces one `SecondYellow` dismissal, not separate yellow and red events.
  Yellow/red correlation uses the same maximum of two polls or 250 ms, whichever comes
  first; either half of a possible second-yellow pair may open the holdback. `Card.clock`
  is captured from the snapshot containing the first relevant delta. If the window closes
  without a valid pair, the reducer emits the individually supported `Yellow` or
  `StraightRed` event using that original clock; boundary closure never discards a
  confirmed card delta. The holdback must not cross a suspension, phase or session
  boundary, another card for the player, or match termination. `SecondYellow` fires the
  music export's `event red` clip.
- **Phase detection**: for PES17, phase is inferred from the highest active 9-bucket
  segment, using the verified segment map for buckets 0–5. Buckets 6–8 remain hypothetical
  and produce `Phase::Unknown` until verified; ambiguous or unverified segment combinations
  likewise produce `Unknown` rather than guessing. Most counters share this layout on both
  versions.
- **Clock reconstruction**: on PES17 the displayed minute is the *mode* of "Last
  minute on pitch" across all players (the mode defeats per-player glitches caused by
  subs). The original's code comments describe a mode but its `std::max_element` on
  the count map actually returns the maximum minute (lexicographic pair comparison),
  not the most frequent one. The suite implements the actual mode and excludes invalid
  values and players not yet participating, determined from roster, first-minute, and
  current-position evidence; it does not discard minute zero from confirmed starters.
  A tie prefers the candidate nearest the previous reconstructed minute, then the lower
  minute. The tick count is the max of "Game ticks played" (summed across its 13
  segments per player). Seconds are interpolated from ticks using an adaptive ticks-per-
  minute estimate. Its initial value uses a fixed profile seed of a 10-minute match
  (`32 × 10` ticks per game minute), matching the legacy default at
  `MatchReader.h:74`; the reducer auto-calibrates against the recorded minute→tick history
  and records the detected match length. The seed is per profile: at Match Time 10 the
  measured rate is 320 ticks per game minute on PES 2017, about 356 on PES 2019 and about
  371 on PES 2021 (stoppage included). The legacy correction, which moves in steps of 32,
  settles at 352 on PES 2019 and records its 10-minute matches as 11 minutes long. The 13
  "Game ticks played"
  counters are per playing position, not time segments (a goalkeeper's ticks land in one
  counter, a centre-back's in another), which is why the sum, not any one counter, is
  the player's time. Injury time is
  detected when the minute pins at 45/90/105/120; halves, extra time, and penalties
  are tracked through phase transitions. Clock stop/start is detected from tick
  stagnation through an explicit debounce state machine: a stop is published only after
  at least two consecutive valid snapshots with unchanged ticks, and a restart only after
  two consecutive advancing snapshots. Clock debounce uses profile-owned
  `stop_debounce_ms` and `start_debounce_ms` constants verified from fixtures; both the
  two-sample condition and the corresponding elapsed-time minimum must be satisfied. The
  elapsed-time and event-clock anchor is the first unchanged or advancing sample,
  respectively. Read failures and phase transitions are not treated as clock samples. The
  original's estimate jumps up to three and a half minutes ahead around a substitution
  made during a stoppage and is back within a minute of the HUD about a minute later (seen
  on PES 2017, 2019 and 2021 against the HUD clock; README "Independent measurements"), so
  the fixtures include such a substitution and the reducer must not reproduce the jump. The
  verified values of each version are recorded in its profile before that version is
  registered, so changing the poll rate does not materially change behavior. **On PES21
  the fork reads the clock directly** *(unverified)* via a
  pointer chain to a clock object exposing the current segment (u32) and a raw tick
  count (f32, ticks/3600 = minutes); the segment maps to the period base minute and
  even segments flag injury time. The direct read supersedes the reconstruction where
  available; the reconstruction remains the PES17 path and the fallback if the PES21
  clock chain breaks.
- **Stop reasons**: inferred from which counters moved around the stop — offside
  beats foul, goal/own goal, a save implies a corner, an off-target shot implies a
  goal kick — replicating the original's heuristic table. A half-end reason (the
  original defined it but never actually inferred it) falls out of the clock model
  instead: a stop landing on a phase boundary.
- **The post-goal window**: a goal freezes the clock through the celebration and the
  replay; the next `ClockStarted` is the kickoff, a few seconds after the replay is
  dismissed. That stop/start pair brackets the goalhorn window the music player's
  autopilot uses (horn after its announcement delay, fade on the restart). Replay/menu
  state itself is **not** present in the known tables — if a game-state flag is ever
  located in memory, it joins the signature set and the feed as a dedicated event; until
  then the kickoff restart is the proxy.
- **Cup stage / competition mode** (open item, tied to Cup-mode migration): PES knows
  the current competition stage (group, knockout round, final) for its own bracket and
  extra-time/penalty logic, but neither the original nor the fork mapped where this is
  stored. Matches are currently played in Exhibition mode, which carries no stage info;
  the planned migration to Cup mode makes this data available. It likely lives near the
  team-data table or the match/stat context the pointer chain resolves to. If located,
  it joins the signature set and the feed as a dedicated event, allowing the match-type
  selector to be auto-populated instead of manual.

---
