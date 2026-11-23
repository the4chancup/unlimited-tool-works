# Unlimited Tool Works: Core spec: design guidelines

The suite's look in one list: the values that make a window recognisably the suite's, for
anyone drawing a new view, a draft or a page. Each value is set once in code, in
`crates/studio_core/src/theme.rs`, and once in the drafts, in the stylesheet of
`docs/prototypes/index.html` (its `:root` variables). The reasons behind each value, and
what was tried before it, are in the rows of `plans/core/gui.md` "Other UI components" this
file points at. A change to the look starts in the drafts' stylesheet
(`plans/core/gui.md` "Prototypes before implementation"), then lands here and in
`theme.rs`.

## Corners

One radius, 2 points, on everything drawn with corners: windows, dialogs, menus, dropdown
lists, tooltips, buttons and fields in every state, and the shapes the suite paints itself
(chips, badges, marks, frames). It reads as a rectangle whose corners are not sharp, never as
a rounded shape. Shapes round by nature keep their own: a radio button, a dot, the bottom
dock's grip. The captain's mark on the pitch keeps the game's radii.

The project's pages take the same radius: the maintainer's pages (`docs/maintainer/`), the
web pages (`docs/web/`) and the scripts that generate some of them, as CSS
`border-radius: 2px` and SVG `rx="2"`.

`scripts/gates/corner_radius.py` fails on any other radius, in the code and in the pages.
Scenario: ST-CORNER-01. Reasons: the "Corners" row.

## Control edges

Every live control draws a 1-point edge inside its box, its fill 30 lighter on the dark theme
(30 darker on the light one), so an empty box still reads as a control:

| Fill | Edge (dark theme) |
|---|---|
| The widgets' grey, 60 (`#3c3c3c`) | `#5a5a5a` |
| The selection blue, `#005c80` | `#1e7a9e` |
| The quiet grey, `#5a5a5a` | `#787878` |
| The A rating's green, `#129648` | `#30b466` |

- The selection blue never fills without its edge, on a control or a row.
- A toggle's unheld side has no fill: its edge is the panel 15 lighter, `#2a2a2a`, the
  quietest.
- A text box keeps its own 1-point border, the quiet line colour (60 dark, 190 light).
- A frameless button (a menu's item, a window's close) stays frameless, and a disabled
  control draws no edge.

Reasons: the "Control edges" row.

## Colours

The dark theme is the default, and the drafts are drawn in it alone. The light theme is
second-class: fewer looks are reviewed in it, but none of its text is left unreadable.

| Role | Dark | Light |
|---|---|---|
| Panel and windows | `#1b1b1b` | `#f0f0f0` |
| A list's or a field's ground | `#0a0a0a` | `#fdfdfd` (a text box) |
| An empty grid position | `#202020` | grey 234 |
| Control at rest, hovered, pressed | grey 60, 70, 55 | grey 225, 211, 232 |
| Text, quiet | `#8c8c8c` | grey 80 |
| A control's text | grey 180 | grey 60 |
| Titles and strong text | `#dcdcdc` | black |
| Selection, and its text | `#005c80`, `#c0deff` | egui's light selection |
| Link and Info | `#58a6ff` | `#0969da` |
| Success | `#3fb950` | `#1a7e37` |
| Warning | `#d29922` | `#956300` |
| Error | `#f85149` | `#d1242f` |
| A help search hit | `#2a3a1e` | `#a7db80` |

- The four notice colours are GitHub's (its Primer pairs), in both themes.
- A light value keeps its dark value's contrast with the panel when that contrast is under
  4.5 (a fill, a quiet grey); a text colour reads at 4.5 on the light panel, its hue kept.
- A colour drawn on a surface both themes share (the pitch, a fill under its own text
  colour) is the same in both.
- The cup's clover keeps its own colours, `#129648` and `#8cc642`.
- Every colour comes from egui's `Visuals` or from `theme::Palette`, never from a literal
  beside its use: `scripts/gates/colour_literals.py` fails on one.

Reasons: the "Dark/light theme" row.

## Type

| Text | Font and size |
|---|---|
| Body text and buttons | Roboto Flex 12.5 |
| Small text | Roboto Flex 11 |
| A page's heading | Roboto Flex 15 |
| A window's title | Roboto Flex 14, the strong colour, centred |
| Monospace | Hack 12 |
| The pitch's formation name, a status notice | Roboto Flex Semibold |

Every symbol the views draw (arrows, ticks, the warning sign, the clover) comes from the
suite's own symbols font, drawn at the text's weight; what a member types falls back to
egui's monochrome emoji fonts. Reasons: the "Dark/light theme" row.

## Spacing and sizes

- Controls are 18 points high (buttons, dropdowns, number boxes, list rows); a button is
  padded 4 across and 1 down.
- A panel's margin is 8; items stand 8 apart across and 3 down; a menu's margin is 6.
- In a form, fields leave 6 points between their boxes, stacked checkboxes 8.
- A dialog has 12 points of margin across and 8 down and is at least 360 wide; its title is
  centred, 12 above the body; its buttons are in one row, right-aligned, 10 under it.
- A dropdown's open list scrolls past 300 points.

Reasons: the "Modals" and "Form grids" rows.

## Surfaces

- Flat: no gradient but a scroll area's fade at its edges, no shadow but a dialog's, a
  floating window's and a popup's.
- A checked box and a chosen radio button are filled in the selection blue, the tick or dot
  in the selection's text colour.
- A disabled control is drawn at half opacity, its text darker still (`#5a5a5a` as seen on
  the dark panel).
