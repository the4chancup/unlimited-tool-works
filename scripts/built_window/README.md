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
| `gui.py` | `info`, `click`, `move`, `drag`, `wheel`, `keys`, `shot`/`wshot`, `burst`, `open`/`folder`/`cancel` for the app's own dialogs, `size`/`maximize`, `close` |

Each script's docstring has its exact usage. The portable copy is the folder `UTW_APP` names,
`.tmp/0_0_1s4t_v2/app` by default (its `data/settings.toml` is the copy's own, so a check never
touches the user's settings).

## The rules `gui.py` keeps

Input goes only into the app's own window: every point is in client coordinates and asserted
inside the client rect and with the app's window under it (a notification over the app once
took a click meant for it), and nothing is sent until the app's window is the foreground window:
keys sent while another window held the focus once typed into the user's chat box. Captures use
PrintWindow, never a screen grab, so another window over the app does not end up in the
picture. `close` sends `WM_CLOSE`, as the window's close button does; the app is never killed.
A check that opens a save opens a copy, or opens it read-only and never saves: the user's saves
are backed up and restored by hash around any check that could write one.

## Requirements

Windows, Python with `psutil` and `Pillow`, and a release build (`target/release/utw.exe`).
