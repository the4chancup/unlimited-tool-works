# Unlimited Tool Works — Library crates plan: color_tools

Part of the [Library crates plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## `libs/color_tools`

One small crate for the suite's color work: kit-color extraction (pure logic, the
Phase 2 surface) plus the shared color-picker widget and the kit menu icon drawing
routines (Phase 8, when `egui` joins the crate). Splitting the widget into its own
crate isn't worth the boilerplate at this size; if the egui dependency ever bothers
a headless consumer, the widget module can move behind a feature flag.

### Dominant kit-color extraction

Kit colors (the two per-kit menu colors compiled into `UniColor.bin`) matter: they
drive the match UI scoreboard and the kit-selection color dots, which is what lets
teams pick non-clashing kits when a preview player's custom body hides the actual
kit. Managers often don't bother providing them, so the Team compiler derives them
from the kit's main texture when a kit folder has no `colors.txt` (see the Team
compiler plan's kit steps and `kit_colors_derived` message).

The extraction (`color_tools::kit::extract_kit_colors(rgba, width, height) -> Option<KitColors>`;
the caller decodes, so the crate stays pure logic with no image dependency):

1. **Decode** the kit's main texture (`kit.dds`) to RGBA via `dds_convert`, top
   mip only. That is the caller's step; the function takes the RGBA8 pixels.
2. **Sample fixed template regions.** The community kit template has a fixed UV
   layout (the same for every kit — the config's "shirt model" field does not
   affect it, see the Kit config editor plan). Measured on the PES 19 colored
   template sheet (`Kit_col_template_pes19.png`): the shirt, front and back, is the
   center band x 0.336–0.664, y 0.02–0.91 with the collar strip below it at
   y 0.92–0.95; the shorts are the two lower side panels x 0.021–0.31 and
   0.69–0.979, y 0.586–0.918; long sleeves sit above the shorts, short sleeves and
   socks beside the band. The lib constants are those zones inset to keep seams
   and neighbours out: **shirt** x 0.36–0.64, y 0.05–0.88; **shorts** x 0.04–0.29
   and 0.71–0.96, y 0.61–0.90. Pixels with alpha below 128 are skipped; sampling
   every 4th pixel in both axes (16k samples per region on a 2048 texture) is
   plenty for a two-color answer.
3. **Cluster** each region's samples: quantize to 5 bits per channel, sort the
   bins by count, then merge each bin into the first larger cluster whose mean
   lies within RGB distance 24 (a greedy, deterministic merge); the result is the
   ranked list of `(mean color, share)`. Sponsor logos and badges end up in small
   clusters and lose automatically.
4. **Pick**, with "distinct" meaning RGB distance at least 60 (about navy against
   black), the same plain metric as the merge so the crate has one notion of
   color distance:
   - Color 1 = the shirt region's largest cluster.
   - Color 2 = the first distinct candidate in this order: the shirt's second
     cluster if it holds at least 25% of the region (a two-tone kit); the shorts'
     largest cluster; the shirt's second cluster if it holds at least 10% (a
     trim color, when the shorts match the shirt); the shorts' second cluster.
     When nothing is distinct (a one-color kit) color 2 is the shorts' largest
     cluster, identical or near-identical to color 1, and the result says so.

Calibration evidence (134 texture/config pairs from the exports on the writing
machine, 2048 and 4096 DXT1/DXT5 kits, the harness in `scripts/provenance/calib`, which
predates the crate and carries its own clustering: it is the record of how the thresholds
were chosen, not a second implementation to keep in step): the
declared config colors are **not** a usable ground truth: only 55 of
133 declared shirt colors appear anywhere in the shirt region's top four clusters
(managers leave the template's colors or pick an accent), and among the two-tone
kits whose managers did declare the second shirt color, its share is 28–49% for
real two-tone kits and 0–19% for trims, which is where the 25% and 10% thresholds
sit. The check that stands is visual: swatch sheets of every kit next to its
extracted pair read right on every standard-template kit; the only wrong answers
are textures that are not on the template at all (custom kit models whose atlas is
mostly black), which no region choice can fix. The region constants themselves are pinned by
a test against the colored template sheet (`Kit_col_template_pes19.png` subsampled to
128x128, `tests/fixtures/`): the shirt region must contain only the sheet's two shirt reds
and the shorts regions only its two yellows, with the expected literals counted by a script,
not by the crate.

The same routine powers suggestion swatches in GUI tools, so it returns the full
ranked cluster list per region, not just the two winners.
