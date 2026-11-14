// The Team compiler's draft (plans/team_compiler/gui.md), a view of the drafts'
// host page, index.html: the live-validating team grid (one row per export, a
// cell per player at fixed positions 01-23, 01-35 on the referee row, and a
// cell for its portraits and a cell per kit), the detail panel a cell opens, the toolbar (the run strip and
// the split Compile button with its output modes) and the log panel.
// An initial draft: the rows are samples built to show every cell state and
// every row action the plan names, not a recorded run. Team names and IDs are
// teams_list.txt's; /egg/ is a real team the bundled list does not hold.
// Marked "sample": wording and looks the plan does not fix, for the maintainer.
// Drawn only with what egui can draw (docs/plans/core/gui.md "Prototypes
// before implementation"). Widget mapping: a .tc-cell and a row's label =
// a rect the Painter fills, its text painted over it, Sense::click (the kit
// swatch two more rects; a blocked cell's dashed outline, dashed line
// shapes); the ID cell's <input> = a TextEdit; title="" = on_hover_text;
// .tc-detail = a Panel::right inside the tool's Ui; .tc-bar = the
// toolbar's Panel::bottom; .tc-split = two buttons with no gap, the arrow
// a menu_button; a .menu at the pointer = Response::context_menu;
// <input type=checkbox> = Checkbox; .log-panel = the shell's log widget.
// The wrapper keeps the view's names out of the host's scope; its body is left
// unindented, as in tools/save_editor.js.
// Names this script uses without declaring them (render, h, row, weak, strong,
// button, combo, cb, collapsing, window_, closeWindow, publishNotice,
// shellPesVersion, switchToTool, toolActivity, mmss, EXE_FOLDER, appendLog,
// logPanel, openHelp, openOutputStub, named, registerTool, CATALOG,
// COMPILE_TEAMS, closeMenus, inWin) are the host's, index.html.
(() => {
"use strict";

const TEAM_COMPILER_CSS = `
/* A file dragged over the grid: the area it lands in, outlined. */
.tc-top.tc-drop { outline: 2px dashed var(--sel-fg); outline-offset: -4px; }
  /* The view takes the central panel edge to edge (a pipeline tool's own
     panels fill the Ui it is given), so its two horizontal lines span the
     panel; what sits between them brings its own 8 points at the sides. */
  .center.tc-view { overflow: hidden; scrollbar-gutter: auto; padding: 0; }
  .tc-view .log-panel { margin: 0 8px 8px; }
  .tc-view .pipe { position: relative; }
  /* Every margin of the grid is 8: the panel's left and top, each side of a
     vertical line, between the players, the portraits and the kits. The
     table's own 2 between cells counts in each, so the paddings and the
     spacer cells bring the other 6 or 4. */
  .tc-top { flex: 1; min-height: 0; display: flex; padding: 0 8px 0 0; }
  /* The left margin is the scroll area's own padding: as the panel's it would
     put the area's clip 2 inside the compile column's checkboxes. */
  .tc-top .pipe-grid { flex: 1; padding-left: 6px; box-sizing: border-box; }
  /* The table keeps its own width whatever room the panel has (the detail
     panel open beside it squeezes no column: the grid scrolls sideways), and
     takes the panel's height: its last row, empty, stretches over
     what the exports leave, so the two lines reach the panel's foot. */
  .tc-grid { border-collapse: separate; border-spacing: 2px; height: 100%;
    width: max-content; }
  .tc-grid tr { height: 26px; }
  .tc-grid tr.fill { height: auto; }
  /* The title row: what each group of columns holds, in the weak text, kept
     in view while the rows scroll under it. It is solid from the legend's
     line down: every cell of it sticks, the two lines' cells too, each on
     the panel's fill and spread over the 2 between cells, and the panel's
     top margin is the row's own padding, so no row shows above or through it. */
  /* It sticks 2 down, where it already stands under the table's own spacing:
     at 0 the first scroll would pull it up by those 2. */
  .tc-grid tr.head > * { position: sticky; top: 2px; background: var(--panel); z-index: 1;
    box-shadow: 0 0 0 2px var(--panel); }
  .tc-grid tr.head th { padding: 6px 0 2px; font-weight: 400; color: #6e6e6e;
    text-align: left; white-space: nowrap; }
  .tc-grid tr.head th.mid { text-align: center; }
  .tc-grid tr.head th.mid > div { width: 50px; display: flex; justify-content: center; }
  .tc-grid tr.head .tc-line { z-index: 2; }
  .tc-grid td { padding: 0; white-space: nowrap; }
  /* A row that is off (disabled, or unticked for this run) is flat grey. */
  .tc-grid tr.off .tc-id { color: #6e6e6e; }
  /* 4 at the sides and an ID cell of 36: with the compile column shown, the widest row and its button
     then fit the default window to the point, and no sideways scroll bar shows. */
  /* The label is one width for every team; a name too long for it at the
     usual size is drawn smaller (nameSize). */
  .tc-name { width: 52px; min-width: 52px; max-width: 52px; padding: 0 4px !important; height: 26px;
    border-radius: 2px; cursor: pointer; position: relative; overflow: hidden; }
  /* A warning or an error the check found: a thick outline in its colour,
     inside the cell's edge, over the cell's own colour and a kit's swatch. */
  .tc-cell.ol::after, .tc-name.ol::after { content: ""; position: absolute; inset: 0;
    border-radius: 2px; box-shadow: inset 0 0 0 3px var(--ol); pointer-events: none; }
  .tc-name.hl, .tc-cell.hl { outline: 2px solid var(--sel-fg); outline-offset: 0; }
  .tc-id { width: 36px; text-align: center; height: 26px; border-radius: 2px;
    background: var(--widget); color: var(--strong); }
  .tc-id.bad { background: #f85149; color: #1b1b1b; }
  .tc-id input { width: 30px; box-sizing: border-box; text-align: center; }
  .tc-gap > div { width: 4px; }
  /* The compile column's checkbox, 4 from the panel's edge and 4 from the
     team's label: the panel's 6 and the table's 2 are already at its left. */
  .tc-grid td.act > input[type=checkbox] { display: block; margin: 0 2px 0 -4px; }
  /* A cell clips what it holds (the host's td rule); the box reaches 4 left of its own. */
  .tc-grid td.act { overflow: visible; }
  /* A vertical line down the whole panel (egui: one vline the Painter draws
     over the panel's height), 4 points clear of the cells on either side,
     unbroken across the 2px between two rows. */
  /* The cell's width is its div's: a bare td of 5 is squeezed to under 2. */
  .tc-line { position: relative; }
  .tc-line > div { width: 13px; }
  .tc-line::after { content: ""; position: absolute; left: 6px; top: -2px; bottom: -2px;
    width: 1px; background: var(--stroke); }
  .tc-slot.wide, .tc-cell.wide { width: 50px; }
  /* Every position is drawn, a faint square where the export has nothing: the
     full grid leads the eye along a row to its button. */
  .tc-slot { width: 26px; height: 26px; background: var(--faint); border-radius: 2px; }
  .tc-cell { width: 26px; height: 26px; border-radius: 2px; box-sizing: border-box;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    font-family: var(--font-mono); font-size: 11px; line-height: 11px; cursor: pointer;
    position: relative; overflow: hidden; }
  /* A compiling cell moves, as nothing else in the grid does: a light bar
     sweeps its bottom edge. It is painted from the clock each frame (egui:
     ctx.input(|i| i.time)), because the view is redrawn on every tick of a
     run, and above the Warning and Error rings (z-index over the ring's
     ::after; egui: painted after the ring). */
  .tc-sweep { position: absolute; bottom: 0; height: 2px; width: 40%; background: var(--strong);
    border-radius: 1px; pointer-events: none; z-index: 1; }
  /* sample look: a blocked cell (its export is skipped) keeps no fill. */
  .tc-cell.blocked { background: transparent !important; border: 1px dashed #8c2a2a;
    color: #8c2a2a !important; }
  /* A kit's two menu colours fill the cell's foot, 2 inside its edges so the
     cell's own colour frames them; the label stands over them. */
  .tc-cell.kit { justify-content: flex-start; padding-top: 3px; }
  .tc-sw { position: absolute; left: 2px; right: 2px; bottom: 2px; height: 9px; }
  .tc-acts .row { flex-wrap: nowrap; }
  /* The detail window (egui: a Window inside the tool's panel, movable by its
     title bar, where the member leaves it). */
  .tc-detail { position: absolute; z-index: 5; width: 300px; background: var(--panel);
    border: 1px solid var(--stroke); border-radius: 2px; box-shadow: 0 4px 16px #000a; }
  .tc-detail .bar { display: flex; align-items: center; padding: 2px 4px 2px 8px;
    border-bottom: 1px solid var(--stroke); cursor: move; user-select: none; }
  .tc-detail .body { padding: 8px; }
  .tc-detail .find { margin: 6px 0; }
  /* sample: where a player's 3D preview will stand (a later feature). */
  .tc-detail .preview { height: 180px; margin-bottom: 8px; background: var(--extreme);
    border-radius: 2px; display: flex; align-items: center; justify-content: center; }
  .tc-bar { display: flex; align-items: center; gap: 8px; padding: 4px 8px; min-height: 24px;
    border-top: 1px solid var(--stroke); }
  .tc-bar .strip { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis;
    white-space: nowrap; }
  .tc-split { display: flex; gap: 1px; }
  .tc-split button:first-child { border-radius: 2px 0 0 2px; min-width: 150px; }
  .tc-split button:last-child { border-radius: 0 2px 2px 0; padding: 0 4px; }
  .tc-badge { color: var(--warn); }
  .tc-empty { margin-top: 8px; padding: 24px; text-align: center; border: 1px dashed var(--stroke);
    border-radius: 2px; }
  /* The Save editor's toolbar line: the host's .top (a Panel::top's stroke). */
  /* Two rows, filled column by column: each column a pair of related states. */
  .tc-legend { display: grid; grid-template-rows: auto auto; grid-auto-flow: column;
    grid-auto-columns: max-content; gap: 4px 16px; }
  /* Its title and the one state with no pair stand over both rows, centred. */
  .tc-legend .both { grid-row: 1 / span 2; align-self: center; }
  .tc-legend .name { font-size: 14px; display: inline-flex; align-items: center; gap: 2px; }
  .tc-legend.shut { display: flex; }
  .tc-legend span.k { display: inline-flex; align-items: center; gap: 4px; }
  .tc-legend .tc-cell { width: 14px; height: 14px; cursor: default; }
  .tc-legend .two { line-height: 1.25; }
`;

// ---- Cell states (gui.md "Cell states"): the check-phase and compile-outcome colours ----
// A check's warning or error is no fill of its own: it outlines the cell's
// colour (checked, or shallow-checked) until the compile has gone through
// the cell, which then turns solid in the same colour.
// The suite's warning amber and error red (the drafts' --warn and --error).
const WARN = "#d29922", ERR = "#f85149";
const COLOURS = { unchecked: "#0a0a0a", checking: "#dcdcdc", partialOk: "#3278dc",
  fullOk: "#800080", done: "#008000", doneWarn: WARN, doneErr: ERR };
const OUTLINES = { warning: WARN, error: ERR, partialError: ERR };
const OFF = "#333333";
const STATE_NAMES = { unchecked: "Unchecked", checking: "Checking",
  partialOk: "Shallow check done (archive)", fullOk: "Checked", warning: "Warning",
  partialError: "Error", error: "Error", done: "Done",
  doneWarn: "Done with warnings", doneErr: "Done with errors", blocked: "Blocked" };
// The label's colour follows the cell's own colour, not the theme: a soft light
// grey on a dark cell, a soft dark grey on a light one.
function labelColour(hex) {
  const [r, g, b] = [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16));
  // 125, not the usual 140: the error red (#f85149) weighs 130 and takes the
  // dark grey, light text on it reading poorly; no other state colour lies between.
  return 0.299 * r + 0.587 * g + 0.114 * b > 125 ? "#1b1b1b" : "#dcdcdc";
}

// ---- Sample rows ----
// A cell: its check status, its folder, the codes of its findings (the
// Team compiler's catalog rows), a kit's menu colours, and `dropFile` where
// the finding drops one file and the rest of the folder still compiles.
const pad = n => String(n).padStart(2, "0");
const players = (from, to, status = "fullOk", folder = n => `${pad(n)} - Player ${pad(n)}`) =>
  Object.fromEntries(Array.from({ length: to - from + 1 }, (_, i) =>
    [from + i, { status, folder: folder(from + i), codes: [] }]));
const kit = (folder, c1, c2, status = "fullOk", codes = []) =>
  ({ status, folder, codes, swatch: [c1, c2] });
const with_ = (cells, n, patch) => { Object.assign(cells[n], patch); return cells; };
// A row's portraits cell, by the row's key: how many of its players have a
// portrait, the cell's status and its findings. A referee export has none.
const PORTRAITS = {
  "3": [23, "fullOk"],
  // An old Portraits/player_05.dds, with the one-click rename the plan gives it.
  a: [22, "error", ["portrait_name_invalid"],
    { label: "Rename player_05.dds to 05.dds", folder: "Portraits", count: 23 }],
  // /adv/ 11's portrait is in the player's own folder: its finding colours
  // this cell and the player's.
  civ4xg: [23, "fullOk"], inv1: [23, "fullOk"], aco: [23, "partialOk"], adv: [11, "error", ["texture_not_pow2"], null, true], an: [null, "unchecked"],
  egg: [23, "fullOk"], bant: [23, "fullOk"], biz1: [20, "fullOk"], biz2: [20, "partialOk"] };
function portraitsOf(key) {
  const [count, status, codes = [], fix, dropFile] = PORTRAITS[key] || [];
  return status ? { count, status, codes, fix, dropFile, folder: "Portraits" } : null;
}
// Every cell of a row but its label: the players, the portraits, the kits.
const cellsOf = r => [...Object.values(r.players), ...(r.portraits ? [r.portraits] : []),
  ...Object.values(r.kits)];

function sampleRows() {
  return [
    { key: "3", name: "/3/", id: 701, display: "3 Full Aesthetics Export Autumn26",
      label: { status: "fullOk", codes: [] }, players: players(1, 23),
      // Every kit slot, the widest a row gets.
      kits: { GK: kit("g1 - Keeper", "#f0c020", "#202020"), K1: kit("p1 - Home", "#c02020", "#f0f0f0"),
        K2: kit("p2 - Away", "#f0f0f0", "#2040a0"), K3: kit("p3 - Third", "#202020", "#c02020"),
        K4: kit("p4 - Retro", "#f0e0a0", "#803020"), K5: kit("p5 - Green", "#208040", "#f0f0f0"),
        K6: kit("p6 - Sky", "#60b0e0", "#f0f0f0"), K7: kit("p7 - Orange", "#f08020", "#202020"),
        K8: kit("p8 - Violet", "#7040b0", "#f0c020"), K9: kit("p9 - Grey", "#808080", "#202020") } },
    { key: "a", name: "/a/", id: 702, display: "a Full Aesthetics Export Autumn26",
      label: { status: "warning", codes: ["shared_folder_orphaned"] },
      players: with_(with_(players(1, 23), 7, { status: "warning", codes: ["fmdl_texture_not_found"] }),
        // sample: the one-click rename of a misnamed player folder; which code
        // names that folder is not in the catalog yet, so the nearest stands in.
        15, { status: "warning", codes: ["root_file_unexpected"], folder: "15 Snuffy",
          fix: { label: "Rename to 15 - Snuffy", folder: "15 - Snuffy" } }),
      kits: { GK: kit("g1", "#30a050", "#101010"), K1: kit("p1 - Pink", "#f080b0", "#f0f0f0"),
        K2: kit("p2 - Black", "#101010", "#101010", "fullOk", ["kit_colors_derived"]),
        K3: kit("p3 - Third", "#6040a0", "#f0c020") } },
    { key: "aco", name: "/aco/", id: 703, display: "aco Full Aesthetics Export Autumn26.zip", archive: true,
      label: { status: "partialOk", codes: [] },
      players: with_(players(1, 23, "partialOk"), 4, { status: "partialError", codes: ["texture_not_div4"] }),
      kits: { GK: kit("g1", "#202020", "#f0f0f0", "partialOk"), K1: kit("p1", "#2060c0", "#f0f0f0", "partialOk") } },
    { key: "adv", name: "/adv/", id: 704, display: "adv Midcup Aesthetics Export Autumn26",
      label: { status: "fullOk", codes: [] },
      players: with_(with_(players(1, 11), 5, { status: "error", codes: ["texture_not_div4"], dropFile: true }),
        // A portrait kept in the player's folder: its finding is the player's too.
        11, { status: "error", codes: ["texture_not_pow2"], dropFile: true,
          note: "The finding is on this player's portrait, which is in his folder" }),
      kits: { K1: kit("p1 - Lakers", "#602080", "#f0c020") } },
    { key: "an", name: "/an/", id: 705, display: "an Full Aesthetics Export Autumn26",
      label: { status: "checking", codes: [] },
      players: { ...players(1, 9), ...players(10, 16, "checking"), ...players(17, 23, "unchecked") },
      kits: { GK: kit("g1", "#0a0a0a", "#0a0a0a", "unchecked"), K1: kit("p1", "#0a0a0a", "#0a0a0a", "unchecked") } },
    { key: "egg", name: "/egg/", id: null, display: "egg Full Aesthetics Export Autumn26",
      label: { status: "error", codes: ["team_name_unknown"] }, blocked: "team_name_unknown",
      players: players(1, 23), kits: { GK: kit("g1", "#f0f0f0", "#c08020"), K1: kit("p1", "#f0e080", "#f0f0f0") } },
    { key: "asp", name: "/asp/", id: 706, display: "asp Aesthetics Export Autumn26", special: "untagged",
      label: { status: "error", codes: ["export_tag_missing"] }, players: {}, kits: {} },
    { key: "b", name: "/b/", id: 707, display: "b Full Aesthetics Export Summer26", special: "old",
      label: { status: "error", codes: ["export_layout_old"] }, players: {}, kits: {} },
    { key: "bant", name: "/bant/", id: 708, display: "bant Full Aesthetics Export Autumn26", special: "disabled",
      label: { status: "fullOk", codes: ["export_disabled"] }, players: players(1, 23),
      kits: { GK: kit("g1", "#101010", "#f0f0f0"), K1: kit("p1", "#208040", "#f0f0f0") } },
    { key: "biz1", name: "/biz/", id: 709, display: "biz Full Aesthetics Export Autumn26", special: "clash",
      label: { status: "error", codes: ["duplicate_aesthetics_export"] }, blocked: "duplicate_aesthetics_export",
      players: players(1, 23), kits: { K1: kit("p1", "#c0a020", "#101010") } },
    { key: "biz2", name: "/biz/", id: 709, display: "biz Full Aesthetics Export Autumn26 v2.7z", special: "clash",
      archive: true, label: { status: "error", codes: ["duplicate_aesthetics_export"] },
      blocked: "duplicate_aesthetics_export",
      players: players(1, 23, "partialOk"), kits: { K1: kit("p1", "#c0a020", "#101010", "partialOk") } },
    // A name too long for the label at the usual size.
    { key: "civ4xg", name: "/civ4xg/", id: 788, display: "civ4xg Full Aesthetics Export Autumn26",
      label: { status: "fullOk", codes: [] }, players: players(1, 23),
      kits: { GK: kit("g1", "#c0a020", "#202020"), K1: kit("p1", "#2050a0", "#c0a020") } },
    // A name that takes two lines of the label.
    { key: "inv1", name: "Invitational 1", id: 851, display: "Invitational 1 Full Aesthetics Export Autumn26",
      label: { status: "fullOk", codes: [] }, players: players(1, 23),
      kits: { GK: kit("g1", "#202020", "#f0f0f0"), K1: kit("p1", "#f0f0f0", "#c02020") } },
    { key: "refs", name: "/refs/", id: null, refs: true, display: "refs Full Aesthetics Export Autumn26",
      label: { status: "fullOk", codes: [] },
      players: players(1, 35, "fullOk", n => `${pad(1 + (n - 1) % 5)} - Referee ${1 + (n - 1) % 5}`),
      kits: { K1: kit("p1 - Referee", "#101010", "#f0e020") } },
  ].map(r => ({ ...r, portraits: portraitsOf(r.key) }));
}

// ---- State ----
// D: the draft controls (the scenarios' GIVENs). S: the view's own state.
// L: the log panel's state, in the host's shape.
const D = { writable: "yes", dataReadOnly: false, empty: false, closeOnSuccess: false };
// A compiling cell's bar, sliding left to right along its bottom edge,
// entering and leaving it, once every 0.9 s.
const sweepEl = cell => h("span", { class: "tc-sweep",
  style: `left:${(-40 + 140 * busyPhase(cell, 900)).toFixed(1)}%` });
// The phase of a compiling cell's motion, 0 to 1, counted from when the cell
// started, so a short task still shows the motion's start.
const busyPhase = (cell, periodMs) => ((performance.now() - cell.busyAt) % periodMs) / periodMs;
const S = { rows: sampleRows(), mode: "normal", column: false, unticked: {}, detail: null,
  highlight: null, run: null, last: null, finished: false, ids: {}, logScroll: 0,
  // Where the member left the detail window (null until it first opens), and
  // the legend folded away: both kept across launches (localStorage here).
  detailPos: JSON.parse(localStorage.getItem("drafts.tc.detail_pos") || "null"),
  legendShut: localStorage.getItem("drafts.tc.legend_shut") === "1" };
const L = { log: [], logHeight: 180, logCollapsed: false, logFilter: "all",
  logStick: true, logNew: 0 };

const MODES = { normal: "Compile", sideload: "Compile for sideloading", test: "Test output" };
const rowOf = key => S.rows.find(r => r.key === key);
const catalog = code => CATALOG["team-compiler"].find(r => r.code === code);
const severity = code => {
  const s = (catalog(code) || {}).severity || "Error";
  return s === "W" ? "Warning" : s.startsWith("E") ? "Error" : s;
};
const messageOf = code => (catalog(code) ? catalog(code).condition.replaceAll("`", "") : code);
// Worst severity first, then by code.
const RANK = { Fatal: 0, Error: 1, Warning: 2, Info: 3 };
const sorted = codes => [...codes].sort((a, b) =>
  RANK[severity(a)] - RANK[severity(b)] || a.localeCompare(b));
// What a cell shows: the run's outcome once it has one, else its check status.
const eligible = r => !r.special && !r.blocked && !S.unticked[r.key];
const rowOff = r => r.special === "disabled" || !!S.unticked[r.key];
// What a cell shows: blocked while its export is skipped whole (from the
// check on, so the grid says before Compile what will not compile), else
// the run's outcome once it has one, else its check status.
const shown = (r, cell) => cell === r.label ? labelOut(r) || cell.status
  : r.blocked ? "blocked" : cell.out || cell.status;
// A team's label after the run: the export's outcome, the worst of what its
// cells and the label itself came to (a cell the run skipped for its error
// counts as one). None while any cell of the row is still to be checked or
// processed.
function labelOut(r) {
  const cells = cellsOf(r);
  const pending = c => c.busy || ["unchecked", "checking"].includes(c.status)
    || (!c.out && (OUTCOME[c.status] || c.dropFile));
  if (!cells.some(c => c.out) || cells.some(pending)) return null;
  const states = [r.label.status, ...cells.map(c => c.out || c.status)];
  if (states.some(s => ["doneErr", "error", "partialError"].includes(s))) return "doneErr";
  return states.some(s => ["doneWarn", "warning"].includes(s)) ? "doneWarn" : "done";
}
// A cell's or a label's fill and outline. An off row is flat grey.
function look(r, cell) {
  if (rowOff(r)) return { fill: OFF, text: "#6e6e6e", outline: null };
  const state = shown(r, cell);
  const fill = OUTLINES[state] ? COLOURS[r.archive ? "partialOk" : "fullOk"] : COLOURS[state] || COLOURS.unchecked;
  return { fill, text: labelColour(fill), outline: OUTLINES[state] || null };
}
const lookStyle = l => `background:${l.fill}; color:${l.text}` + (l.outline ? `; --ol:${l.outline}` : "");

function cellTip(r, cell, what) {
  const state = shown(r, cell);
  const lines = [cell.folder || r.display, rowOff(r) ? "Off: not compiled" : STATE_NAMES[state]];
  if (what === "portraits")
    lines.push(cell.count == null ? "Portraits not counted yet"
      : `${cell.count} of the export's ${Object.keys(r.players).length} players have a portrait`);
  if (state === "blocked") lines.push("Blocked: this export is skipped (" + r.blocked + ")");
  if (cell.note) lines.push(cell.note);
  for (const code of sorted(cell.codes)) lines.push(`${severity(code)} - ${code}: ${messageOf(code)}`);
  if (what === "kit" && cell.codes.includes("kit_colors_derived"))
    lines.push("The menu colours shown are the ones derived from the texture");
  return lines.join("\n");
}

// ---- The grid ----
function cellEl(r, cell, text, scope, what) {
  const state = shown(r, cell), l = look(r, cell);
  // The kit's menu colours, once known and while the row is on (sample look:
  // two flat halves stand in for the icon's pattern, which color_tools draws).
  const swatch = cell.swatch && !rowOff(r) && !["unchecked", "checking", "blocked"].includes(state);
  return h("div", { class: "tc-cell" + (what === "portraits" ? " wide" : "") + (cell.busy ? " busy" : "")
      + (state === "blocked" && !rowOff(r) ? " blocked" : "") + (l.outline ? " ol" : "") + (swatch ? " kit" : "")
      + (S.highlight === scope ? " hl" : ""),
    style: lookStyle(l),
    title: cellTip(r, cell, what), "data-scope": scope,
    onclick: e => openDetail(e, { row: r.key, what, slot: text }) },
    text,
    cell.busy ? sweepEl(cell) : null,
    swatch ? h("span", { class: "tc-sw", style: `background:linear-gradient(90deg, ${cell.swatch[0]} 50%, ${cell.swatch[1]} 50%)` })
      : null);
}

// A vertical line's cell.
const vline = () => h("td", { class: "tc-line" }, h("div", {}));
// The room between two sections of a row.
const gap = () => h("td", { class: "tc-gap" }, h("div", {}));
const KIT_SLOTS = ["GK", "K1", "K2", "K3", "K4", "K5", "K6", "K7", "K8", "K9"];
// The player columns. sample: the referee row's positions 24 to 35 take a
// second line under 01 to 12, where the plan extends the columns to 35;
// 35 columns push every row's kit cells out of the default window.
const SLOTS = 23;

// The ID cell: the teams list's ID, or red and editable for a team the list
// does not hold. Enter confirms; an ID another team holds asks first.
function idCell(r) {
  if (r.refs) return h("td", { class: "tc-id", title: "The referee export has no team ID" }, "");
  if (r.id != null) return h("td", { class: "tc-id mono", title: "From teams_list.txt" }, String(r.id));
  const entry = idEntry(r);
  const input = h("input", { type: "text", class: "mono", value: entry.text, placeholder: "???",
    oninput: e => { entry.text = e.target.value; },
    onkeydown: e => { if (e.key === "Enter") confirmId(r); } });
  const tip = entry.note ? entry.note.text
    : "team_name_unknown: " + r.name + " is not on the teams list. Type its ID (701-920) and press Enter to add it";
  return h("td", { class: "tc-id bad", title: tip }, input);
}
// A row's typed ID and what the last Enter answered (the cell's tooltip).
const idEntry = r => S.ids[r.key] || (S.ids[r.key] = { text: "", note: null });
function confirmId(r, reassign) {
  const entry = idEntry(r);
  const n = Number(entry.text);
  if (!/^\d+$/.test(entry.text) || n < 701 || n > 920) {
    entry.note = { text: "A team ID is a number from 701 to 920" };
  } else if (D.dataReadOnly) {
    entry.text = "";
    entry.note = { text: "teams_list_read_only: the data folder is not writable, so the teams list was not changed" };
  } else if (COMPILE_TEAMS[n - 701] && !reassign) {
    entry.note = { text: `${n} is in use by ${COMPILE_TEAMS[n - 701]}`, ask: true };
  } else {
    // The write revalidates every row: the team that lost the ID turns red.
    const loser = S.rows.find(o => o.id === n);
    if (loser) { loser.id = null; loser.label = { status: "error", codes: ["team_name_unknown"] };
      loser.blocked = "team_name_unknown"; }
    r.id = n; r.label = { status: "fullOk", codes: [] }; delete r.blocked;
    delete S.ids[r.key];
  }
  render();
}

// What a row says in place of its cells: a message too long for the room
// right of the last line, with the buttons that answer it. The team's cells
// are hidden while it stands (an untagged or old export has none to show; a
// team typed an ID another team holds gets its cells back with the answer).
function rowMessage(r) {
  const acts = [];
  if (r.special === "untagged")
    acts.push(h("span", { class: "warn" }, "Full or Midcup?"),
      button("Full", () => tagExport(r, "Full"), { title: "Renames the export: " + r.display.replace(" ", " Full ") }),
      button("Midcup", () => tagExport(r, "Midcup"), { title: "Renames the export: " + r.display.replace(" ", " Midcup ") }));
  if (r.special === "old")
    acts.push(h("span", { class: "err" }, "Old export format"),
      button("Upgrade", () => switchToTool("export-upgrader"),
        { title: "Opens the Export upgrader with this export as its input" }));
  const asked = r.id == null && !r.refs && idEntry(r).note;
  if (asked && asked.ask)
    acts.push(h("span", { class: "warn" }, asked.text + "; reassign?"),
      button("Yes", () => confirmId(r, true)),
      button("No", () => { idEntry(r).note = null; render(); }));
  return acts.length ? h("td", { class: "tc-acts act", colspan: String(SLOTS + 3 + KIT_SLOTS.length) },
    row(...acts)) : null;
}
// A row's small button, right of the last line.
function rowActions(r) {
  const acts = [];
  if (r.special === "disabled")
    acts.push(button("Enable", () => { delete r.special; r.label.codes = []; render(); },
      { title: "Removes the export's NO_USE marker" }));
  if (r.special === "clash")
    acts.push(r.archive
      // An archive's Disable: no NO_USE marker can be written inside it, so it
      // is moved out of the compiler's way instead.
      ? button("Move to unused", () => dropClash(r), { title: "Moves the archive into the exports folder's unused folder: no NO_USE marker can be written inside an archive" })
      : button("Disable", () => { r.special = "disabled"; dropClash(r, true); },
        { title: "Writes the export's NO_USE marker, so the other one compiles" }));
  return acts.length ? h("td", { class: "tc-acts act" }, row(...acts)) : null;
}
function tagExport(r, tag) {
  r.display = r.display.replace(" ", ` ${tag} `);
  delete r.special;
  r.label = { status: "fullOk", codes: [] };
  r.players = players(1, 23);
  r.kits = { GK: kit("g1", "#101010", "#f0f0f0"), K1: kit("p1", "#40a0c0", "#f0f0f0") };
  r.portraits = { count: 23, status: "fullOk", codes: [], folder: "Portraits" };
  render();
}
// One of two clashing exports leaves (disabled, or moved away): the other compiles.
function dropClash(r, keep) {
  if (!keep) S.rows = S.rows.filter(o => o !== r);
  for (const o of S.rows) if (o !== r && o.special === "clash" && o.name === r.name) {
    delete o.special; delete o.blocked;
    o.label = { status: o.archive ? "partialOk" : "fullOk", codes: [] };
  }
  render();
}

// The team label's font size: the usual 12 while the name fits the 44 points
// the label's 52 leave between its paddings, else the largest half-point size at which it does (the monospace
// font's letters are 0.602 of its size wide; egui: the galley measured, the
// size stepped down until it fits).
const nameSize = name => Math.min(12, Math.floor(2 * 44 / (name.length * 0.602)) / 2);
// The label's lines and their size: one line down to 9 points (eight
// letters); a longer name is cut in two halves, one over the other, at 9 or
// at what the longer half needs. sample: the cut is at the middle letter.
function nameLines(name) {
  if (nameSize(name) >= 9) return { lines: [name], size: nameSize(name) };
  const half = Math.ceil(name.length / 2);
  return { lines: [name.slice(0, half), name.slice(half)], size: Math.min(9, nameSize(name.slice(0, half))) };
}
function gridRow(r) {
  const off = rowOff(r);
  const cells = [];
  if (S.column)
    cells.push(h("td", { class: "act" }, r.special ? null
      : h("input", { type: "checkbox", checked: !S.unticked[r.key], title: "Compile this export at the next Compile",
          onchange: e => { if (e.target.checked) delete S.unticked[r.key]; else S.unticked[r.key] = true; render(); } })));
  const l = look(r, r.label), named = nameLines(r.name);
  cells.push(h("td", { class: "tc-name mono" + (l.outline ? " ol" : "") + (S.highlight === r.name ? " hl" : ""),
    style: lookStyle(l) + `; font-size:${named.size}px; line-height:${named.lines.length > 1 ? named.size + 1 + "px" : "normal"}`,
    "data-scope": r.name,
    title: cellTip(r, { ...r.label, folder: r.display + (r.archive ? " (archive: shallow check)" : "") }),
    oncontextmenu: e => { e.preventDefault(); rowMenu(e, r); },
    onclick: e => openDetail(e, { row: r.key, what: "label" }) },
    named.lines.map(line => h("div", {}, line))));
  cells.push(idCell(r), vline());
  const message = rowMessage(r), acts = rowActions(r);
  if (message) {
    cells.push(message, vline());
  } else {
    for (let n = 1; n <= SLOTS; n++)
      cells.push(playerSlot(r, n));
    // The portraits: one cell, the count of the row's players that have one
    // over the players it has, in the colour of the worst of them.
    const po = r.portraits;
    cells.push(gap(), h("td", { class: "tc-slot wide" },
      po ? cellEl(r, po, (po.count == null ? "?" : po.count) + "/" + Object.keys(r.players).length,
        `${r.name} portraits`, "portraits") : null));
    cells.push(gap());
    for (const k of KIT_SLOTS)
      cells.push(h("td", { class: "tc-slot" },
        r.kits[k] ? cellEl(r, r.kits[k], k, `${r.name} ${k}`, "kit") : null));
    cells.push(vline());
  }
  if (acts) cells.push(acts);
  const lines = [h("tr", { class: off ? "off" : "" }, cells)];
  if (r.refs)
    lines.push(h("tr", {}, h("td", { colspan: String(S.column ? 3 : 2) }), vline(),
      Array.from({ length: SLOTS }, (_, i) => playerSlot(r, SLOTS + 1 + i)),
      gap(), h("td", { class: "tc-slot wide" }), gap(),
      KIT_SLOTS.map(() => h("td", { class: "tc-slot" })), vline()));
  return lines;
}
const playerSlot = (r, n) => h("td", { class: "tc-slot" },
  r.players[n] ? cellEl(r, r.players[n], pad(n), `${r.name} ${pad(n)}`, "player") : null);

// A row's context menu: the CLI's --export from the grid.
function rowMenu(e, r) {
  closeMenus();
  const [left, top] = inWin(e.clientX, e.clientY);
  const item = (text, fn, disabled) => h("button", { disabled,
    onclick: ev => { ev.stopPropagation(); closeMenus(); fn(); } }, text);
  document.getElementById("win").append(h("div", { class: "menu", style: `left:${left}px; top:${top}px` },
    item("Compile only this export", () => startRun([r.key]), !eligible(r) || !!S.run),
    item("Open folder", () => openFolder(r.display))));
}
const openFolder = what => window_("Open folder",
  h("p", {}, "The file manager opens here, on ", h("span", { class: "mono" }, `${EXE_FOLDER}\\exports\\${what}`), "."),
  button("Close", closeWindow), "open-folder");

function grid() {
  if (D.empty)
    return h("div", { class: "tc-empty", "data-panel": "empty" },
      h("p", {}, strong("No exports found")),
      h("p", { class: "mono" }, EXE_FOLDER + "\\exports"),
      h("p", {}, "Drop an export here: a folder, a .zip or a .7z."),
      button("Open exports folder", () => openFolder("")));
  return h("table", { class: "tc-grid", "data-panel": "grid" }, titleRow(), S.rows.flatMap(gridRow),
    h("tr", { class: "fill" }, h("td", { colspan: String(S.column ? 3 : 2) }), vline(),
      h("td", { colspan: String(SLOTS + 3 + KIT_SLOTS.length) }), vline()));
}
// The title row. sample: the titles' wording.
function titleRow() {
  const th = (text, props = {}) => h("th", props, text);
  return h("tr", { class: "head" },
    S.column ? th("") : null,
    th("Team"), th("ID", { class: "mid" }), vline(),
    th("Players", { colspan: String(SLOTS) }), gap(),
    th(h("div", {}, "Portraits"), { class: "mid" }), gap(),
    th("Kits", { colspan: String(KIT_SLOTS.length) }), vline(), th(""));
}
// The legend, above the grid and outside its scroll area, a line under it
// as under a toolbar: what each cell colour means.
// The legend's columns: a pair of related states one over the other, or one
// state alone, centred. It folds to its title, and stays folded.
const LEGEND = [["unchecked", "checking"], ["partialOk"], ["warning", "error"],
  ["doneWarn", "doneErr"], ["fullOk", "done"], ["blocked"]];
function legend() {
  const tip = S.legendShut ? "Show the legend" : "Hide the legend";
  const toggle = () => {
    S.legendShut = !S.legendShut;
    localStorage.setItem("drafts.tc.legend_shut", S.legendShut ? "1" : "0");
    render(); };
  // The chevron points where a click sends the legend: up to fold it away,
  // down to bring it back. The title does what the chevron does.
  const title = h("span", { class: "both name", title: tip, style: "cursor:pointer", onclick: toggle },
    button(S.legendShut ? "▾" : "▴", null, { frame0: true }), "Legend");
  if (S.legendShut) return h("div", { class: "tc-legend shut top weak", "data-panel": "legend" }, title);
  const sample = { key: "", label: {} };
  const entry = (k, alone) => {
    const l = k === "blocked" ? { fill: "transparent", text: "", outline: null } : look(sample, { status: k });
    return h("span", { class: "k" + (alone ? " both" : "") },
      h("span", { class: "tc-cell" + (k === "blocked" ? " blocked" : "") + (l.outline ? " ol" : ""),
        style: lookStyle(l) }),
      // The one long name takes two lines; its swatch stays centred on both.
      k === "partialOk" ? h("span", { class: "two" }, h("div", {}, "Shallow check done"), h("div", {}, "(archive)"))
        : STATE_NAMES[k]);
  };
  return h("div", { class: "tc-legend top weak", "data-panel": "legend" }, title,
    LEGEND.flatMap(col => col.map(k => entry(k, col.length === 1))));
}

// ---- The detail window: what a clicked cell holds, and what to do about it ----
// It opens beside the first cell clicked and from then on where the member
// left it; a click on another cell changes what it shows, not where it is.
function openDetail(e, detail) {
  if (!S.detailPos) {
    const pipe = e.currentTarget.closest(".pipe").getBoundingClientRect();
    const cell = e.currentTarget.getBoundingClientRect(), scale = winScale();
    S.detailPos = { x: Math.min((cell.right - pipe.left) / scale + 8, pipe.width / scale - 310),
      y: Math.min((cell.top - pipe.top) / scale, pipe.height / scale - 320) };
  }
  S.detail = detail;
  render();
}
function dragDetail(e, win) {
  if (e.button !== 0 || e.target.closest("button")) return;
  const from = { ...S.detailPos }, x0 = e.clientX, y0 = e.clientY, scale = winScale();
  const move = ev => {
    S.detailPos = { x: Math.max(0, from.x + (ev.clientX - x0) / scale), y: Math.max(0, from.y + (ev.clientY - y0) / scale) };
    win.style.left = S.detailPos.x + "px"; win.style.top = S.detailPos.y + "px";
  };
  const up = () => {
    document.removeEventListener("mousemove", move);
    document.removeEventListener("mouseup", up);
    localStorage.setItem("drafts.tc.detail_pos", JSON.stringify(S.detailPos));
  };
  document.addEventListener("mousemove", move);
  document.addEventListener("mouseup", up);
  e.preventDefault();
}
function detailPanel() {
  if (!S.detail) return null;
  const r = rowOf(S.detail.row);
  if (!r) return null;
  const cell = S.detail.what === "label" ? r.label
    : S.detail.what === "kit" ? r.kits[S.detail.slot]
    : S.detail.what === "portraits" ? r.portraits : r.players[Number(S.detail.slot)];
  if (!cell) return null;
  const folder = S.detail.what === "label" ? r.display : cell.folder;
  const title = S.detail.what === "label" ? r.name
    : S.detail.what === "portraits" ? `${r.name} portraits` : `${r.name} ${S.detail.slot}`;
  // A link opens it with no cell clicked: at the grid's right.
  const pos = S.detailPos || { x: 760, y: 70 };
  const win = h("div", { class: "tc-detail", "data-panel": "detail", style: `left:${pos.x}px; top:${pos.y}px` });
  win.append(
    h("div", { class: "bar", onmousedown: e => { S.detailPos = pos; dragDetail(e, win); } },
      strong(title), h("span", { style: "flex:1" }),
      button("✕", () => { S.detail = null; render(); }, { frame0: true, title: "Close" })),
    h("div", { class: "body" },
      S.detail.what === "player" ? h("div", { class: "preview weak" }, "3D preview (a later feature)") : null,
      h("div", { class: "mono" }, folder),
      h("div", { class: "weak" }, rowOff(r) ? "Off: not compiled" : STATE_NAMES[shown(r, cell)]),
      cell.codes.length ? sorted(cell.codes).map(code =>
        h("div", { class: "find " + (severity(code) === "Warning" ? "warn" : severity(code) === "Info" ? "" : "err") },
          severity(code), " ",
          h("span", { class: "msg-link", title: "Open the help at this message",
            onclick: () => openHelp({ tool: "team-compiler", topic: "messages", anchor: code }) }, code),
          h("div", {}, messageOf(code))))
        : h("div", { class: "find weak" }, "No findings."),
      row(button("Open folder", () => openFolder(r.display + (S.detail.what === "label" || r.archive ? "" : "\\" + folder)),
          { title: r.archive ? "An archive export: its folder opens with the archive selected" : null }),
        // A finding with one obvious fix: a rename the member asks for by a click.
        cell.fix && !r.archive ? button(cell.fix.label, () => {
          cell.folder = cell.fix.folder; if (cell.fix.count) cell.count = cell.fix.count;
          cell.status = "fullOk"; cell.codes = []; delete cell.fix; render(); },
          { title: "A rename; refused when that name exists" }) : null)));
  return win;
}

// ---- The toolbar: the run strip and the split Compile button ----
const BADGES = {
  running: "PES is running: the compile cannot replace the installed CPKs. Close the game, or the run writes to the output folder only",
  elevation: "The game folder needs administrator rights to write to. Restart the app as administrator, or the run writes to the output folder only" };
// The grid summed up, what the strip says while nothing runs: can I compile?
function summary() {
  if (D.empty) return "";
  const on = S.rows.filter(eligible);
  const having = states => on.filter(r => [r.label, ...cellsOf(r)].some(c => states.includes(c.status))).length;
  const count = (n, one, many = one) => n ? `${n} ${n === 1 ? one : many}` : null;
  return [count(S.rows.length, "export", "exports"), count(S.rows.length - on.length, "skipped"),
    count(having(["error", "partialError"]), "with errors"), count(having(["warning"]), "with warnings")]
    .filter(Boolean).join(" · ");
}
function toolbar() {
  const run = S.run;
  const strip = run
    ? `${run.stage} · ${mmss(run.elapsed)} · ${run.files} files written (${Math.round(run.mb)} MB) · ${run.memory} MB of system memory in use`
    : [S.last, summary()].filter(Boolean).join("  —  ");
  const sideloadOff = ["2015", "2016"].includes(shellPesVersion());
  const main = button(run ? "Cancel" : MODES[S.mode], run ? cancelRun : () => startRun(), {});
  main.id = "compile-btn";
  const arrow = button("⏷", e => { e.stopPropagation(); modeMenu(e.currentTarget, sideloadOff); },
    { disabled: !!run, title: "The output modes" });
  return h("div", { class: "tc-bar", "data-panel": "run" },
    h("span", { class: "strip mono", id: "compile-progress" }, strip),
    S.mode === "sideload"
      ? button("Launch PES", () => window_("Launch PES",
          h("p", {}, `PES${shellPesVersion()}.exe starts from the game folder here, never through Steam.`),
          button("Close", closeWindow), "launch-pes"),
          { title: "Starts the selected version's game from its install folder" })
      : null,
    D.writable !== "yes" ? h("span", { class: "tc-badge", title: BADGES[D.writable] }, "⚠") : null,
    h("div", { class: "tc-split" }, main, arrow));
}
function modeMenu(btn, sideloadOff) {
  closeMenus();
  const r = btn.getBoundingClientRect();
  const [left, top] = inWin(r.right, r.top);
  const item = (text, fn, disabled, title) => h("button", { disabled, title,
    onclick: ev => { ev.stopPropagation(); closeMenus(); fn(); } }, text);
  const pick = m => () => { S.mode = m; render(); };
  const m = h("div", { class: "menu", style: `left:${left}px; top:${top}px` },
    item(MODES.normal, pick("normal")),
    item(MODES.sideload, pick("sideload"), sideloadOff,
      sideloadOff ? "No runtime serves sideloaded files on PES 2015 and 2016" : "Unpacked into the game folder's livecpk"),
    item(MODES.test, pick("test"), false, "The processed exports, unpacked in their own layout"),
    h("div", { class: "sep", style: "margin:5.5px 0" }),
    item(S.column ? "Hide the compile column" : "Show the compile column",
      () => { S.column = !S.column; if (!S.column) S.unticked = {}; render(); }),
    item("Open output folder", openOutputStub),
    h("div", { class: "sep", style: "margin:5.5px 0" }),
    item("Copy report", () => window_("Copy report", h("p", {},
      "The run's lines, as the console prints them, are put on the clipboard."),
      button("Close", closeWindow), "more")));
  document.getElementById("win").append(m);
  // Opens upward and right-aligned: the toolbar sits above the log.
  m.style.left = (left - m.offsetWidth) + "px";
  m.style.top = Math.max(4, top - m.offsetHeight - 2) + "px";
}

// ---- The run: a fake compile over the eligible rows' cells ----
// The app's tasks run in parallel and end in no fixed order, so the cells
// change scattered over the grid, a few at once: the fake run takes them in
// a shuffled order (the same one every time), BUSY of them in hand, as a
// 16-thread machine's.
const BUSY = 16;
const OUTCOME = { fullOk: "done", partialOk: "done", warning: "doneWarn" };
function startRun(only) {
  if (S.run) return;
  const rows = S.rows.filter(r => only ? only.includes(r.key) : eligible(r));
  for (const r of S.rows) for (const cell of cellsOf(r)) { delete cell.out; delete cell.busy; }
  const tasks = rows.flatMap(r => cellsOf(r)
    // A red cell is skipped and keeps its colour, unless only one file is dropped.
    .filter(cell => OUTCOME[cell.status] || cell.dropFile).map(cell => ({ r, cell })))
    .map((task, i) => ({ ...task, order: (i * 7919) % 211 })).sort((a, b) => a.order - b.order);
  const start = Date.now();
  S.finished = false;
  S.run = { stage: "Processing", elapsed: 0, files: 0, mb: 0, memory: 310, tasks, at: 0, rows: rows.length };
  L.log.push({ severity: "Info", code: null, scope: null,
    text: `Info - ${MODES[S.mode]}: ${rows.length} exports` });
  S.run.timer = setInterval(() => {
    const run = S.run;
    if (!run) return;
    run.elapsed = Math.floor((Date.now() - start) / 1000);
    const ended = run.tasks[run.at - BUSY];
    if (ended) { delete ended.cell.busy;
      ended.cell.out = ended.cell.dropFile ? "doneErr" : OUTCOME[ended.cell.status];
      run.files += 4; run.mb += 2.1; }
    const next = run.tasks[run.at++];
    if (next) { next.cell.busy = true; next.cell.busyAt = performance.now(); run.memory = 310 + (run.at * 37) % 900; }
    else if (run.at - BUSY < run.tasks.length) { /* the last ones in hand end */ }
    else if (run.stage === "Processing" && S.mode === "normal") run.stage = "Packing the CPKs";
    else if (run.stage === "Packing the CPKs") {
      run.stage = "Updating the save";
      if (D.writable === "running") { stopTimer(); return askEditMode(run); }
    }
    else return endRun();
    toolActivity("team-compiler", { done: Math.min(run.at, run.tasks.length), total: run.tasks.length, elapsed: run.elapsed });
    render();
  }, 60);
  render();
}
function stopTimer() { clearInterval(S.run.timer); }
function stopRun() {
  stopTimer();
  toolActivity("team-compiler", null);
}
// The savefile step with the save's own PES running (gui.md "Row actions",
// the save findings): the Save editor's Edit mode question, in place of the
// command line's --edit-mode-closed, opening on what the step writes, since
// the member asked for a compile, not a save. Update save runs the step as
// the flag does; Skip leaves the save as it was, the run going on to its end.
// sample wording: the dialog's text and the log lines.
function askEditMode(run) {
  render();
  const go = write => {
    closeWindow();
    appendLog([write
      ? { severity: "Info", code: null, scope: null, text: "Info - The save was updated" }
      : { severity: "Warning", code: "savefile_skipped_pes_running", scope: null,
          text: "Warning - The save was not updated: PES is running" }], L);
    endRun();
    render();
  };
  const pes = `PES ${shellPesVersion()}`;
  window_(`Update ${pes}'s save?`, h("div", {},
    h("p", {}, "The CPKs are installed. One step is left: writing the compiled teams' aesthetics "
      + `settings into ${pes}'s save while the game is running.`),
    h("p", {}, "If the game's Edit mode is open, use its Data Management > Load once the save is "
      + "updated, before you leave Edit mode. Leaving Edit mode after changing anything in it "
      + "writes the game's own copy over the save, without the new settings."),
    h("p", {}, "If Edit mode is closed, the new settings show when you open it.")),
    [button("Skip", () => go(false)), button("Update save", () => go(true))], "edit-mode");
}
function endRun() {
  const run = S.run;
  stopRun();
  S.run = null;
  S.last = `${MODES[S.mode]} finished in ${mmss(run.elapsed)} · ${run.files} files written (${Math.round(run.mb)} MB)`;
  S.finished = true;
  appendLog([{ severity: "Info", code: null, scope: null, text: `Info - Compiled ${run.rows} exports` }], L);
  publishNotice(`Compiled ${run.rows} exports`, { label: "Open output", run: openOutputStub }, "success");
  // Success is a run with no export done with errors: with one, the window
  // stays open on the grid that shows it.
  const failed = S.rows.filter(eligible).some(r => cellsOf(r).some(c => c.out === "doneErr"));
  if (S.autorun && D.closeOnSuccess && !failed)
    window_("Draft only: the window closes", h("p", {}, "Started by quick_compile.bat with "
      + "quick_compile_close_on_success on, the app closes here: the run ended with no error."),
      button("Close", closeWindow), "autorun-close");
  S.autorun = false;
}

// ---- A check arriving live (gui.md "Live validation") ----
// Every cell of the rows starts as Checking and flips to its result as its
// scope's check ends: the checks run in parallel, so the cells change
// scattered over the rows, a few at once. `then` runs once the last one is in.
function liveCheck(rows, then) {
  const cells = rows.flatMap(r => [r.label, ...cellsOf(r)]);
  // Every cell starts unchecked; a few at a time start checking, scattered
  // over the rows as the parallel checks pick them up, and each turns to its
  // result a few ticks after it started.
  for (const cell of cells) { cell.real = cell.real || cell.status; cell.status = "unchecked"; }
  const order = cells.map((cell, i) => ({ cell, k: (i * 7919) % 211 })).sort((a, b) => a.k - b.k);
  const PER_TICK = 4, TICKS_CHECKING = 3;
  let tick = 0, started = 0, ended = 0;
  clearInterval(S.checkTimer);
  S.checkTimer = setInterval(() => {
    tick++;
    for (let n = 0; n < PER_TICK && started < order.length; n++) {
      const o = order[started++];
      o.cell.status = "checking"; o.at = tick;
    }
    // Cells end in the order they started, each TICKS_CHECKING ticks later.
    while (ended < started && tick - order[ended].at >= TICKS_CHECKING) {
      const { cell } = order[ended++];
      cell.status = cell.real; delete cell.real;
    }
    render();
    if (ended >= order.length) { clearInterval(S.checkTimer); S.checkTimer = null; if (then) then(); }
  }, 50);
  render();
}

// ---- The file drop (gui.md "Row actions", the empty grid) ----
// A folder, .zip or .7z dropped anywhere on the grid, filled or empty, is
// copied into the exports folder and checked like any new export; a name
// the folder already holds is refused, naming it, and nothing is copied.
// sample: every new name becomes /ck/'s export (the teams list's 712).
function dropNames(names) {
  for (const name of names) {
    if (S.rows.some(r => r.display === name)) {
      publishNotice(`${name} is already in the exports folder: nothing copied`, null, "warning");
      continue;
    }
    const r = { key: "ck" + S.rows.length, name: "/ck/", id: 712, display: name,
      archive: /\.(zip|7z)$/i.test(name),
      label: { status: "fullOk", codes: [] }, players: players(1, 23),
      kits: { GK: kit("g1", "#202020", "#f0c020"), K1: kit("p1", "#f0f0f0", "#108040"),
        K2: kit("p2", "#108040", "#f0f0f0") },
      portraits: { count: 23, status: "fullOk", codes: [], folder: "Portraits" } };
    if (r.archive) { r.label.status = "partialOk";
      for (const cell of cellsOf(r)) cell.status = "partialOk"; }
    D.empty = false;
    S.rows.push(r);
    publishNotice(`${name} copied into the exports folder`, null, "success");
    liveCheck([r]);
  }
  render();
}
function dropZone(el) {
  const files = e => e.dataTransfer && [...e.dataTransfer.types].includes("Files");
  el.addEventListener("dragover", e => { if (!files(e)) return; e.preventDefault(); el.classList.add("tc-drop"); });
  el.addEventListener("dragleave", e => { if (e.target === el) el.classList.remove("tc-drop"); });
  el.addEventListener("drop", e => { if (!files(e)) return; e.preventDefault(); el.classList.remove("tc-drop");
    dropNames([...e.dataTransfer.files].map(f => f.name)); });
  return el;
}
// Cancel: the cells not yet processed return to their check colours; nothing
// is written or replaced.
function cancelRun() {
  for (const r of S.rows) for (const cell of cellsOf(r)) {
    delete cell.busy; delete cell.out;
  }
  stopRun();
  S.run = null;
  S.last = "Cancelled: nothing was written";
  appendLog([{ severity: "Warning", code: null, scope: null, text: "Warning - The run was cancelled" }], L);
}

// The check's findings as log lines, once: a click on one highlights its cell.
function seedLog() {
  for (const r of S.rows) {
    const each = (cell, scope) => { for (const code of cell.codes)
      L.log.push({ severity: severity(code), code, scope,
        text: `${severity(code)} - ${scope} - ${code}: ${messageOf(code)}` }); };
    each(r.label, r.name);
    for (const [n, cell] of Object.entries(r.players)) each(cell, `${r.name} ${pad(n)}`);
    if (r.portraits) each(r.portraits, `${r.name} portraits`);
    for (const [k, cell] of Object.entries(r.kits)) each(cell, `${r.name} ${k}`);
  }
}
seedLog();
function gotoCell(scope) {
  S.highlight = scope;
  render();
  const el = document.querySelector(`#center [data-scope="${scope}"]`);
  if (el) el.scrollIntoView({ block: "nearest", inline: "nearest" });
}

// ---- The view: the grid in what remains, the toolbar, the log at the bottom ----
function drawView(center) {
  const oldLog = center.querySelector(".log-body");
  if (oldLog) S.logScroll = oldLog.scrollTop;
  center.innerHTML = "";
  center.classList.add("tc-view");
  center.append(h("div", { class: "pipe" },
    D.empty ? null : legend(),
    dropZone(h("div", { class: "tc-top" }, h("div", { class: "pipe-grid" }, grid()))),
    toolbar(),
    named(logPanel(L, { tool: "team-compiler", onLine: gotoCell }), "log"),
    detailPanel()));
  const lb = center.querySelector(".log-body");
  if (lb) lb.scrollTop = L.logStick ? lb.scrollHeight : S.logScroll;
}

// ---- Draft controls: the scenarios' GIVENs, visibly outside Studio ----
function draftControls() {
  return [
    h("label", { class: "cb", title: "TC-GUI-16" }, "The destination is",
      combo([["yes", "writable"], ["running", "locked: PES is running"], ["elevation", "locked: elevation needed"]],
        D.writable, v => D.writable = v)),
    h("span", { title: "TC-GUI-07" },
      cb("The data folder is read-only", D.dataReadOnly, v => D.dataReadOnly = v)),
    h("span", { title: "gui.md \"Row actions\", the empty grid" },
      cb("The exports folder is empty", D.empty, v => D.empty = v)),
    button("Reset the sample rows", () => { S.rows = sampleRows(); S.detail = null; S.unticked = {};
      S.ids = {}; S.last = null; S.finished = false; L.log = []; seedLog(); render(); }),
    h("span", { title: "gui.md \"Live validation\"" },
      button("Check every export again", () => liveCheck(S.rows.filter(r => !r.special)))),
    h("span", { title: "gui.md \"Row actions\", the empty grid: a drop works on a filled grid too" },
      button("Drop ck Full Aesthetics Export Autumn26.zip", () => dropNames(["ck Full Aesthetics Export Autumn26.zip"])),
      button("Drop an export already there", () => dropNames(["a Full Aesthetics Export Autumn26"]))),
    h("span", { title: "gui.md \"Toolbar\": a --gui autorun presses Compile once the initial check completes" },
      button("Start as quick_compile.bat", () => { if (S.run) return; S.autorun = true; S.mode = "normal";
        liveCheck(S.rows.filter(r => !r.special), () => startRun()); }),
      cb("quick_compile_close_on_success", D.closeOnSuccess, v => D.closeOnSuccess = v)),
    collapsing("team-compiler/walk", "How to walk the scenarios", () =>
      WALK.map(([id, how]) => h("div", {}, strong(id), " ", how)))];
}
const WALK = [
  ["TC-GUI-04", "the rows: /3/ fully checked, /aco/ an archive (blue cells, one faint red), /adv/ with players 01 to 11 only (12 to 23 empty, no cell) and no keeper kit, /an/ part checked, part checking, part unchecked"],
  ["TC-GUI-07", "/egg/ is not on the teams list: its ID cell is red; type 950 and Enter (out of range), 714 and Enter (in use by /co/: the question takes the place of the row's cells until Yes or No), 850 and Enter (written); with the data folder read-only the entry is dropped"],
  ["TC-GUI-08", "/asp/ has no tag: Full or Midcup renames it and its cells appear"],
  ["TC-GUI-09", "hover a cell for its folder and findings; click it for the detail panel (/a/ 07 a warning, /a/ 15 a folder name with its one-click rename, /a/'s portraits an old file name in Portraits with its rename, which colours the portraits cell alone; /adv/ 11's portrait, kept in the player's folder, colours his cell and the portraits cell)"],
  ["TC-GUI-10", "a kit cell shows its two menu colours under its label; /a/ K2's black pair is the derived one"],
  ["TC-GUI-11", "the Compile button's arrow: the three modes (sideloading disabled with the sidebar at PES 2015 or 2016), the compile column and Open output folder; picking a mode relabels the main part, and sideloading adds Launch PES"],
  ["TC-GUI-12", "Compile: the cells change scattered over the grid, a few at once, as parallel tasks end; each turns bold, then green, amber or red (/adv/ 05, one file dropped); the strip counts; the button reads Cancel"],
  ["TC-GUI-13", "Cancel during a run: every cell returns to its check colour, the log's last line says so"],
  ["TC-GUI-14", "Compile with /egg/ or the two /biz/ exports unresolved: their cells are blocked, each tooltip naming the cause"],
  ["TC-GUI-15", "click a warning or error line of the log: its cell, or the row's label, is outlined"],
  ["TC-GUI-16", "the destination control: the badge beside the button, its tooltip the reason and the remedy"],
  ["live check", "Check every export again: every cell reads Checking, then flips to its result as its check ends, scattered over the rows"],
  ["drop", "drag a folder or a .zip from the desktop onto the grid (or use the two Drop controls): a new name is copied in and checked live; a name already there is refused, naming it"],
  ["Edit mode", "set the destination to \"locked: PES is running\" and Compile: once the CPKs are packed, the save's step asks whether to update the save; Update save does, Skip leaves it and logs why"],
  ["autorun", "Start as quick_compile.bat: the check runs, then Compile presses itself; with quick_compile_close_on_success ticked, the window would close at the end of a run with no error (the sample run ends with /adv/ done with errors, so it stays: untick /adv/ in the compile column first to see the close)"],
  ["row actions", "/b/ (old layout) offers Upgrade; /bant/ (NO_USE) is greyed with Enable; the two /biz/ exports clash: Disable on the folder, Move to unused on the archive; a right click on a row's label offers Compile only this export"],
];

const screen = () => "main";
function linkKeys() {
  const k = {};
  if (S.mode !== "normal") k.mode = S.mode;
  if (S.column) k.column = "1";
  if (D.empty) k.empty = "1";
  if (D.writable !== "yes") k.dest = D.writable;
  if (S.detail) k.detail = S.detail.row + "." + S.detail.what + (S.detail.slot ? "." + S.detail.slot : "");
  if (S.finished && !S.run) k.run = "done";
  return k;
}
const linkWarn = what => console.warn("team-compiler link: " + what);
function linkOpen(scr, keys) {
  if (scr !== "main") linkWarn("unknown screen " + scr);
  if (keys.mode !== undefined) { if (MODES[keys.mode]) S.mode = keys.mode; else linkWarn("mode=" + keys.mode); }
  if (keys.column !== undefined) S.column = keys.column === "1";
  if (keys.empty !== undefined) D.empty = keys.empty === "1";
  if (keys.dest !== undefined) { if (BADGES[keys.dest]) D.writable = keys.dest; else linkWarn("dest=" + keys.dest); }
  if (keys.detail !== undefined) {
    const [key, what, slot] = keys.detail.split(".");
    if (rowOf(key) && ["label", "player", "kit", "portraits"].includes(what)) S.detail = { row: key, what, slot };
    else linkWarn("detail=" + keys.detail);
  }
  // run=done: the finished run's colours, without the timer.
  if (keys.run === "done") {
    for (const r of S.rows) for (const cell of cellsOf(r)) {
      if (eligible(r) && (OUTCOME[cell.status] || cell.dropFile))
        cell.out = cell.dropFile ? "doneErr" : OUTCOME[cell.status];
    }
    S.last = "Compile finished in 00:42 · 1184 files written (612 MB)";
    S.finished = true;
  } else if (keys.run !== undefined) linkWarn("run=" + keys.run);
}

registerTool({ id: "team-compiler", css: TEAM_COMPILER_CSS, draw: drawView,
  controls: draftControls, screen, linkKeys, linkOpen,
  // The screens scripts/drafts/draft_shots.py captures, as the links this view writes.
  shots: ["team_compiler/main", "team_compiler/main?detail=a.player.15",
    "team_compiler/main?run=done", "team_compiler/main?mode=sideload&column=1",
    "team_compiler/main?empty=1", "team_compiler/main?dest=running"] });
})();
