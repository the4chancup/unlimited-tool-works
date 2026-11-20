# Thumbnails

The pictures a written file carries in its container. They are embedded in the Studio binary.
PES 2017 lists every Team Export with its own stock picture, so `team_export.png` is never seen
in game.

| File | Used for |
|---|---|
| `team_export.png` | the thumbnail of every PES 2017 Team Export file the Save editor writes (`plans/pes_savefile/operations.md`, the Texport writer's container): 320 x 176, the size of the game's own |

`team_export.png` is rendered from `team_export.svg` by `scripts/images/render_images.py`:
the 4cc clover, its ring and leaves taken from the symbols font's
`resources/symbols/f100-4cc-clover-ring.svg` and `f101-4cc-clover-leaves.svg`, in the
sidebar's two greens (`#129648`, `#8cc642`), 136 across, centred on `#1f2527`. Edit the SVG,
then run the script and commit both.

The game's own thumbnail (PES 2017's logo over a Team Export shield, the same in every file the
game writes) is not shipped: the suite ships Konami material only to make files the games
read.
