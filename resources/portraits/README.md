# Placeholder portraits

What the game shows for a player who has no portrait, which the Save editor's Tactics pitch
draws on a starter's marker whenever it cannot show his portrait (`plans/save_editor.md`
"Tactics editor", the pitch's look). They are embedded in the Studio binary, so the pitch draws
them with no PES folder set or reachable.

Each is the suite's own drawing of the game's `common/render/symbol/blank/player-dummy.png`, a
128 x 128 PNG the game changed twice: simple shapes in the game's colours and proportions, not
traced from it. The game's files are not shipped, since the suite ships Konami material only to
make files the games read. Each PNG is rendered from the SVG beside it by
`scripts/images/render_images.py`: edit the SVG, then run the script and commit both.

| File | Used for | Looks like |
|---|---|---|
| `player-dummy_15.png` | PES 2015 | a white silhouette on a light blue square, 3 pixels short of the bottom |
| `player-dummy_16.png` | PES 2016-2018 | a blue-grey face and shoulders, transparent around them |
| `player-dummy_19.png` | PES 2019-2021 | PES 2016's drawing in beige, 4 pixels lower |
