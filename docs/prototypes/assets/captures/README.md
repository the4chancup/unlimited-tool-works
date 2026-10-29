# The built window's captures

The draft host's **Overlay** control (`plans/core/gui.md` "The host's other review aids")
lays one of these images over the draft at half opacity, or beside it, so a panel the app
draws elsewhere than its draft shows at once.

- **What:** captures of the built app's window, not of the drafts: its client area (no title
  bar or border), at the simulated window's size, the default 1280 × 780 client unless a file
  says otherwise.
- **Names:** `<tool>_<screen>.png`, the tool's id with underscores and the view's screen as the
  draft's link names it (`save_editor_player.png` for `#save_editor/player`); the overlay
  builds the name from the screen in view and shows the name it looked for when there is
  none.
- **Date:** each carries its capture date in its bottom-right corner, so a stale one shows.
- **When:** refreshed when a view's slice lands, by the lead's capture script on a release
  build.
- **Bundles:** a bundled draft (`scripts/bundle_drafts.py`) does not carry them; its overlay
  shows the missing-file line.
