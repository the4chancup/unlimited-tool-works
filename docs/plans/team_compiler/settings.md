# Unlimited Tool Works — Team compiler plan: Settings

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Settings

### Common settings (studio_core)

What exists of this is in `spec/team_compiler/settings.md` "Common settings (studio_core)".

| Setting | Old default | Notes |
|---|---|---|
| `fpc_hide_boots_id` | — (new default 55) | The boots ID an `fpc_on` marker writes where the folder sets none (`plans/save_editor.md` "Full Player Customization (FPC)"); 1 to 100. A run with a value outside is refused as an invalid configuration (exit code 2), as `memory_cap_percent` is |
| `fpc_hide_gloves_id` | — (new default 11) | The same for gloves; 11 to 100 |

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
`gui_run` implementation accepts `compile` only — `check` is meaningless in the GUI, where checking
is live — and queues the compile until the initial exports check has completed, then triggers the
same code path as the Compile button with the parsed `--mode`. `quick_compile_close_on_success`
governs what happens when that run ends.
