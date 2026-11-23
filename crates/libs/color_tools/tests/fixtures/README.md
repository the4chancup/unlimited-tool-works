# color_tools fixtures

| File | Made by | What it exercises |
|---|---|---|
| `kit_col_template_pes19_128.rgba` | `scripts/provenance/fixtures/color_tools_template_fixture.py` from the community's `Kit_col_template_pes19.png` (the kit UV layout reference, every zone a flat color), 128x128 nearest-neighbour, raw RGBA8 (no header: `extract_kit_colors` takes pixels) | the region constants against the real template layout: the shirt region sees only the two shirt reds, the shorts region only the two shorts yellows; the expected literals were counted by the script, not by the crate |
| `Kiticons.png` | the community's PES 2015 sheet "Settings for gameplan kit icons", copied as it is | nothing reads it: the 24 shapes of `icon.rs`'s `PATTERNS` were transcribed from it, from pixel maps of each icon, and it is kept so they can be checked against it |

Both sheets are community assets, not game files.
