# Unlimited Tool Works — Match tracker plan: GUI view

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

Part of the [Match tracker plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## GUI view

```
┌─────────────────────────────────────────────────────────────┐
│ [Current Match] [aaa vs bbb 14-02]                          │  ← match tabs
│─────────────────────────────────────────────────────────────│
│ Tracking: reading stats                            [Start/Stop] │  ← run strip
│           /aaa/  2 - 1  /bbb/            67:24 ⏸               │  ← score + clock
│ ┌────────────────────────────────┬────────────────────────────┐ │
│ │ Player       Goals Assists ... │ 23' GOAL: Player One       │ │
│ │ Player One     1      0        │       assist: P. Two       │ │
│ │ Player Two     0      1  [ heatmap ] │ 41' YELLOW: ...      │ │
│ │ ...                            │ 67' IN: Sub Guy            │ │
│ └────────────────────────────────┴────────────────────────────┘ │
│ [Columns…] [□ Benched] [□ QR] [Finish match] [Discard draft] │
│                                      [Open] [Save] [Export…] │
└─────────────────────────────────────────────────────────────┘
```

- **Stats table**: rows = players (home/away tints; benched players hidden unless
  toggled), columns = the stat catalog with a column picker (show/hide + order,
  persisted). Changed cells flash and fade back — the original's red-fade effect,
  done as a time-based color lerp in the update loop. Ctrl+C copies the selection as
  TSV. A **heatmap** button on each player row opens the heatmap viewer (available
  for live in-memory data and saved match files that carry heatmap grids).
- **Event log**: side-colored lines stamped with the reconstructed minute.
- **Score display**: unclassified totals are shown separately with a warning, for example
  `2–1 (+1 unclassified)`; they are never folded into regulation or shootout score.
- **Match tabs**: at most one feed-bound **Current Match**, plus offline drafts, finalized
  matches, and opened files; a finished match stays open as a tab. On a validated session
  boundary after `Live`, the old Current Match tab becomes a finished tab and a new Current
  Match tab is inserted only after the broker batch is prepared/reserved, the staged
  finalized revision is durable, and the infallible atomic
  `MatchFinalized`/`TeamsDetected` commit completes. If the durability write fails, the old
  tab remains the authoritative Current Match with a finalization-blocked status and the
  detected replacement-team state is retained for retry; new-session deltas are not applied
  to the old match. A team change while still provisional updates or replaces the existing
  Current Match draft without creating a finished empty tab. On
  a profile-change release, the old Current Match becomes an **Offline draft** tab labeled
  with its original game profile and receives no further live updates. After tracking
  restarts, the next successful team detection creates the new Current Match tab. Save,
  Finish, and Discard act on the selected tab, but
  Finish or Discard of an offline draft never publishes to `match_feed`.
- **Lifecycle controls**: **Finish match** finalizes the selected live, suspended, or
  offline draft after confirmation, but is unavailable until that draft has reached
  `Live`. **Discard draft** abandons the selected draft and is available from
  `TeamsDetected`, `Live`, and `Suspended`, including offline drafts. For a feed-bound
  draft it publishes `MatchAbandoned`; for an offline draft it removes only the local
  draft. **Stop tracking** only suspends acquisition; it does not finalize or discard
  match data.
- **QR sync** (opt-in): a small QR code of the current wall-clock milliseconds,
  regenerated at a capped cadence of approximately 30 Hz while enabled — the original's
  stream-latency measurement aid, kept via the `qrcode` crate.
- **Export…**: a small menu offering the wiki footballbox export (copies
  `{{Football box}}` wikitext to the clipboard or saves as `.wiki.txt`). It is enabled
  only when the match is finalized and `score.classification_complete` is true. If
  `history_complete` is false or required minutes are missing, the exporter presents a
  validation summary and either omits incomplete
  scorer/card details or requires explicit user confirmation or editing; it never
  invents minutes.

---
