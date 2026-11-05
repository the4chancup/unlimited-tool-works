# Library parity: the attribution's classes

`attribute.py` gives every non-equal line of a library run's `parity.tsv` files one cause.
`red_parity` (`crates/tools/team_compiler/examples/red_parity.rs`, whose header defines each
line class) writes the lines; this table names the causes `attribute.py` returns, in the
order of their IDs. The rules that pick them are in `cause_of`, first match wins.

## Intentional differences (I)

The compiler's or the upgrader's documented behavior, where Red's output differs by design.
A line of one of these needs no action.

| ID | Difference |
|---|---|
| I1 | `*.fpk.xml` dropped: Red copies the source's, the Studio format has none |
| I2 | a relocated copy: one Red texture, a copy per wearer |
| I4 | `kitN` spelling of kit-dependent names |
| I5 | a Full export's kit placeholders and rebuilt bins |
| I6 | kit config names |
| I7 | the number atlas re-arranged |
| I7b | FTEX header canonical (type 0x9, depth 1), pixels equal |
| I8 | an uncompressed texture re-encoded |
| I9 | logos and portraits re-encoded |
| I10 | a model `face.xml` does not list is not packed |
| I10b | an `.mtl` no packed model uses is not packed |
| I11 | the pre-Fox referee layout |
| I12 | the template `face_diff` |
| I13 | a mask or an extra GK kit texture |
| I13b | a kit texture under a name the game never loads (`kit_texture_not_used`) |
| I14 | a model re-bound through the IR (`skeleton_retargeted`) |
| I14? | an FMDL differs, re-bound or paired: listed with the unattributed lines, checked by hand |
| I15 | the last two BC blocks (the smallest mips) |
| I18 | a referee without a face model gets no face |
| I19 | `face.xml` model names (same types and materials, each names its own file) |
| I19b | a generated `face.xml` lists `hair_high` as parts |
| I20 | a referee's modelless item folder: Red packs an empty package, ours none |
| I21 | a refs export's template output (bins, marker) and no referee portrait |
| I22 | a file the game never reads (`.txt`, `.png`): Red packs it, ours reports `file_not_used` |
| I23 | the hand auto-split moves a face model's hands into the gloves |
| I24 | FMDL texture names keep their letter case |
| I25 | a face folder Red discards for the other engine's file, ours ignores the file |
| I26 | a normal map re-encoded as DXT5nm BC3 |
| I27 | a PNG source converted to the `.dds` its material names |
| I28 | a Common file nothing references is not packed |
| I29 | a Midcup kit config from the template: the run installs nothing |
| I30 | a game-path face folder: Red generates its `face.xml`, ours keeps the member's |
| I31 | a kit texture the upgrade drops as a leftover: Red packs it |
| I32 | a modelless item folder: Red packs an empty package, ours marks the wearers `fpc_on` |
| I33 | an item folder with a stray FMDL: Red discards it, ours drops the file |
| I34 | a face whose Common path names the team (`common/o/`): Red discards it, ours migrates it |
| I35 | a pre-Fox Common model already named `oral_x`: Red packs it as `oral_oral_x`, ours adds the prefix once |

## Defect candidates (D)

Defects the library runs found and later steps fixed (6.16). A line in one of these is a
regression, diagnosed before the phase closes.

| ID | Defect |
|---|---|
| D1 | an empty face folder migrated as `ingame_face` |
| D2 | a Midcup colors-only kit compiled as a placeholder |
| D9 | a stray FMDL copy migrated into the player folder and merged as hair |

## The other causes

- `error: <codes>`: an Error finding in the line's owner (a player slot, a kit, a shared
  folder) dropped or replaced its content; the stricter-than-Red classes. Each export is listed.
- `warning-slot <class>: <codes>`: a Red entry missing from ours whose slot carries a Warning;
  listed with the unattributed lines.
- `container (its member lines)`: a CPK or FPK that differs because a member does; its member
  lines carry their own cause.
- `unattributed <class> <kind>`: no rule claims the line. Each one is diagnosed; a new
  intentional difference gets the next I number here and its rule in `cause_of`, citing the
  plan section or decision entry it follows.
