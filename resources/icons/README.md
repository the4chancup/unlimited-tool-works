The tools' icons for the shell's icon font, one SVG per tool, named by the tool's id: each one
filled outline (`fill="currentColor"`, no stroke) on a 256 grid, y down, every point inside
16..240, the form a font glyph takes. The icon font build reads them as they are.

How they were drawn: on Phosphor regular's convention (16-unit strokes, round caps and joins),
then each icon's strokes outlined into one shape. A tool's kind is one mark in the bottom-right
corner: an editor's a pencil, a compiler's the open box under the Team compiler's figure, a
creator's or generator's a plus, the Refs arranger's a refresh sign. Each mark hides what lies
under it: the base minus the mark's shape widened by a 12-unit gap, then the mark. The running
figure (Team compiler, Player aesthetics editor) is a filled outline traced from the
maintainer's drawing. The Music player, the Music export editor and the Team creator take
Phosphor's music notes and three figures (MIT), the latter scaled into 16..240.

The Stadium compiler has no file: it keeps Phosphor's buildings glyph (U+E102) for now.
