# Unlimited Tool Works — Match tracker plan: Reading the game

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

Part of the [Match tracker plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Reading the game

### Game structures

PES17 exposes two relevant memory-resident data sources. The roster/team-data table is
found by scanning committed private read-write regions of the known size and validating
its contents. The player-stat blocks use a version-profile pointer-chain fast path where
that chain has been verified, with validated region/content scanning as fallback (see
"Acquisition and polling"):

- **Team data table** (fixed offset inside a 384KB region): both teams' names, IDs,
  scoreboard abbreviations and banner texts, 32 player slots per side (validity
  marker, player ID, name, print name, height/weight, condition arrow), shirt
  numbers, manager name/ID, and the full tactics setup (presets, formations,
  advanced instructions, set-piece takers). A candidate is accepted only when each
  side has at least 11 entries with the correct home/away validity marker and
  team-consistent player IDs, and both team IDs and names are plausible.
- **Stats table** (home and away blocks at fixed offsets inside a 576KB region): one
  4784-byte entry per player carrying 175 stat fields. The identified ones include
  goals, own goals, assists, cards, the shot/pass/tackle breakdowns, saves,
  first/last minute on pitch, current position, game ticks played, player rating,
  and stamina; the rest are unidentified and are tracked anyway. Most counters are
  **segmented into 9 buckets**: buckets 0–2 are confirmed as the first half and 3–5 as
  the second half. The legacy code hypothesizes that 6–8 cover extra time and penalties,
  but their exact mapping is not registered on a version until verified from recorded
  extra-time and shootout matches on that version. Until then those combinations produce
  `Phase::Unknown`. A
  candidate is accepted only when
  each block has at least 11 consecutive entries with plausible player IDs and a threshold
  matching the corresponding detected roster/team. The original's `70100 ≤ id < 90300`
  heuristic is reference evidence, not Studio's acceptance range: the suite supports team IDs
  701–920, and roster membership is authoritative for validation. Home and
  away indicators or IDs must also agree with side assignment.

Signatures — process name, region sizes, offsets, entry layout, and the stat catalog —
are **per-game-version data, not code**. PES17's are known from the original; PES21's are
**claimed by the unofficial fork (unverified)** and add a substantially different layout:
8500-byte entries (vs 4784), a declared 269-slot catalog (vs 175) with only 263 explicit
initializers, duplicate `visualIndex` values, overlapping definitions, and an expanded set
of named stats at different offsets, a profile maximum of 40 player slots (vs 32; the
fork's operational 32-slot limit leaves slots 32–39 unverified), a 64-bit
process, and indicator-based entry validation (`[+0]=u16` 0xFFFD home / 0xFFFE away,
player ID at `+4`) instead of the ID-range check. Segmentation is per-stat, not
per-version: most counters have 9 phase buckets on both, and "Game Ticks Played" has 13
on both. Both versions define a 140×u32 field named "Play Area Grid" — PES17 at
entry+0x1034 and PES21 at entry+0x1938. These are candidate heatmap grids; spatial heatmap
semantics remain unverified as described below. The original mapped the PES17 grid and
processed it only through the generic stat path as a summed scalar column, hidden by
default; it never interpreted or rendered the 10x14 grid as a heatmap. The fork added the
reading and rendering code for PES21.
The PES21 stats-table base also exposes 8 per-team grids in the fork. The suite's global
PES version selector picks among registered, verified signature sets, and the tool
reports itself unavailable for versions without one; the first release registers PES 2019
and PES 2021 (the measured layouts below), and PES 2017 follows once its open points are
closed (README "Priority").

**PES 2019 and PES 2021, measured.** An independent rebuild of SEN:P-AI (zocram4cc's 1.3.0,
see the README's "Independent measurements") measured both versions on a live game, each
value repeated in two matches and checked against the game's own Team Stats, Player
Ratings and Match Events pages. These are measurements, not the fork's claims:

| | PES 2017 (original) | PES 2019 | PES 2021 |
|---|---|---|---|
| Process | `PES2017.exe` (32-bit) | `PES2019.exe` (64-bit) | `PES2021.exe` (64-bit) |
| Team region size, table offset | 0x60000, 0x538B4 | 0x50000, varies by launch (five offsets in five launches) | 0x50000, 0x35AA8 (two launches) |
| Team side stride; player entry stride | 0x520; 0xE8 | 0x5EC; 0x100 | 0x690; 0x188 |
| Player slots per side (physical) | 32 | 40 | 40 |
| Stats region size | 0x90000 | 0x190000 | 0x190000 |
| Home / away stat block offset | 0x35010 / 0x5DE40 | 0x7B5A4 / 0xD18EC | 0x7B5C4 / 0xD4C14 |
| Stat entry size | 4784 | 8176 | 8500 |
| u32×9 blocks before the tail; tail offset | 113; 0xFE8 | 168; 0x17A4 | 177; 0x18E8 |
| Mapped catalog entries | 175 | 711 | 719 |
| Game ticks per game minute (Match Time 10) | 320 | ≈356 | ≈371 |
| Player ID range (exclusive) | (70099, 90300) | (70099, 91100) | (70099, 91100) |

The entry keeps PES 2017's shape on both: a u32 player ID at +0, u32×9 counter blocks from
+0x4, then a tail whose fields sit at the same distances from its start as on PES 2017
(First/Last Minute on Pitch at +0, Game Ticks Played ×13 at +0x10, Player Rating at +0x48,
which matches the Player Ratings page for every player on both versions). The u32 before
each entry's player ID is its side marker, 0xFFFD home and 0xFFFE away, the same values the
team table's pairs use. The condition byte holds `0x10 | ordinal` (PES 2017: the top three
bits). On PES 2019 the team table's offset inside its region changes between launches while
its layout does not, so its profile finds the table by its pairs (marker and player IDs of
one team), as the deep scan does, rather than at a fixed offset; the stats blocks kept their
offsets. Neither game has Stamina, Tackles, Touches or the Ground/Aerial/Low/Chipped pass
families: those stats are absent from their profiles, not zero counters. In PES 2021's data
Saves can exceed Shots Faced On Target, and the game's own Team Stats page agrees, so no
profile takes that as an invariant.

### Acquisition and polling

- A background thread polls at a configurable rate (default 100ms) through the
  acquisition ladder *find process → find teams → find stats → diff stats*, demoting
  one rung when a read fails (game closed, tables reallocated, match exited) and
  re-acquiring automatically. The team table is re-read every tick. A validated
  team-identity change after the current match has reached `Live` is itself a positive
  session boundary and initiates staged boundary finalization: the complete
  old-`MatchFinalized`/new-`TeamsDetected` batch goes through the staged
  prepare/reserve → durable write → infallible commit pattern defined under `match_feed.md` "The match feed".
  On write failure the prior lifecycle remains
  authoritative, the UI marks the tab as finalization-blocked, acquisition does not apply
  the new session to the old match, and the detected replacement-team state is retained for
  retry. Before first
  acquisition reaches `Live`, team-identity changes replace the provisional
  `TeamsDetected` draft without finalizing an empty match. A same-team rematch additionally
  requires a readiness not-live→live cycle **plus** complete counter/clock reset and fresh
  progression, then uses
  the same staged boundary-finalization path. When the readiness flag is unavailable, the
  equivalent positive signal is a stat-table disappearance or reallocation followed by a
  complete clock/counter reset and fresh live
  progression; a reset observed in place without that session boundary suspends for
  confirmation rather than finalizing immediately. A table address change alone triggers
  reacquisition, not a new match.
- **Stat-table location — pointer chain first, scan as fallback** *(unverified for
  PES21; defined in the fork's PES17 profile but not wired into its operational reader,
  pending verification)*: the fork follows a static-RVA pointer chain
  (`moduleBase + rootRVA → … → stats-table base`) to a match/stat context from which
  home stat entry 0 is reached at `+0x3804` on PES17 or `+0x64D8` on PES21. This is
  ASLR-proof and far cheaper than region scanning; it does **not** locate the separate
  roster/team-data table. A readiness flag along the chain reports whether a match is
  live, so the ladder can short-circuit before kickoff. Pointer chains break across game
  patches, so the content-signature scan stays the fallback and the deep-scan escalation
  below still applies when the chain goes stale; the chain is just the fast path when it
  resolves.
- **Deep-scan escalation**: when the teams are found but the stats table is not (the
  pointer chain is stale, or PES occasionally allocates the table in a region that misses
  the size filter), the ladder escalates automatically: the cheap size-filtered scan
  keeps running every tick, and a deep scan searches every RW region for several known
  home-roster player IDs, validates entry stride and team consistency, and confirms the
  away block at the profile's relative offset. It must not depend solely on the
  historical `team_id * 100 + 1` assumption. The deep scan runs on a throttle (every
  few seconds) until the table turns up. The original gated
  this behind a manual **"Search Harder"** button because a full-memory scan was too
  costly to run continuously even SIMD-accelerated; throttled on the polling thread, a
  plain chunked scan is cheap enough to need no button (before kickoff, when the table
  genuinely doesn't exist yet, the extra scans are bounded idle work). The run strip
  shows when the deep scan is active. The deep scan uses a time/byte budget per slice
  with exponential backoff and cancellation, so it never blocks the 100 ms polling
  cadence; benchmarks on the intended streaming machine confirm the budget before
  release.
- The polling thread belongs to the tool, not its view — tracking continues while the
  streamer has the music player's view active. A Start/Stop tracking toggle controls
  the thread. Each tracking run owns a cancellation token and monotonically increasing
  generation ID. Stop or tool drop cancels and joins the worker before addresses or
  profile state are reused; messages from older generations are discarded.
- **PES version change while tracking is blocked**: changing the global version
  selector while the tracker holds live addresses and `DecodedGameSnapshot` state would
  interpret memory with the wrong pointer width or layout. The tracker exposes a
  version-change guard to the shell while tracking is active, via the
  `version_change_blocker` hook on `StudioTool` (see the core plan's "Tool plugin
  interface"). Stopping tracking suspends the current match and clears addresses before
  the new profile takes effect. If the PES version then changes while a suspended draft
  exists, that draft remains open under its original `game_profile_id` but is detached
  from live acquisition and cannot be resumed with the new profile. The
  `FeedReleased { reason: ProfileChanged }` transition goes through the staged
  prepare/reserve → durable write → infallible commit pattern defined under `match_feed.md` "The match feed";
  its durable-write step atomically converts any active recovery marker for the draft to
  `offline_only` and records the release reason. Commit publishes reliable `FeedReleased`
  for the old `match_id`
  and clears the retained snapshot and watch, after which the tracker commits the draft's
  offline binding state. The draft remains available locally as an offline draft.
  Saving, finalizing, or discarding that offline draft does not publish to `match_feed`;
  only the currently feed-bound draft may publish lifecycle or game events.
  `FeedReleased { reason: ProfileChanged }` uses `UserAction` provenance because it follows
  the explicit version-change command. Future release reasons, including tool shutdown and
  explicit release, are deferred until their operational paths are defined. The next Start
  begins a fresh acquisition run; the next successful team detection creates a new feed-
  bound `TeamsDetected` draft with a new `match_id`. Explicit Finish is the ordinary
  finalization path.

All Win32 access (`OpenProcess`/`VirtualQueryEx`/`ReadProcessMemory` via the
`windows` crate) is isolated in a memory-source trait, so the match logic runs
identically against a live process or recorded `RawTableSnapshot` fixtures (see
Testing).

---
