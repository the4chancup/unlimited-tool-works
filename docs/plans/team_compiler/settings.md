# Unlimited Tool Works — Team compiler plan: Settings

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Settings

### Common settings (studio_core)

What exists of this is in `spec/team_compiler/settings.md` "Common settings (studio_core)".

| Setting | Old default | Notes |
|---|---|---|
| `fpc_hide_boots_id` | — (new default 55) | The boots ID an `fpc_on` marker writes where the folder sets none (`plans/save_editor.md` "Full Player Customization (FPC)"); 1 to 100. A run with a value outside is refused as an invalid configuration (exit code 2), as `memory_cap_percent` is |
| `fpc_hide_gloves_id` | — (new default 11) | The same for gloves; 11 to 100 |
| `invert_value_scrolling` | — (new default off) | The wheel's steps over a value turned the other way, in every tool (`plans/core/gui.md` "The wheel over a value") |

### Team compiler settings

What exists of this is in `spec/team_compiler/settings.md` "Team compiler settings".

| Setting | Old default | New default | Controls |
|---|---|---|---|
| `run_pes` | 0 | same | Launch PES after compiling (only after a successful deployment; a degraded run never launches) |
| `quick_compile_close_on_success` | — | 0 | After a compile started by GUI autorun (`quick_compile.bat` → `UTW --gui team-compiler compile`), close the window when the run completes with no Error-level findings and nothing skipped for errors; stay open on errors, failed deployment, or cancellation so the grid/log can be reviewed. Warnings alone still close (they are in the logs, as Red's `pause_allow = 0` reasoned). Never applies to a manual Compile click |

### Path resolution

What exists of this is in `spec/team_compiler/settings.md` "Path resolution".

| Artifact/input | Base location |
|---|---|
| Relative `exports_folder_path` | Created when missing (from Phase 8 also at GUI launch, for the watcher) |

### CLI

What exists of this is in `spec/team_compiler/settings.md` "CLI".

The GUI autorun form extends the spec's command list:

```text
UTW --gui team-compiler compile [exports-root] [--mode normal|test|sideload]   # GUI autorun
```

The `--gui` form (the core plan's "Launch modes") is what `quick_compile.bat` runs. The tool's
`gui_run` implementation accepts `compile` only (`check` is meaningless in the GUI, where
checking is live). It queues the compile until the initial exports check has completed, then
triggers the same code path as the Compile button with the parsed `--mode`.
`quick_compile_close_on_success` governs what happens when that run ends.

`check` and `compile` take `--timestamps`, off by default, for profiling a run: each finding
line on stdout starts with the seconds since the run started, to the millisecond, when the
finding was made (`+12.345 - co Spring.zip: Warning …`).

A task's findings were made when the task finished processing on its worker, every other
finding when it was reported. A `duplicate_path` that an `overrides/` file raises on one of a
task's entries, found as its batch is written, is timed with that task's findings. It joins
them because splitting it out would carry a second list through the writer for one rare line
of a profiling aid.

The lines keep their order (findings are reported in manifest order, so the CPK and the log
read the same on every run, `pipeline.md` "3. Per-model-folder parallel steps (rayon)"), so
their times need not increase. A task that waited behind a slower one shows when it was really
done.

`--timestamps` is a flag rather than a setting, since it serves one run being looked at. The
GUI's log does not show the times.
