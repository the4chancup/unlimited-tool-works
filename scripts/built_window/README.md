# Built-window checks

Scripts for looking at a release build's own window, the way a member sees it: copy the build
into a portable folder with known settings, start it, drive it, capture it, close it. Each step
is one command, so the person (or agent) running the check looks at a capture before choosing
the next input. A headless test sees the widgets egui lays out; only the built window shows what
the GPU draws (a texture's sampling, a font's hinting, the window's own scaling).

| Script | Does |
|---|---|
| `monitors.py` | prints each monitor's rectangle, work area and DPI: run first, since the primary monitor and its scaling decide where the window opens and what a capture's pixels mean |
| `setup.py` | `dark` or `light`: copies `target/release/utw.exe` into the portable copy, its settings to the default window (1280 x 780) and the theme asked; refuses while the copy runs |
| `launch.py` | starts the portable copy detached, in its own folder |
| `gui.py` | `info`, `click`, `move`, `drag`, `wheel`, `keys`, `type`, `shot`/`wshot`, `burst`, `open`/`folder`/`cancel` for the app's own dialogs, `size`/`maximize`, `place`, `close` |

When the primary monitor is scaled (this machine's 4K runs at 200%), run `gui.py place` right
after `launch.py`: it moves the window onto a monitor at 100%, where `gui.py`'s points are the
capture's pixels. On the scaled monitor a click lands at half the point given.

Each script's docstring has its exact usage. The portable copy is the folder `UTW_APP` names,
`.tmp/0_0_1s4t_v2/app` by default (its `data/settings.toml` is the copy's own, so a check never
touches the user's settings). The window's title is "Unlimited Tool Works". In the default
window, Open at (88, 17) then the PES 2021 entry at (120, 168) opens the maintainer's real
save: a check only reads it and never presses Save.

A shipped build greys out the tools still listed in `CLI_ONLY` (`crates/studio/src/main.rs`):
the Compiler and the Export upgrader until their views land. A look at one of their windows
runs on a release build made in the `look_grid` worktree (beside the repository, under
`4cc-studio-temp`), checked out at the commit to look at, with the tool taken off that list
and the edit never committed. A build in a sidekick's worktree cannot open them.

## The rules `gui.py` keeps

Input goes only into the app's own window: every point is in client coordinates and asserted
inside the client rect. Clicks, drags, the wheel and keys are posted to the app's window as
window messages, so they need neither the foreground nor the real pointer, and no key can reach
another program (keys once sent through the real keyboard typed into the user's chat box). The
hover commands, which move the real pointer, bring the app to the front first and refuse a point
another window covers (a notification over the app once took a click meant for it). Captures use
PrintWindow, never a screen grab, so another window over the app does not end up in the
picture. `close` sends `WM_CLOSE`, as the window's close button does; the app is never killed.
A check that opens a save opens a copy, or opens it read-only and never saves: the user's saves
are backed up and restored by hash around any check that could write one.

A window narrower than the default 1280 points scales the whole UI down with it, so `size`
cannot make room tighter for a widget: everything shrinks together. To test a widget against
the window's edge, make its content longer (a long name, more rows), not the window smaller.

## Requirements

Windows, Python with `psutil` and `Pillow`, and a release build (`target/release/utw.exe`).
