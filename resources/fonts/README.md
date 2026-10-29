# Fonts

`RobotoFlex-Regular.ttf` and `RobotoFlex-Semibold.ttf` are the suite's body font and its one
heavier face, for the Tactics pitch's formation name alone (the weight whose stroke matches
the game's label as egui draws it, `plans/save_editor.md` "The formation's name"): static
instances of Roboto Flex's variable file as Google Fonts ships it
(`github.com/google/fonts`, `ofl/robotoflex/RobotoFlex[GRAD,XOPQ,XTRA,YOPQ,YTAS,YTDE,YTFI,YTLC,YTUC,opsz,slnt,wdth,wght].ttf`,
1,787,292 bytes, SHA-256 `9b523f7d…`), cut by fontTools' instancer at optical size 12.5 (the
body text's size, the optical size a browser draws it at) and weight 400 and 600, every other
axis at its default; 96,352 and 97,076 bytes, SHA-256 `219b3036…` and `2cbd1950…`. The drafts
draw the same two files. Licence: the SIL Open Font License 1.1, `RobotoFlex-OFL.txt` beside
them.

`symbols.ttf` holds the symbols the views draw that the body font lacks (→, ≡, ✔, ✖ and the
like), each at its own codepoint: `scripts/make_symbols.py` builds it from one SVG per symbol in
`resources/symbols/`, on the body font's units per em and vertical metrics, and writes the
drafts' copy too; `scripts/make_symbols_test.py` checks the committed files are its output.
Each symbol's advance and ink box are those of the glyph the reviewed drafts drew it in (Segoe
UI's, or Segoe UI Symbol's, measured; no outline copied), but ↳, which is → with a stem from
its tail to the body font's cap height: Segoe UI Symbol's, half as wide, turns too short to
read at the body size; and ⏴ and ⏵, whose ink is Segoe UI Symbol's size centred in the advance,
since a stepper centres each in its own button. Licence: the repository's own.

`phosphor-subset.ttf` is the shell's icon font until the suite's own replaces it (worklog 8.14):
the 19 glyphs the suite draws, cut from the regular weight of Phosphor Icons, as the
`egui-phosphor` 0.14.0 crate ships it (`res/Phosphor.ttf`, SHA-256 `06b91e02…`). The cut
keeps each glyph at its Phosphor codepoint and drops the ligatures, so text never turns into
an icon. `studio_core`'s icon constants name the glyphs.

| Constant | Codepoint | | Constant | Codepoint |
|---|---|---|---|---|
| `ARROW_FAT_UP` | U+E52E | | `NOTE_PENCIL` | U+E34C |
| `BUILDINGS` | U+E102 | | `PLUS_CIRCLE` | U+E3D6 |
| `CROSSHAIR` | U+E1D6 | | `QUESTION` | U+E3E8 |
| `DATABASE` | U+E1DE | | `SCROLL` | U+EB7A |
| `DOWNLOAD_SIMPLE` | U+E20C | | `SMILEY` | U+E436 |
| `FLAG` | U+E244 | | `SOCCER_BALL` | U+E716 |
| `GEAR` | U+E270 | | `T_SHIRT` | U+E670 |
| `MUSIC_NOTE` | U+E33C | | `USERS_THREE` | U+E68E |
| `MUSIC_NOTES` | U+E340 | | `WARNING` | U+E4E0 |
| | | | `X` | U+E4F6 |

Built with fontTools 4.64.0's subsetter (no layout features, every name record kept, the
`.notdef` outline kept); the output is 7,344 bytes, SHA-256 `51be8e44…`, the same on every run.

## Licence

Phosphor Icons, MIT License:

```
MIT License

Copyright (c) 2020-2021 Phosphor Icons

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
