# Unlimited Tool Works — Match tracker plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The match tracker (`crates/tools/match_tracker`, tool id `match-tracker`, sidebar
label "Match tracker" — unlike Rigdio/RigDJ the old name has been out of use for years
and does not stay in the label) is the successor to **SEN:P-AI**, the live
match-stats extractor used on stream alongside PES 2017.

This plan also specifies **`libs/match_feed`**, the small lib crate through which the
tracker publishes live match events to the rest of the suite — primarily to the
[Music player](../music_player.md), which uses the feed to run the soundboard
autonomously (its "Autopilot" section specifies that behavior). Platform context is
in the [core plan](../core/README.md).

Unlike every other tool in the suite, the tracker's primary input is not files but the
**memory of the running game process**. Live tracking is Windows-desktop-only (see
"Platform notes"); opening and viewing saved match files (`.match.json`) works on
all platforms.

---

The plan is split into parts; section headings are unchanged from the
single-file plan.

| Part | Covers |
|---|---|
| [Reading the game](reading_the_game.md) | Reading the game |
| [Match model](match_model.md) | Match model and event derivation |
| [Match feed](match_feed.md) | The match feed (`libs/match_feed`) |
| [GUI view](gui.md) | GUI view |
| [Match files](match_files.md) | Match files |
| [Testing](testing.md) | Testing |

## Background

SEN:P-AI (`Tools_4cc/SEN_P-AI`, C++/Qt, v1.2.6) attaches to a running PES 2017,
locates the in-memory team and stat tables, and turns raw stat changes into a live
match view: score, reconstructed game clock, an event log (goals, cards, subs), and a
full per-player stats table. It saves matches, including in-progress manual or timed
autosaves, as binary `.sen` files and broadcasts events as length-prefixed JSON over a
named pipe — the protocol Rigdio
v1.9–v1.10 consumed for automatic event clips, until Rigdio removed its client in
v1.11 (2020); the 1.3.0 rebuild keeps the pipe and posts the same events to the Rigdio
fork's local HTTP API. It has not been used on stream in a while, but it is the original
map of the game's in-memory match structures (the 1.3.0 rebuild's `docs/measurements/` is
the newer).

| Tool | Role | Location |
|------|------|----------|
| SEN:P-AI | Live PES17 match stat/event extraction (stream side) | `Tools_4cc/SEN_P-AI` |
| SEN:P-AI 1.3.0 (independent rebuild) | The ATF bot's author's rebuild (2026-10-11): PES 2017/2019/2021 layouts measured on a live game, with the verification evidence, and events sent to the Rigdio fork's local API; surveyed 2026-10-13: its stoppage chant and its finding that anthems have no memory cue went into the Music player plan's "Autopilot", its pipe tap and HTTP output into "Background" and "Relationship to the old codebase" here | [`zocram4cc/SEN_P-AI`](https://github.com/zocram4cc/SEN_P-AI), local copy `Tools_Marco/SEN_P-AI` |

### Unofficial fork (unverified findings)

A community-made unofficial update of SEN:P-AI exists at `Tools_4cc/SEN_P-AI-Unofficial`.
Below and in this plan's parts, "the fork" without a qualifier means this one; the ATF
bot author's is "the 1.3.0 rebuild".
Its quality and stability are not trusted, so it is **not** an architectural reference —
only a source of reverse-engineered game knowledge to mine and then verify independently.
The findings below are attributed to it and marked **(unverified)** until confirmed against
a live game; they are collected in the "Verification" section under Testing. What the fork
adds over the original, relevant to this plan:

- **Partial PES 2021 profile**: a substantially mapped second game version — different
  process name (`PES2021.exe`, 64-bit), a pointer-chain stat-table locator, an 8500-byte
  stat entry, indicator-based entry validation, and player/team heatmap offsets. The fork
  declares a 269-slot stat catalog, but only 263 slots are explicitly initialized and the
  table contains duplicate legacy `visualIndex` values and overlapping definitions. It is
  an internally inconsistent reverse-engineering lead, not a directly reusable schema.
  Most segmented counters use 9 buckets on both versions, while "Game Ticks Played" uses
  13 on both. The profile declares up to 40 slots per team, but the fork's operational
  reader still uses 32-slot structures (`StatTableReader.h` defines `MAX_PLAYERS` as 32
  and `teamInfo::player[32]`), so support above 32 remains unverified. PES21 possession
  remains an unresolved TODO, and clock start/stop derivation is not wired for PES21.
  **(unverified)**
- **Pointer-chain acquisition**: an ASLR-proof locator that follows a static-RVA pointer
  chain to the stat-table base. The fork wires it into the operational PES21 reader and
  defines a PES17 profile for it, but does not wire that profile into the operational
  PES17 reader, so the PES17 path remains pending verification. The original used an
  exact-region-size scan as its normal path and a manually triggered whole-RW-region
  content scan as its fallback; it had no pointer-chain fast path. **(unverified)**
- **Substitution journal (PES2021)**: a per-match record at a fixed offset from the
  stats-table base that names *both* the incoming and outgoing player and the exact
  minute — the original could only see the incoming player via first-minute-on-pitch.
  **(unverified)**
- **Direct clock read (PES2021)**: a pointer chain yielding the current segment and a raw
  tick count, converting directly to game minute + injury time — vs the original's
  mode-of-last-minute reconstruction. **(unverified)**
- **WebSocket remote ingest**: a push client to a remote server with a spectator share URL,
  alongside the original named pipe. Not adopted (see "External consumers"), but noted as a
  prior art data point for a future web-spectator feature.

Two things the fork carries from the original are also relevant here: the QR-timestamp
widget, and the **wiki footballbox formatting primitives** in `WikiFootballbox.h`.
`footballBox::toWikiText()` is a generic parameter-map serializer for fields such as
teams, score, date/time and goals; it is not a scorer/minute/card assembler. The original
only declared `Match::toWikiFootballBox()` and never implemented it. The fork adds a
separate partial match adapter for teams, score, goals, and own goals, but omits cards,
date, and time and does not use the generic `footballBox` builder. `WikiEditor`, the
wiki-API client that would round-trip output through the wiki, is unfinished in both.
The suite therefore implements a new, tested exporter informed by these primitives,
rather than porting a supposedly complete generator (see `match_files.md` "Match files").

**Priority.** The first implementation registers PES 2019, then PES 2021, then the other
versions, PES 2017 among them (the maintainer's order; "Key decisions", Game versions): the
cup plays 19 and 21, and both layouts are measured ("Independent measurements"). The
unofficial fork's PES21 claims (pointer chains, substitution journal, direct clock read)
stay inactive reference data until verified (see "Verifying source-derived findings"); the
measured layout registers without them. PES 2017 registers once its own open points are
closed: bucket 8 of its phase map, the fork's PES17 pointer chain (refuted on the measured
executable, so the scan-only path stands), and the heatmap viewer over its measured grid
(see "Game structures"); extra time stays `Phase::Unknown` on every version until recorded
and gates none of them. Heatmaps ship for a version once its grid is measured (PES 2017 and PES 2021
are; PES 2019's is not); the new footballbox exporter does not depend on the fork's heatmap
or PES21 findings.

### Independent measurements (PES 2017, PES 2019 and PES 2021)

A second, unrelated rebuild of SEN:P-AI, [`zocram4cc/SEN_P-AI`](https://github.com/zocram4cc/SEN_P-AI)
1.3.0, keeps the original's algorithm and adds PES 2019 and PES 2021 as data: a per-version
layout (`src/GameLayout.cpp`) and a stat catalog per version (`src/catalogs/`). Unlike the
unofficial fork's claims, every value was measured, and the measurement is written down so
it can be repeated
([`docs/measurements/`](https://github.com/zocram4cc/SEN_P-AI/tree/8b5bc8e/docs/measurements)):
regions found with a read-only scanner, each offset the same in two matches, each mapped
stat matched against the game's own Team Stats, Player Ratings or Match Events page in
both. Its acceptance runs
([`docs/verification/`](https://github.com/zocram4cc/SEN_P-AI/tree/8b5bc8e/docs/verification),
summary in `SUMMARY.md`) are out-of-sample matches whose events, scores, scorers, cards,
substitutions and Team Stats totals match the game's screens, and on PES 2017 it reproduces
the original's events and `.sen` statistics byte for byte. Later matches in fresh game
launches (`docs/verification/2026-10-11/`), one per version, sampled the stats blocks every
three seconds against the HUD clock and read the full-time pages: they give the bucket
boundaries, Player Rating, the heatmap grids' layout on PES 2017 and PES 2021, a check of the
unofficial fork's PES 2017 pointer chain, and the one layout value that did not survive a
relaunch, PES 2019's team table offset. The reader only reads: its process handle is
`PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`. The runs were on one member's cup installs
under Wine (Linux), against these executables (SHA-256):

| Game | Executable | FileVersion |
|---|---|---|
| PES 2017 | `4cd49eacd6edd1391eadca1e18bb8ba7092eef8447b928ebf82be381aa93a330` | 1.0.0.0 |
| PES 2019 | `06e980f0a8138b7cc8255c8100b93a33414ba7f309e74fc5368c80a23b4965a5` | 1.6.9.2 |
| PES 2021 | `d5faee3ba913463d373d320871e32ce9bc4d6db9b7ea22e53340dd4607e6ee76` | 1.1.9.3 |

The PES 2019 executable is the cup's own "PES2019 1.06 with 4cc Patch v3" build. The
maintainer's installed `PES2019.exe` is 1.6.9.4, which differs from it in 149 bytes with
the same PE timestamp and section layout, so the measured addresses are expected to hold
there; the first live run re-checks them.

What it settles and what it leaves open is recorded per claim under Testing, "Verifying
source-derived findings"; the measured values are in "Game structures". It reads the stats
table by scanning only, so the unofficial fork's PES 2021 pointer chains, substitution
journal and direct clock read stay unverified, and no match reached extra time, so buckets
6 and 7 are unobserved on every version.

### Relationship to the old codebase

As with the other tools, SEN:P-AI defines *what* the tracker must do — the memory
signatures, the event derivation rules, the clock reconstruction — and its structure
headers (`TeamDataTable.h`, `StatsInfo.cpp`) are reverse-engineering leads for a separately
documented, independently verified layout specification. It is not an architectural
reference:

- **Qt signal/slot coupling throughout**: the memory reader emits UI strings
  ("SEN:P-AI noticed the teams!") directly, the match/event logic lives inside a
  QWidget (`MatchReader`) and mutates table widgets as it derives events, and the
  clock state is spread across widget fields.
- **Global mutable stat catalog**: a 175-entry static array with static lookup maps,
  mutated in place by the column-selection dialog.
- **The `.sen` format**: a hand-rolled binary section format whose reader and writer
  are one templated function running in both directions via macros.
- **Half-finished features**: the footballbox primitives and unimplemented match
  adapter described above, and a pipe server whose only consumer on stream removed its
  client years ago (the 1.3.0 rebuild's live output is HTTP; its `tools/senpai_tap.c` reads
  the pipe only to verify).

**Licensing constraint.** Both SEN:P-AI repositories are GPLv3. Unlimited Tool Works uses an
independent reimplementation based on independently verified factual layouts and observable
behavior; the legacy source is a reverse-engineering reference, not a code source. Do not
copy or adapt the fork's rendering or reducer code. The core plan's future
project-wide licensing section must reflect the same constraint; this plan does not change
that file.

**Status:** pre-implementation architectural plan. The Rust crates and APIs shown below
are proposed contracts; legacy-source claims are evidence to verify, not implemented
Unlimited Tool Works behavior.

---

## Crate layout

The tracker follows the standard tool-crate skeleton (core plan, "Tool crates"). Its elaboration
exists for one reason: the plan promises that live tracking is *Windows-only behind a trait* while
everything else (match model, files, view, feed publishing) is portable — and that promise is a
placement rule, not a comment.

```
crates/tools/match_tracker/src/
├── lib.rs              # Tool — wiring only; disables Start tracking when no memory source exists
├── settings.rs         # poll rate, autosave, match files folder, footballbox export options
├── cli.rs              # watch / replay / export subcommands
├── messages.rs
├── memory/             # reading the game — the only platform-specific code in the crate
│   ├── mod.rs          #   MemorySource trait (find process, read region, enumerate RW regions)
│   ├── win32.rs        #   #[cfg(windows)] backend: process discovery, ReadProcessMemory, elevation diagnosis
│   ├── replay.rs       #   recorded-snapshot backend for tests and the replay CLI
│   └── profiles/       #   per-version signatures — data only
│       ├── mod.rs      #     VersionProfile: table sizes, strides, pointer chains, readiness flag
│       ├── pes17.rs
│       └── pes21.rs    #     (unverified entries marked as such in doc comments)
├── acquire/            # the acquisition ladder, source-agnostic
│   ├── mod.rs          #   ladder state machine: find process → teams → stats → diff; demotion/re-acquire
│   ├── scan.rs         #   size-filtered region scan, deep-scan escalation, content validation
│   └── chain.rs        #   pointer-chain fast path
├── model/              # what a match is
│   ├── mod.rs          #   Match, Team, PlayerStats, lifecycle (TeamsDetected → Live → Finalized)
│   ├── diff.rs         #   stat diffing → events
│   ├── clock.rs        #   clock reconstruction (ET, penalties)
│   └── boundary.rs     #   session-boundary detection and staged finalization
├── files.rs            # match file JSON save/load, autosave, recovery
├── publish.rs          # match_feed publisher: model changes → feed events (prepare/commit pattern)
└── view/
    ├── mod.rs          #   match tabs, Start/Stop, source state indicator
    ├── stats.rs        #   stats table
    ├── events.rs       #   event log
    └── heatmap.rs      #   heatmap / footballbox export view
```

Placement rules:

- **`memory/win32.rs` is the only `#[cfg(windows)]` in the crate**, and `windows`-crate imports
  appear nowhere else. A Wine `/proc` backend would be a sibling file implementing the same trait.
- **`acquire/` and `model/` never import `memory/win32`**; they take a `&dyn MemorySource`. This is
  what makes the replay backend a complete test harness: every ladder and model test runs on
  recorded snapshots with no game present.
- **`memory/profiles/` is data.** Offsets, strides, and chains live there with their verification
  status; `acquire/` reads them, never hardcodes them.
- **`publish.rs` is the only module that knows about `match_feed`**, so the Music player's autopilot
  contract has one place to look.
- The saved-match viewing path (`files.rs` + `view/`) has no dependency on `memory/` or `acquire/`,
  which is how it remains available on non-Windows and in Studio Web.

---

## Settings (tool section)

| Setting | Default | Notes |
|---------|---------|-------|
| Poll rate | 100ms | acquisition + diff cadence; bounded to a safe supported range |
| Autosave on match end | off | optional named autosave on finalization, not raw table loss; every finalization path first guarantees a durable safety/recovery file |
| Timed autosave + interval | off / 10s | interval is bounded; resets after a successful write |
| Autosave directory | platform documents directory / `Unlimited Tool Works/Matches` | user-selectable |
| Autosave filename pattern | `%HOME vs %AWAY %Y-%m-%d.match.json` | deterministic suffix on collision |
| Show benched by default | off | |
| QR sync widget | off | |

## CLI

```
studio match-tracker watch
    # independently attach and print versioned reliable-event/ClockWatch JSON lines
studio match-tracker dump-fixture <out> --duration <seconds>
    # debug/developer builds only: record RawTableSnapshot fixtures for replay tests
```

`watch` doubles as the external integration point: anything that wants live match
events (overlay scripts, third-party tools) runs this headless tracker session and pipes
its versioned JSON envelope (wire contract under `match_feed.md` "The match feed") instead of speaking a
bespoke protocol. `dump-fixture` records only the profile ID, monotonic
sample timing, and target-table bytes needed for `RawTableSnapshot` fixtures; it excludes
unrelated process memory.

---

## Platform notes

- **Live tracking is Windows-only**: process discovery and memory reading use Win32
  APIs. On other platforms (and in the WASM build, where process access can never
  exist) Start tracking is disabled with an explanatory tooltip; opening and viewing
  saved match files, heatmap viewing, and footballbox export remain available.
- PES running under Wine/Proton exposes its memory via `/proc/<pid>/mem`; a Linux
  backend behind the same memory-source trait is possible future work, not planned.
- Reading another process's memory can require elevation depending on how the game
  was launched. Access-denied errors use `libs/elevation` for diagnosis and optional
  elevated relaunch on Windows, and are reported distinctly from "process not found."
  The tracker otherwise follows the same operational constraint as the original.

---

## Improvements over SEN:P-AI (summary)

1. **Match logic decoupled from GUI and Win32** — pure model over
   `DecodedGameSnapshot` input, testable headless against recorded `RawTableSnapshot`
   fixtures.
2. **The feed replaces the pipe** — typed in-process events with snapshot replay, and
   the main consumer ships in the same binary: the Rigdio integration lost in 2020
   returns, stronger (autopilot instead of event clips only).
3. **Readable match records** (JSON) instead of a bespoke binary format.
4. **Per-version signature data** instead of hardcoded PES17 constants.
5. **Wiki footballbox export** — a new, tested match exporter informed by the old
   formatting primitives and the fork's partial adapter produces `{{Football box}}`
   wikitext directly from match files.
6. **Heatmaps** — if the PES17 grid is verified to populate with stable semantics,
   per-player position grids that the old tool processed only as a hidden summed scalar
   gain an independent GUI viewer implemented from verified 10x14 grid semantics;
   otherwise the PES17 viewer remains hidden.
7. **Suite integration**: settings, theme, logging, single binary — no separate Qt
   runtime.

## Dropped features

- **Named pipe server and the JSON event log file**: no surviving consumers; the
  `watch` CLI covers scripting and the match file covers the record.
- **`.sen` files**: dropped in both directions; old archives stay readable in old
  SEN:P-AI.

## Key decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Naming | `match_tracker` crate; label "Match tracker" — no "(SEN:P-AI)" suffix | Suite naming convention; unlike still-active Rigdio/RigDJ, the old name has been out of use for years and carries no recognition value |
| Music player interface | In-process `libs/match_feed`: typed pub/sub + snapshot replay | Both tools live in one binary, so the in-process boundary needs no socket or serialization; the independent `watch` CLI serializes the same public model separately |
| External consumers | No GUI server; `watch` runs an independent headless tracker and prints versioned JSON lines | The old pipe's only consumer is gone; a CLI stream covers scripts without inter-process GUI plumbing. A program that drives the Music player (the Rigdio fork's rigbot) joins the suite and calls it in-process, so the player serves no inbound API either (`core/development_plan.md` "Unscheduled tools") |
| Match files | New versioned JSON format only; `.sen` dropped | Readable, diffable, schema-evolvable; old archives keep old SEN:P-AI |
| Autonomy | Full, Co-pilot, Assist (highlight-only), and Off playback modes; modes do not transfer feed score authority, and manual playback override stays live | Specified in the Music player plan; tracker stop, feed release, abandonment, or finalization restores manual score accounting |
| Deep scan | Automatic throttled escalation; no "Search Harder" button | A plain chunked scan on the polling thread is cheap enough that the manual gate the original needed just becomes streamer homework |
| Game versions | Register verified signature sets only; PES 2019 first, then PES 2021, then the others (PES 2017 among them) | The maintainer's order, 2026-10-13: the cup plays 19 and 21, and both layouts are measured ("Independent measurements"), so the first release serves a cup match rather than the version the original was written for. On every version, extra time (buckets 6 and 7) stays `Phase::Unknown` until recorded, the unofficial fork's pointer chains, substitution journal and direct clock read stay unregistered until verified, and heatmaps register only where the grid is measured (PES 2017 and PES 2021, not PES 2019). PES 2017's open points (bucket 8, its refuted pointer chain) gate registering PES 2017, not the first release |
| Platform | Windows-only backend behind a memory-source trait | Win32 live tracking now; a Wine `/proc` backend stays possible; live memory access is impossible in WASM, while saved-match viewing remains supported |
