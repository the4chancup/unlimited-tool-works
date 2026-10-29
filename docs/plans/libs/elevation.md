# Unlimited Tool Works — Library crates plan: elevation

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## `libs/elevation`

What exists of this is in `spec/libs/elevation.md` "`libs/elevation`".

- **Manifest execution level**: the `studio` binary ships `asInvoker`; a
  `requireAdministrator` cargo feature flips the manifest for always-elevated
  setups.
- The desktop updater can also need elevation for a protected program directory. The match
  tracker normally reads without elevation, but uses the same helpers when process
  permissions require it.
- The GUI's "run as administrator" action relaunches through `relaunch_elevated`. On POSIX
  the relaunch is `Unsupported` and the tool prints the sudo hint itself.
- Consumers: `team_compiler`, `stadium_compiler`, `balls_compiler`, `match_tracker` (when
  access requires it), and the desktop updater.

The relaunch itself is a manual check at the first tool phase that needs it.
