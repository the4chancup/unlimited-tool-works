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

`icons.ttf` is the shell's icon font: `scripts/make_icons.py` builds it from one SVG per
icon in `resources/icons/` (its README says how they were drawn) and writes the drafts' copy
too; `scripts/make_icons_test.py` checks the committed files are its output. Each glyph sits
at the codepoint `studio_core`'s icon constants (`icons.rs`) name, and the font has no layout
tables, so text never turns into an icon. Its metrics (1024 units per em, ascent 960,
descent -64, every advance 1024) are Phosphor's, the font the rail was first measured in.
Six glyphs are Phosphor Icons' regular weight copied point for point (help, settings,
dismiss, warning, download and the Stadium compiler's buildings), and the Music player, the
Music export editor and the Team creator draw Phosphor's notes and figures; licence below.

## Licence of the Phosphor glyphs in `icons.ttf`

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
