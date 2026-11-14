# Kit templates

Textures the Compiler fills a kit with when the export lacks them (the Compiler's plan,
`pipeline.md` "Kits", the placeholder kit). They are embedded in the Studio binary.

| File | Used for |
|---|---|
| `placeholder_kit.dds` | the `kit` texture of a placeholder kit (`kit_placeholder`): a kit folder whose effective textures lack `kit`, an empty one included |
| `kit_mask.dds` | the `kit_mask` texture of a kit compiled for PES 2015 to 2017 whose effective textures lack one (`pipeline.md` "Kits": "a pre-Fox target lacking a mask gets the template", silently, as Red's `kit_masks_check` copied it beside every main kit texture without a `_mask`) |

`kit_mask.dds` is Red's template, byte for byte: `Engines/templates/kit_mask.dds` of
`4cc-aet-compiler-red`, commit `3ab27d7` (2024-05-03, "Add missing kit_mask.dds"), SHA-256
`fa786fdec5ab05ef…`, identical to Blue's `lib/templates/kit_mask.dds`. It is 64 × 64 DXT1 with a
full mip chain of 7 levels, 2,872 bytes, and flat: every level-0 block is the same
(`color0` (255, 190, 0), `color1` (98, 101, 0), every index 3), so every pixel is their
one-third blend, about (150, 131, 0): a mask with no specular or roughness detail, the
default the community converters also write for a Fox kit taken to pre-Fox. Copied by the lead
at step 4.16 with `.tmp/copy_kit_mask_template.py`, which asserted the size and the hash.

`placeholder_kit.dds` is the magenta/black checkerboard, the "missing texture" pattern, so a
placeholder that gets rendered reads as exactly that. Written by the lead at step 3.9e (script
kept in the session scratch, `.tmp/make_placeholder_kit.py`), to this recipe:

- 64 × 64, DXT1 (FourCC `DXT1`, no alpha), a full mip chain of 7 levels (64 down to 1), 2,872
  bytes with the 128-byte header;
- level 0: 8-pixel checks, magenta (255, 0, 255) where `(x / 8 + y / 8)` is even, black
  elsewhere, so the top-left check is magenta;
- each further level the 2 × 2 average of the one above: checks of 4, 2 and 1 pixels, then the
  uniform average (127, 0, 127), stored as RGB565 (123, 0, 123);
- every 4 × 4 block holds at most two colors and uses indices 0 and 1 only, so DXT1 stores the
  image exactly, up to RGB565: `color0` is the block's brighter color and `color1` black
  (four-color mode), except an all-black block, whose `color0` and `color1` are both 0
  (three-color mode, index 0: black all the same).

Checked when written: every pixel of every level decoded back by the script, and level 0 by
Pillow's DDS decoder, matched the recipe.
