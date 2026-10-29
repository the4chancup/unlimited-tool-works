# Unlimited Tool Works — Match tracker plan: Match feed

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

Part of the [Match tracker plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## The match feed (`libs/match_feed`)

The interface between the tracker and the rest of the suite. It is a lib crate
because tool crates never depend on tool crates; the tracker publishes, any tool can
subscribe (today: the music player; future consumers: overlays, a wiki helper).

```rust
// Shared identity, roster, and score primitives are canonically declared in their
// corresponding match_feed modules and re-exported here.
pub use crate::identity::MatchId;
pub use crate::roster::{PlayerRef, Roster, Side};
pub use crate::score::SideScore;

// MatchId is a UUID v4 in schema version 1. PlayerRef contains side,
// roster_index, stable game player_id, and display/fallback name. Roster
// contains stable team identity, display names, and ordered player entries.
// SideScore contains explicit home and away totals. These fields and their
// serialized representations are normative parts of the public contract even
// though their implementations live in separate modules.

pub enum Phase {
    FirstHalf,
    SecondHalf,
    ExtraTimeFirstHalf,
    ExtraTimeSecondHalf,
    Shootout,
    Unknown,
}

pub struct InjuryTime {
    pub minutes: u32,
    pub seconds: Option<u8>,
}

pub struct Clock {
    pub phase: Phase,
    pub display_minute: Option<u32>, // None during a shootout or when no valid clock has been reconstructed
    pub seconds: Option<u8>,
    pub injury_time: Option<InjuryTime>,
    pub observed_at_utc: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub observation_instant: Option<Instant>,
}

pub struct WireClock {
    pub phase: Phase,
    pub display_minute: Option<u32>,
    pub seconds: Option<u8>,
    pub injury_time: Option<InjuryTime>,
    pub observed_at_utc: Option<DateTime<Utc>>,
    pub observation_age_ms: Option<u64>,
}

pub struct Rosters {
    pub home: Roster,
    pub away: Roster,
}

pub struct MatchSnapshot {
    pub match_id: MatchId,
    pub lifecycle: LifecycleState,
    pub rosters: Rosters,
    pub clock: Option<Clock>,
    pub clock_motion: Option<ClockMotion>,
    pub has_reached_live: bool,
    pub score_authority: bool,
    pub score: Score,
    pub unclassified_goals: Vec<UnclassifiedGoal>,
    pub next_unclassified_goal_id: u64,
    pub scorer_counts: BTreeMap<PlayerKey, u32>,
    pub disciplinary: BTreeMap<PlayerKey, DisciplineState>,
    pub participation: BTreeMap<PlayerKey, ParticipationState>,
    pub history_complete: bool,
}

pub struct FeedMessage {
    pub match_id: MatchId,
    pub sequence: Option<u64>, // None only for subscriber-local SnapshotReplay
    pub provenance: Provenance,
    pub event: FeedEvent,
}

pub struct PublishRequest {
    pub match_id: MatchId,
    pub provenance: Provenance,
    pub event: FeedEvent,
}

pub enum Provenance {
    LiveObserved,
    LiveDerived,
    UserAction,
    RecoveredHistorical,
    SnapshotReplay,
}

pub enum FeedEvent {
    Snapshot       { state: MatchSnapshot, through_sequence: u64 },
    TeamsDetected  { home: Roster, away: Roster },
    MatchLive      { snapshot: MatchSnapshot },
    Goal           { side: Side, scorer: PlayerRef, assist: Option<PlayerRef>,
                     kind: GoalKind, clock: Clock,
                     resolves_unclassified: Option<UnclassifiedGoalId> },
    GoalAmended    { goal_sequence: u64, side: Side, scorer: PlayerRef,
                     assist: Option<PlayerRef>, kind: GoalKind, goal_clock: Clock },
    ScoreReconciled { snapshot: MatchSnapshot, reason: ScoreReconcileReason },
    OwnGoal        { conceding_side: Side, player: PlayerRef, clock: Clock,
                     resolves_unclassified: Option<UnclassifiedGoalId> }, // scoring side is the opposite
    Card           { side: Side, player: PlayerRef, card: CardKind, clock: Clock },
    SubOn          { side: Side, player_in: PlayerRef, player_out: Option<PlayerRef>, clock: Clock },
    ClockStarted   { clock: Clock },
    ClockStopped   { clock: Clock, reason: StopReason, stop_epoch: u64 },
    MatchSuspended { last_clock: Option<Clock>, reason: SuspensionReason },
    MatchResumed   { snapshot: MatchSnapshot },
    MatchFinalized { last_clock: Option<Clock>, score: Score },
    MatchAbandoned,
    FeedReleased   { reason: FeedReleaseReason },
}

pub struct ClockWatch {
    pub match_id: MatchId,
    pub revision: u64,
    pub clock: Clock,
}

pub enum ClockMotion {
    Unknown,
    Running,
    Stopped { stop_epoch: u64, reason: StopReason },
}

pub enum FeedReleaseReason {
    ProfileChanged,
}

pub enum ScoreReconcileReason {
    UnclassifiedGoalObserved,
    ClassificationBecamePermanent,
    ClassificationResolved,
}

pub enum SuspensionReason {
    SourceLoss,
    UserStopped,
}

pub enum StopReason {
    Unknown,
    Foul,
    Offside,
    Goal,
    OwnGoal,
    CornerKick,
    GoalKick,
    HalfEnd,
}

pub struct SegmentEvidence {
    pub segment_index: u8,
    pub observed_value: u32,
    pub poll_timestamp_utc: Option<DateTime<Utc>>,
}

pub enum UnclassifiedGoalState {
    Pending,
    Permanent,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct UnclassifiedGoalId(pub u64);

pub enum GoalAttribution {
    Scored { scoring_side: Side, scorer: PlayerRef },
    OwnGoal { conceding_side: Side, player: PlayerRef },
}

pub struct UnclassifiedGoal {
    pub id: UnclassifiedGoalId,
    pub attribution: GoalAttribution,
    pub original_clock: Option<Clock>,
    pub observed_segments: Vec<SegmentEvidence>,
    pub observed_order: Option<u64>,
    pub state: UnclassifiedGoalState,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct PlayerKey {
    pub side: Side,
    pub player_id: u32,
}

pub enum DismissalKind {
    SecondYellow,
    StraightRed,
}

pub struct DisciplineState {
    pub yellow_count: u8,
    pub dismissal: Option<DismissalKind>,
}

pub struct ParticipationState {
    pub roster_index: u8,
    pub has_appeared: bool,
    pub currently_on_pitch: bool,
}

pub struct Score {
    pub regulation: SideScore,
    pub shootout: Option<SideScore>,
    pub unclassified: SideScore,
    pub classification_complete: bool,
}

pub enum GoalKind { OpenPlay, Penalty, Shootout, Unknown }
pub enum CardKind { Yellow, SecondYellow, StraightRed }
```

`PlayerKey` is ordered by side and stable game player ID; display names and mutable roster
metadata are not map keys. In match-file and feed-wire JSON projections, `PlayerKey`-keyed
maps are encoded as arrays of records sorted by `(side, player_id)`, not as JSON objects.
Each record contains an explicit `player: PlayerKey` and `value`; readers reject duplicate
keys. `DisciplineState.dismissal` uses `DismissalKind`, keeping the non-dismissal
`CardKind::Yellow` out of that field. Exact substitution times remain in the event timeline
rather than being duplicated in `ParticipationState`.

`ScoreReconcileReason::UnclassifiedGoalObserved` creates a new live-observed uncertain
record. `ClassificationBecamePermanent` changes an unresolved record from `Pending` to
`Permanent` without assigning a score domain. `ClassificationResolved` is used for every
classification that must be reconciliation-only: the automation boundary has closed, no
valid monotonic anchor exists, or the resolved attribution/domain has no automation-eligible
feed event. Restoration itself publishes no reliable score-reconciliation event:
`SnapshotReplay` is sufficient to install the saved canonical state.

`next_unclassified_goal_id` starts at 0. Allocation assigns its current value, advances it
with checked arithmetic, and never reuses an ID within the `match_id`, including for records
materialized from recovery. Live-observed unclassified deltas
set `original_clock` and `observed_order`. Records materialized from recovered aggregate
counters leave either field absent when the original minute or ordering is unknowable; such
records never become eligible for one-shot automation. `Score.unclassified` always projects
each record onto its benefiting/scoring side; for `GoalAttribution::OwnGoal`, that is the
side opposite `conceding_side`.

The broker validates provenance against the semantic origin of each event; combinations
outside this table are rejected:

| Event category | Allowed provenance |
|----------------|--------------------|
| Direct live stat or table observations | `LiveObserved` |
| Reducer correlations, clock/lifecycle derivation, amendments, and reconciliation | `LiveDerived` |
| Finish, Abandon, user Stop, and profile release commands | `UserAction` |
| Recorded-minute journal notifications recovered after re-attach | `RecoveredHistorical` |
| Subscriber-local `Snapshot` initialization only | `SnapshotReplay` |

Each `FeedEvent` variant's automation eligibility and embedded snapshot payload:

| Event | Embeds MatchSnapshot in FeedEvent? | One-shot automation? | Score authority effect |
|-------|-------------------|----------------------|------------------------|
| `Snapshot` | Yes (replay) | No | Reflects current `score_authority` |
| `TeamsDetected` | No | No | No authority (setup only) |
| `MatchLive` | Yes | No | Acquires |
| `Goal` | No | Yes (if live provenance) | — |
| `GoalAmended` | No | No (presentation only) | — |
| `ScoreReconciled` | Yes | No | — |
| `OwnGoal` | No | Yes (if live provenance) | — |
| `Card` | No | Yes (if live provenance) | — |
| `SubOn` | No | Yes (if live provenance) | — |
| `ClockStarted` | No | No | — |
| `ClockStopped` | No | No | — |
| `MatchSuspended { SourceLoss }` | No | No (pauses) | Retains |
| `MatchSuspended { UserStopped }` | No | No (cancels) | Releases |
| `MatchResumed` | Yes | No | Acquires |
| `MatchFinalized` | No | No (cancels) | Releases |
| `MatchAbandoned` | No | No (cancels) | Releases |
| `FeedReleased` | No | No (cancels) | Releases |

The "Embeds MatchSnapshot" column describes the `FeedEvent` payload only. Independently,
every retained-state transition supplies the broker with its post-transition
`PublishTransition.resulting_snapshot`.

`MatchSnapshot` fields and their persistence rules:

| Field | Type | When set | Serialized in match file | Preserved in recovery |
|-------|------|----------|--------------------------|-----------------------|
| `match_id` | `MatchId` | Match creation | Yes | Yes |
| `lifecycle` | `LifecycleState` | Every transition | Yes | Usually; a saved `Live` draft is normalized to `Suspended { reason: SourceLoss }` before restore |
| `rosters` | `Rosters` | Initial `TeamsDetected` and provisional team/roster corrections | Yes | Yes |
| `clock` | `Option<Clock>` | First `Live`, updated through match | Yes (latest) | Yes (latest) |
| `clock_motion` | `Option<ClockMotion>` | First `Live`, updated through match | Yes (latest) | Usually; a saved `Live` draft is normalized to `Unknown` unless continuity of the same running/stopped window can still be proved |
| `has_reached_live` | `bool` | First `MatchLive` | Yes | Yes |
| `score_authority` | `bool` | Authority transitions | Yes | Yes |
| `score` | `Score` | Initialized at match creation; updated by Goal/OwnGoal observations, recovered aggregates, and classification/reconciliation | Yes | Yes |
| `unclassified_goals` | `Vec<UnclassifiedGoal>` | While any attributed goal lacks a score domain; stable `id` identifies later resolution and `state` distinguishes retryable from permanent | Yes | Yes |
| `next_unclassified_goal_id` | `u64` | Initialized to 0 at match creation; checked increment whenever an unclassified record is allocated | Yes | Yes |
| `scorer_counts` | `BTreeMap<PlayerKey, u32>` | Classified non-shootout Goal observations, eligible recovered aggregates, and classification/reconciliation | Yes | Yes |
| `disciplinary` | `BTreeMap<PlayerKey, DisciplineState>` | Card observations and recovered aggregate/current-state reconciliation | Yes | Yes |
| `participation` | `BTreeMap<PlayerKey, ParticipationState>` | `SubOn`, position/participation observations, and recovered current-state reconciliation | Yes | Yes |
| `history_complete` | `bool` | Initialized provisionally at match creation; confirmed or set permanently false on first stats acquisition and later reacquisition according to the late-attach/missing-order-or-time rules | Yes | Yes |

Together with the explicit authority fields, snapshot replay alone is sufficient to
distinguish paused feed authority from fully manual behavior without inferring it from
clock presence. Clock and clock motion are `None` until first `Live`; after that the
snapshot retains the most recently valid values through suspension and finalization.
`ClockMotion::Stopped.stop_epoch` remains unchanged across reacquisition only when
unchanged ticks and reducer state prove continuity of the same stoppage. Any observed or
potentially unobserved progression changes the epoch or sets motion to `Unknown`, which
cancels pending one-shot automation on reconciliation. Consumers associate pending
post-goal automation with the `stop_epoch` carried by `ClockStopped`; `MatchResumed` may
preserve that pending action only when its snapshot reports `ClockMotion::Stopped` with
the same epoch. Substitution history carrying a recorded match minute remains in the event
timeline, while the snapshot carries enough current state to reconcile without replaying
it. `SideScore` carries home and away totals.

A new subscriber receives a `Snapshot` with `SnapshotReplay` provenance only when an
active or retained match state exists; otherwise it starts in an explicit local `Idle`
state and waits for `TeamsDetected`. An existing subscriber also receives its own local
replay snapshot when `restore_retained` installs a recovery, as specified below.
`MatchAbandoned` and `FeedReleased` are published with `resulting_snapshot: None`:
subscribers registered before the broker transaction receive the event, while the same
transaction clears retained snapshot/watch state so later subscribers start `Idle`.
`FeedReleased` ends only the feed binding; it does not abandon an offline tracker draft.
For every snapshot-bearing message, `FeedMessage.match_id` must equal
`MatchSnapshot.match_id`.

The feed event table above marks which variants carry snapshots and which are
automation-eligible. Reconciliation includes idempotent setup such as loading missing team
exports from the snapshot rosters; "no one-shot automation" forbids inferred scoring,
audio, and event clips but not state initialization or missing-team lookup. A consumer
that did not receive the referenced `Goal` may insert or update its presentation-only
record from the complete `GoalAmended` payload, but must not score or trigger automation.
On attach or re-attach, the reducer first applies recovered records to canonical tracker
state, may publish `RecoveredHistorical` notifications for presentation/history, then
publishes `MatchLive` or `MatchResumed` with the authoritative resulting snapshot.
Consumers must not apply recovered notifications again to score, discipline, or
participation state.

**Provenance and sequencing.** The provenance table above defines allowed combinations.
Table-loss suspension, auto-detected finalization, and `ScoreReconciled` events are
`LiveDerived`; user stop, Finish, Abandon, and profile-change `FeedReleased` are
`UserAction`. Initial counter aggregates are carried only in snapshots and match files,
not synthesized into feed events — this prevents duplicate goalhorns after tracker
restart, table loss, or late subscription. Reliable sequences are scoped to `match_id`:
initial `TeamsDetected` for a new `match_id` receives sequence 1 and atomically replaces
the prior terminal/released match cursor; provisional team corrections retain the same
`match_id` and continue its sequence. Consumers deduplicate on `(match_id, Some(sequence))`
and always process a `None`-sequence replay snapshot, using its `through_sequence` only as
the reliable-event cursor.

- **In-process pub/sub**: the `studio` binary crate owns the feed and passes publish/subscribe
  handles to tools via constructor injection (`match_tracker::Tool::new(feed.publisher())`,
  `music_player::Tool::new(feed.subscribe())`), not through `ToolContext` — `studio_core`
  must stay minimal and cannot depend on `match_feed`. Subscribers drain a channel in
  their per-frame `tick()` — which the shell calls for **every** tool, not just the
  one whose view is on screen (see "Tool plugin interface" in the core plan) — so
  the music player reacts to goals while the streamer is looking at the tracker's
  stats table, and vice versa. `studio` constructs `MatchFeed::new(wake)` with the GUI-
  neutral callback specified in the core plan. The feed invokes it on every reliable
  publish, worker-state change, and replacement of the coalesced `ClockWatch` value;
  watch replacements remain rate-limited/coalesced, so they cannot cause an unbounded
  repaint rate. Timer-only work uses `ToolContext::request_repaint_after` from `tick`
(`plans/core/gui.md` "Event wiring"). The core plan's registry
  installs `wake` as an `Arc<dyn Fn() + Send + Sync>` after an `egui::Context` is
  available, so `match_feed` does not depend directly on egui.
- **Snapshot replay for late subscribers**: when retained match state exists, subscription
  atomically registers both lanes and captures the latest matching `ClockWatch` together
  with `(snapshot, through_sequence)`. It places an immutable copy of the watch captured
  under the broker lock into the subscriber's initialization queue after the replay
  snapshot. Reliable events published after that capture follow it in the reliable queue,
  so tool start order does not matter, no event can fall into the subscription race, and
  replay cannot be mistaken for a newly observed live transition. On each tick, consumers
  drain initialization and reliable events before sampling the latest coalesced watch. A
  current watch must not replace the captured initialization value or be processed ahead
  of intervening reliable clock transitions. `MatchSnapshot.clock` in a replay is the
  clock reflected at its reliable cursor; the watch lane is authoritative for the current
  high-frequency clock. A watch for another `match_id` is discarded. The replay snapshot
  carries `sequence: None` and does not allocate a new reliable-event sequence;
  `through_sequence` is the last `Some(sequence)` already reflected in its state. Every
  ordinarily published reliable event carries `Some(sequence)`. The reducer produces an
  ordered batch of feed transitions plus one prospective tracker state, without committing
  either. Each transition carries its `PublishRequest` and post-transition
  `MatchSnapshot`, if any. The broker validates and publishes the entire batch under one
  synchronization boundary, assigning sequences in order and enqueuing either all messages
  or none. Subscription cannot interleave with a batch. The broker applies each transition
  to prospective retained snapshot, cursor, and watch state, validates the complete chain,
  and commits that broker state only if every transition succeeds. It performs any required
  retained-watch clearing under the same synchronization boundary used by subscription.
  No public API may advance retained state or the reliable cursor independently; the
  publisher API is conceptually:

  ```rust
  pub struct PublishTransition {
      pub request: PublishRequest,
      pub resulting_snapshot: Option<Arc<MatchSnapshot>>,
  }

  pub struct AssignedSequence {
      pub match_id: MatchId,
      pub sequence: u64,
  }

  pub struct PreparedBatch { /* broker-owned reservation; opaque to publishers */ }

  publisher.publish_batch(
      transitions: Vec<PublishTransition>,
  ) -> Result<Vec<u64>, PublishError>;

  // Durable finalization/release paths separate fallible preparation from
  // infallible commit while retaining the broker transaction reservation.
  publisher.prepare_batch(
      transitions: Vec<PublishTransition>,
  ) -> Result<PreparedBatch, PublishError>;
  prepared.assigned_sequences() -> &[AssignedSequence]; // available to durable revisions
  prepared.commit() -> Vec<u64>;              // infallible; enqueues the reserved batch
  prepared.abort();                           // consumes no sequence

  // Convenience wrapper for a one-transition batch.
  publisher.publish(
      request: PublishRequest,
      resulting_snapshot: Option<Arc<MatchSnapshot>>,
  ) -> Result<u64, PublishError>;
  ```

  `publisher.publish_batch` and its single-event `publish` wrapper reject
  `FeedEvent::Snapshot` and `Provenance::SnapshotReplay`; those values are created only by
  subscription and `restore_retained`. Every publisher-originated message receives
  `Some(sequence)`. The broker validates allowed provenance/event combinations, including
  `RecoveredHistorical` being presentation/history-only and `UserAction` being used only
  for explicit user commands.

  Finalization and feed release first prepare and reserve the complete broker batch.
  Preparation performs every fallible validation and sequence-allocation check without
  exposing messages and makes the reserved resulting sequences available for durable
  revisions. A durable revision records the reserved sequence belonging to its own
  `match_id`; a boundary-finalized old-match file never records the replacement match's
  sequence 1. The prepared reservation excludes reliable publication, subscription,
  restore, and retained/coalesced `ClockWatch` replacement until commit or abort. While the
  reservation is held, the tracker durably writes or converts the required file state. A
  file failure aborts the reservation without consuming sequences. After durability
  succeeds, broker commit is infallible and enqueues the reserved batch, after which the
  tracker commits its prospective in-memory state. No authoritative file mutation occurs
  before a potentially failing `PublishError`.

  A successful batch commits the prospective broker state — retained snapshot, reliable
  cursor, and watch — and returns the assigned sequences. Only after the broker accepts the
  complete batch does the tracker commit its own prospective reducer state. The tracker
  stores the sequence returned for a published `Goal`; any later
  `GoalAmended.goal_sequence` uses that value. On `PublishError`, the prior tracker,
  broker, and authoritative file states remain unchanged, no sequence is consumed, and no
  subscriber observes any part of the batch. Invariant or sequence-overflow errors are
  fatal internal feed errors: live reduction suspends and reports the failure rather than
  continuing from divergent state.

  For snapshot-bearing events, the embedded snapshot must equal the retained resulting
  snapshot; `MatchFinalized.score` must likewise equal the resulting snapshot's score. All
  event fields duplicated in retained state must agree with the transition snapshots:
  `TeamsDetected.home`/`away` equal `resulting_snapshot.rosters`; `ClockStarted.clock`
  equals the resulting clock and the resulting motion is `Running`; `ClockStopped.clock`,
  `reason`, and `stop_epoch` equal the resulting clock and `ClockMotion::Stopped`; and the
  lifecycle `last_clock` fields equal the resulting snapshot's latest clock. A non-`None`
  `resolves_unclassified` must identify a matching prior record with the same attribution;
  that exact record must be absent from the resulting snapshot, with the appropriate score
  and scorer projections updated exactly once. Ordinary `LiveObserved` or `LiveDerived`
  game events (`Goal`, `GoalAmended`, `OwnGoal`, `Card`, `SubOn`, `ClockStarted`, and
  `ClockStopped`) require prior lifecycle `Live`; snapshot-bearing `ScoreReconciled` follows
  its reason-specific reconciliation and finalization rules instead.
  `RecoveredHistorical` journal notifications are the exception: they may be published
  during recovery or re-attach before the resulting `MatchLive` or `MatchResumed`, remain
  presentation/history-only, and do not require prior `Live`.

  The broker also validates legal prior-state and `match_id` transitions plus lifecycle/result-
  state invariants for every reliable transition: `TeamsDetected` retains
  `TeamsDetected`; `MatchLive` and `MatchResumed` retain `Live`; `MatchResumed` is legal
  only from `Suspended` when the prior snapshot has `has_reached_live == true`;
  `MatchSuspended.reason` exactly matches `LifecycleState::Suspended.reason`;
  `MatchFinalized` retains `Finalized`; and `MatchAbandoned` or `FeedReleased` retains no
  snapshot or watch state. A suspended draft that has never reached live returns to
  provisional acquisition and uses `MatchLive` for its first valid live snapshot. No
  retained snapshot may represent no-draft or abandonment state. The broker also validates
  that `Score.unclassified` equals the benefiting/scoring-side projection of
  `unclassified_goals`, that every `UnclassifiedGoal.id` is strictly less than
  `next_unclassified_goal_id`, that IDs are unique within the match, that the cursor never
  decreases for a `match_id`, and that
  `Score.classification_complete == unclassified_goals.is_empty()`. Every key in
  `scorer_counts`, `disciplinary`, and `participation` must correspond to a player in the
  same-side snapshot roster, and each `ParticipationState.roster_index` must identify that
  same roster entry. Every `PlayerRef` must identify the same player ID at its side's
  `roster_index`. For `Goal` and `GoalAmended`, scorer and any present assist sides equal
  the event side; for `OwnGoal`, the player side equals `conceding_side`; for `Card` and
  `SubOn`, all player references equal the event side.
  `GoalAttribution::Scored.scorer.side` equals `scoring_side`, and
  `GoalAttribution::OwnGoal.player.side` equals `conceding_side`. Any mismatch returns
  `PublishError` without consuming a sequence. The broker also validates snapshot authority
  and clock invariants:
  `score_authority` implies `has_reached_live`;
  `TeamsDetected` has both fields false and
  no clock motion; `Live` has both fields true; source-loss `Suspended` has
  `score_authority == has_reached_live`; `Suspended { reason: UserStopped }` and
  `Finalized` have `score_authority == false`; and clock motion is absent exactly when no
  valid live clock has yet been observed. `MatchLive` and `MatchResumed` must embed snapshots
  satisfying the `Live` invariants. Mismatches return `PublishError` without consuming a
  sequence.

  Before calling recovery restore, an eligible saved draft whose lifecycle was `Live` is
  materialized as a recovery revision with
  `LifecycleState::Suspended { reason: SourceLoss }`. It preserves `has_reached_live`, score
  authority, score, and the latest clock; `clock_motion` becomes
  `Some(ClockMotion::Unknown)` unless continuity can still be proved. The normalized
  recovery snapshot is installed in the broker, and its next successful acquisition
  publishes `MatchResumed`. A provisional draft that has never
  reached live still uses `MatchLive` for its first valid live snapshot.

  Recovery uses
  `publisher.restore_retained(snapshot, through_sequence) -> Result<(), RestoreError>`.
  It succeeds when the broker has no retained match state or cursor. A
  same-match restore is accepted only as an exact idempotent replay with an identical
  snapshot and cursor; it may never decrease the cursor or replace newer retained state.
  The restored cursor must permit checked allocation of a subsequent sequence, so
  `u64::MAX` is rejected. A successful restore atomically installs the snapshot and cursor,
  delivers a subscriber-local `SnapshotReplay` (`sequence: None`) to every existing
  subscriber, and wakes them, but allocates no reliable sequence and triggers no
  automation. Subsequent recovered notifications and `MatchResumed` sequences start
  strictly above the restored cursor.
- **`Clock` carries time statistics**: every time-bearing match event carries phase plus
  optional consumer-ready conventional `display_minute`, seconds, injury time, and UTC
  audit timestamp fields. `display_minute` is `Some` for timed regulation and extra-time
  phases and `None` during a shootout. `Clock.seconds` and `InjuryTime.seconds` are `Some`
  only when sub-minute precision was observed or reconstructed. Recorded-minute journal
  events leave them `None`; consumers must not interpret `None` as zero. Live observations
  additionally carry an in-process `observation_instant`. The Music player computes
  deadlines from that monotonic instant;
  UTC is used only for display, files, and wire output. The process-local
  `observation_instant` is never persisted or serialized. JSON-lines output serializes a
  wire projection, not the in-process structures directly. Conversion replaces
  `observation_instant` with `observation_age_ms` at line-emission time; the field is `None`
  exactly when no process-local anchor exists. Non-time-bearing lifecycle messages such as
  initial roster detection carry neither timestamp. The reducer may retain a zero-based
  minute internally, exposed only if useful under an explicit name such as
  `raw_minute_index`. Consumers assign the optional `display_minute` directly and never
  perform the zero-to-one-based conversion.
  Injury time is represented separately from the regulation phase minute; shootouts use a
  distinct phase and have no fabricated regulation minute. Recovered aggregates have no
  wall-clock timestamp; recovered historical records carry their recorded game minute
  but use no wall-clock timestamp unless their source actually supplies one. `PlayerRef`
  combines the stable game player ID with side and roster index, using the name only as
  display/fallback data, so references survive snapshot replay. This feeds the music
  player's `time` conditions without prompting.
- **Two transport lanes**: ordinarily published reliable lifecycle and game events use
  `FeedMessage.sequence = Some(...)` and an ordered, unbounded queue per live subscriber;
  this lane never drops events, and dropping a subscriber releases its queue immediately.
  A disconnected/recreated subscriber initializes from a snapshot and never replays
  missed one-shot automation. Clock updates use the separate coalesced
  `ClockWatch { match_id, revision, clock }` channel; its revision is monotonic within one
  `match_id`, resets for a new match, and does not consume reliable-event sequence
  numbers. Consumers accept a watch value only when its `match_id` matches their active
  snapshot. The broker clears the retained watch in the same reliable transition
  transaction as `MatchAbandoned`, `FeedReleased`, or a new match's `TeamsDetected`;
  suspension retains the last value but publishes no advancing revisions. The bounded,
  lag-dropping watch lane
  prevents high-frequency ticks from accumulating in a subscriber's queue.
- **No built-in pipe/socket server**: the old named pipe's only consumer is gone.
  `UTW match-tracker watch` runs the tracker headlessly in that CLI process and emits
  its own JSON-lines stream; it does not attach to or expose a concurrently running GUI
  instance. Each line is a separately versioned feed-wire envelope
  (`{"feed_schema": {"major": 1, "minor": 0}, "message": ...}`). Feed schema 1.0
  normatively defines `WireEnvelope`, a tagged `WireMessage::{Reliable, ClockWatch}`, and
  wire projections of `FeedMessage`, `FeedEvent`, `MatchSnapshot`, and `UnclassifiedGoal`.
  Every recursively embedded `Clock` becomes `WireClock`; no in-process `Instant` or
  struct-keyed JSON map appears on the wire. The serde tag/content names and field casing
  are part of the major-version contract. External consumers therefore receive both
  transport lanes without conflating their sequence spaces. Major versions are compatibility
  boundaries. Minor versions may add only optional/defaulted fields whose omission does not
  change existing semantics.
  Adding or changing a reliable message/event variant is a major-version change unless
  the wire format defines an explicit forward-compatible opaque-event mechanism;
  consumers must not silently skip unknown state-changing reliable events. This covers
  external integration without running a server inside the GUI app.

What the music player does with the feed — auto team loading, feed-driven scoring and
goalhorns, automatic goal minutes, the revival of the `.4ccm` `event` clips, and
the manual-override rules — is specified in the
[Music player plan's](../music_player.md) "Autopilot" section.

---
