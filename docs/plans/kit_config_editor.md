# Unlimited Tool Works — Kit config editor plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

What exists of this is in `spec/kit_config_editor.md` "Unlimited Tool Works — Kit config editor plan".

The Kit config editor (`crates/tools/kit_config_editor`, tool id `kit-config-editor`,
sidebar label "Kit config editor") is the successor to **Kit Manager PES2016** by
GOALARG — the closed-source, long-abandoned tool used by the 4cc community to edit kit
config files, in its original form and in a hex-patched "PES2021" variant. As with
the Match tracker, the old name carries no recognition value and does not stay in
the label.

## GUI view

Two ways in, per the suite's export-first philosophy:

- **Open export folder** (primary): the `Kits/` subfolders appear as tabs
  (p1 … p9, g1), each titled with its folder name so a label (`p1 - Lakers`) shows; `all/` is
  not a tab. Each tab edits that kit's `config.toml` in place, and its texture-name
  section marks which of the five textures the kit inherits from `all/` (the derivation
  counts them exactly as the compiler does). The tool walks
  the selected source, supplies a canonical `vtree::ScopePath` listing and required
  small metadata to `aesthetics_export::parse_listing`, then consumes the resulting
  `ParsedAestheticsExport` plus kit-scoped validation—not a whole-export
  `ValidatedAestheticsExport`—so an unrelated roster/player error never prevents opening
  and repairing a kit. The shared crate still owns locating and
  validating the folders (see the [Team compiler plan](team_compiler/README.md)), so this
  tool never carries its own idea of the export layout. The same picker accepts a
  single loose file (`config.toml`, a game `.bin`, old-export configs).
- **Open UniformParameter bin** (secondary, via `uniparam`): lists the container's
  entries with a filter box; picking one edits it and saves back into the
  container — covers the cup keeper's "fix one team's config in the compiled bins"
  case without a compile round.

In the loose-file and container modes the five texture names (the `.bin`'s 16-byte
fields) are editable text, ASCII and at most 15 characters, refused at entry otherwise:
a compiled config naming the wrong texture is a kit that does not load, and these two
modes are where it gets fixed. In export mode they are derived from the kit folder and
only shown. A loose-file mode with nothing open shows the picker.

**Saving**, as in the suite's other editors: nothing is written until Save (Ctrl+S);
unsaved changes are the difference from the file as opened or last saved, marked on the
tab, and opening another file or closing the tool with unsaved changes asks to save,
discard or cancel. Export mode writes each changed kit's `config.toml` (and `colors.txt`
and its marker) through `toml_edit`; the loose-file mode writes its file, and the
container mode the container, through a temporary file renamed over it, so a failed write
leaves the old file whole and reports an error naming it. The loose-file mode also has
Save as, after which Save writes the new path. Not Kit Manager's model: its "not saved"
warning looked at whether a Save had happened since the load, not at edits, and its
Save as left later Saves on the original file.

The form groups the fields as the format does — Shirt (models, collars, sleeves,
tight, pattern), Colors (the five config fields, via the shared `color_tools`
picker — no more JCPicker subprocess and hex-paste dance), Name, Numbers
(back/chest/shorts), Badges. Constraint gating is inline: options requiring shirt
model 144/160 gray out with a tooltip instead of the old modal error boxes, and with the
name off (`show = false`) its shape, Y and size gray out the same way, and with
undershirt-only long sleeves (0x02 = 0xBB) the four long-sleeve badge positions do too, as
Kit Manager grays them: in PES 2016 the badge never draws on the undershirt's forearm (Edit
mode and a cup match), only at the shoulder of the short sleeve over it, a corner not
measured further for a tool not yet built. A numeric field
takes any value its bits hold, not Kit Manager's lists (which accept only a listed item,
and stock data exceeds them: collars up to 131 against its 106), and a value outside the
selected version's range shows that version's warning beside the field. An FPC
indicator chip shows when the config carries the FPC values, linking the concept
to the Team compiler's reconciliation rather than leaving it as four magic numbers.

**Kit menu colors**: in export-folder mode each tab also edits the kit folder's
`colors.txt` (the two `UniColor.bin` menu colors that drive the match-UI
scoreboard and the kit-selection color dots) and its optional `icon_<N>` marker
(the menu icon number, 0–23, carried in the empty file's name — written only when
it differs from the default 3, so the marker stays optional). Same picker, with its
suggestion swatches pre-loaded from `color_tools`' dominant-color extraction on
the kit's textures — the manager sees what the compiler would derive on its own
(its fallback when `colors.txt` is missing) and either accepts it with a click or
picks something better. Suggestions **preview live**: hovering a swatch (or
dragging inside the picker) temporarily applies the candidate color to everything
it tints — the menu-UI mockup and the icon gallery below — and reverts on
mouse-out, so a suggestion is judged in context before the click commits it. A
small **menu-UI mockup** sits next to the color fields, showing the two menu
colors as PES actually uses them: the kit-selection color-dot pair and a
scoreboard strip — not just bare swatches. The **icon is picked visually**: a gallery of all 24
kit icon patterns (`color_tools`' vector renditions — the `icon_<N>` space,
not the config's Shirt pattern field), each tinted live with the
kit's current menu colors — never a bare number. A hint notes that only PES
15/16 display the icon in game (the prematch gameplan screens); the tinted
pattern still doubles as a preview of the menu colors themselves. Loose-file and
UniformParameter modes have no kit folder, so the section only appears when
editing inside an export.

**Preview panel**: a schematic shirt/shorts/socks drawing (egui painter) filled
with the config colors, overlaying markers for name arc, number positions, and
badge positions; when editing inside an export, the kit's actual textures
(`kit.dds` and variants, decoded via `dds_convert`) are composited under the
markers. Positions are relative — the game's exact pixel mapping is unknown, and
the old tool's 51MB of prerendered bitmaps is neither reusable nor worth
recreating; the preview is for orientation, the game remains the ground truth.

Unknown bits get a collapsible **raw view** row (hex per unknown region) for
curious users — the raw material for decoding the remaining bits someday.

## Settings (tool section)

| Setting | Default | Notes |
|---------|---------|-------|
| Show preview | on | |
| Show raw unknown-bits row | off | |

## CLI

```
studio kit-config-editor convert <in> <out>   # .bin ↔ .toml, direction by extension
studio kit-config-editor check <path>         # validate configs in an export / folder / container
```

## Testing

What exists of this is in `spec/kit_config_editor.md` "Testing".

- **Manual**: edits made side-by-side in Kit Manager and this tool on the same file
  produce identical bytes, leaving out Kit Manager's own write defects: its Right long X
  sleeve badge field writes nothing, a Name shape edit clears the top bit of Name size
  (a size of 16–20 drops by 16), a Back number spacing edit clears 0x19's undecoded bits
  6–7, and its Short sleeves type control is never enabled.

## Improvements over Kit Manager (summary)

What exists of this is in `spec/kit_config_editor.md` "Improvements over Kit Manager (summary)".

3. **Whole-team editing** — all of an export's kits in tabs, plus direct
   UniformParameter container editing; the original opened one file at a time.
4. **Live preview from the kit's real textures** instead of 51MB of canned bitmaps.
5. **Suite integration** — global PES version drives the encoding; FPC values are
   shared constants with the Team compiler, not folklore numbers; built-in color
   pickers with texture-derived suggestions.
6. **Kit menu colors in the same place** — each kit's `colors.txt` (UniColor menu
   colors) and `icon_<N>` marker (menu icon, picked from a live-tinted visual gallery)
   are edited alongside its config, seeded by the same dominant-color extraction
   the compiler falls back on.

## Dropped features

- **Prerendered preview bitmaps** — replaced by the schematic + texture composite.
- **JCPicker integration** — replaced by the shared `color_tools` picker widget.

## Key decisions

What exists of this is in `spec/kit_config_editor.md` "Key decisions".

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Preview | Schematic + real-texture composite, relative positions only | Honest about not knowing the game's exact mapping; avoids shipping megabytes of prerendered assets |
| Kit menu colors | Export-folder mode also edits each kit's `colors.txt` + `icon_<N>` marker, with `color_tools` extraction as suggestions | One place for all per-kit menu data; shows the manager exactly what the compiler's fallback would pick |
| Extraction preview | Hover live-apply on suggestion swatches + a menu-UI mockup (color dots, scoreboard strip) | Auto-derived colors are judged in their in-game role before committing, not as isolated swatches |
