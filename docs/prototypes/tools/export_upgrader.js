// The Export upgrader's draft (plans/export_upgrader.md "GUI"), a view of the
// drafts' host page, index.html. A second draft, for the maintainer to refine:
// the view reads the old_exports folder beside the app and writes each upgraded
// export into exports, in one of two looks by how many old exports it finds.
// - Five or more: the Team compiler's grid, three lines an export (each
//   player's face folder, and under it the boots and the gloves folder the save
//   has him wear), then the portraits, the kits and the team's files.
// - Four or fewer: each export as two columns, what the old export holds on the
//   left and where it goes in the upgraded one on the right, a row per
//   destination, each row turning as the run writes it.
// Both draw the same cells: a click on either opens the cell's detail. A toggle
// picks either look whatever the count.
// The first draft (two drop targets, one input, the left-behind list; kept in
// git history) is replaced. Not drawn yet from it: model-folder mode, the
// output-exists abort's button, Copy report, a referee export.
// Marked "sample": wording and looks the plan does not fix. The rows are
// samples built to show every state, not recorded runs; team names are
// teams_list.txt's, /egg/ a real team the bundled list does not hold.
// Drawn only with what egui can draw (docs/plans/core/gui.md "Prototypes
// before implementation"). Widget mapping: the grid, its cells, the legend, the
// toolbar and the detail window are the Team compiler's widgets (its draft's
// .tc- rules, one shared widget in the app); .eu-args = a Panel::top's row of
// a TextEdit, two buttons and a Checkbox; .eu-pair = a Grid of three columns,
// labels in the monospace font, the middle column's bracket three line
// segments the Painter draws; .eu-sq = a rect the Painter fills; a section's
// title = a weak label over a separator; <select> = a ComboBox.
// The wrapper keeps the view's names out of the host's scope; its body is left
// unindented, as in tools/save_editor.js.
// Names this script uses without declaring them (render, h, row, weak, strong,
// button, combo, cb, collapsing, window_, closeWindow, publishNotice,
// selectable, shellPesVersion, switchToTool, toolActivity, mmss, EXE_FOLDER, appendLog,
// logPanel, named, registerTool, CATALOG, closeMenus, inWin) are the host's,
// index.html.
(() => {
"use strict";

const EXPORT_UPGRADER_CSS = `
  /* The arguments: one row under the title bar, a line under it. */
  .eu-args { display: flex; align-items: center; gap: 8px; padding: 4px 8px; }
  .eu-args .fld { flex: 1; min-width: 120px; }
  .eu-args .fld input { width: 100%; box-sizing: border-box; }
  .eu-args .fld.bad input { border-color: var(--error); color: var(--error); }
  .eu-look { display: inline-flex; gap: 2px; margin-left: 8px; }
  /* A line's name, between the team's label and the first line. */
  .eu-line { width: 44px; padding: 0 4px !important; color: #6e6e6e; }
  .eu-grid tr.space { height: 6px; }
  .eu-grid .tc-cell.id4 { font-size: 9px; }
  .eu-grid .tc-cell.word { font-size: 10px; }
  .eu-msg { padding: 0 6px !important; }
  /* The two columns. Each export is a block: its two names, then its rows. */
  .eu-pairs { flex: 1; min-height: 0; overflow: auto; padding: 8px; }
  .eu-export { margin-bottom: 16px; }
  .eu-head { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .eu-pair { border-collapse: collapse; width: 100%; table-layout: fixed; }
  .eu-pair col.sq { width: 22px; } .eu-pair col.mid { width: 34px; }
  .eu-pair td { padding: 2px 4px; vertical-align: middle; overflow: hidden;
    text-overflow: ellipsis; white-space: nowrap; }
  .eu-pair tr.sec td { padding-top: 8px; color: #6e6e6e; border-bottom: 1px solid var(--stroke); }
  .eu-pair tr.item { cursor: pointer; }
  .eu-pair tr.item:hover { background: var(--faint); }
  .eu-pair tr.hl { outline: 2px solid var(--sel-fg); outline-offset: -2px; }
  /* A destination not written yet is weak; written, it takes the text colour. */
  .eu-pair td.dst { color: #6e6e6e; } .eu-pair tr.done td.dst { color: var(--text); }
  .eu-pair tr.busy td { font-weight: 700; }
  .eu-pair tr.err td.dst, .eu-pair tr.blocked td.dst { color: #8c2a2a; }
  .eu-pair .sub { color: #6e6e6e; }
  /* The middle column: several sources joined by a bracket into one arrow. */
  .eu-pair td.mid { text-align: center; color: #6e6e6e; position: relative; overflow: visible; }
  .eu-pair td.mid.many::before { content: ""; position: absolute; left: 2px; top: 22%;
    bottom: 22%; width: 6px; border: 1px solid #6e6e6e; border-left: 0; }
  .eu-sq { display: inline-block; width: 14px; height: 14px; border-radius: 2px;
    box-sizing: border-box; vertical-align: middle; position: relative; }
  .eu-sq.blocked { background: transparent !important; border: 1px dashed #8c2a2a; }
  .eu-sq.ol::after { content: ""; position: absolute; inset: 0; border-radius: 2px;
    box-shadow: inset 0 0 0 3px var(--ol); }
  .eu-detail { right: 16px; top: 64px; }
  .eu-detail .path { margin: 2px 0; white-space: normal; word-break: break-all; }
`;

// ---- Cell states: the Team compiler's colours, the upgrader's words ----
const WARN = "#dcc832", ERR = "#ff0000";
const COLOURS = { unchecked: "#0a0a0a", checking: "#dcdcdc", fullOk: "#800080",
  done: "#008000", doneWarn: WARN, doneErr: ERR };
const OUTLINES = { warning: WARN, error: ERR };
// sample: the states' names.
const STATE_NAMES = { unchecked: "Not read yet", checking: "Reading", fullOk: "Ready",
  warning: "Warning", error: "Error", done: "Upgraded", doneWarn: "Upgraded with warnings",
  doneErr: "Not written", blocked: "Not written" };
const OUTCOME = { fullOk: "done", warning: "doneWarn" };
function labelColour(hex) {
  const [r, g, b] = [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16));
  return 0.299 * r + 0.587 * g + 0.114 * b > 140 ? "#1b1b1b" : "#dcdcdc";
}

// ---- Sample exports ----
// A cell: its status, the codes of its findings (the Export upgrader's catalog
// rows), the old paths it is read from and the path it is written to.
const pad = n => String(n).padStart(2, "0");
const KIT_SLOTS = ["GK", "K1", "K2", "K3", "K4", "K5", "K6", "K7", "K8", "K9"];
const SLOTS = 23;
const cell = (src, dst, more = {}) => ({ status: "fullOk", codes: [], src: [].concat(src), dst, ...more });
const mark = (c, status, code, note) => Object.assign(c, { status, codes: code ? [code] : [], note });

// An old export, as the upgrader reads it. `items` are its boots and gloves
// folders, each with the players the save has wearing it: one wearer and the
// folder moves into his own, two or more and it stays shared with a link in
// each. `left` is what the upgrade leaves out, a Warning each.
function oldExport(key, name, o = {}) {
  const count = o.players || SLOTS;
  const player = n => `${pad(n)} - Player ${pad(n)}`;
  const x = { key, name, old: o.old || `${key} Aesthetics Export VGL26`, archive: !!o.archive,
    tag: o.tag || "Full", guess: o.guess || "the name carries no tag: Full",
    label: { status: "fullOk", codes: [] }, special: o.special || null,
    faces: {}, boots: {}, gloves: {}, kits: {}, files: {}, left: o.left || [], shared: [] };
  for (let n = 1; n <= count; n++)
    x.faces[n] = cell(`Faces\\XXX${pad(n)} - Player ${pad(n)}`, `Players\\${player(n)}`);
  for (const [kind, folder, list] of [["boots", "Boots", o.boots || []], ["gloves", "Gloves", o.gloves || []]])
    for (const [id, title, wearers] of list) {
      const src = `${folder}\\${kind[0] === "b" ? "k" : "g"}${id} - ${title}`;
      const own = wearers.length === 1;
      if (!own) x.shared.push({ kind, id, title, wearers,
        cell: cell(src, `${folder}\\${title}`, { id, title, wearers }) });
      for (const n of wearers)
        x[kind][n] = cell(src, own ? `Players\\${player(n)}` : `Players\\${player(n)}\\${title}.${kind}`,
          { id, title, wearers, link: !own });
    }
  for (const [slot, a, b] of o.kits || [["GK", "#202020", "#f0f0f0"], ["K1", "#c02020", "#f0f0f0"], ["K2", "#f0f0f0", "#2040a0"]]) {
    const stem = slot === "GK" ? "g1" : "p" + slot[1];
    x.kits[slot] = cell([`Kit Configs\\XXX\\XXX_DEF_${stem}_realUni.bin`, `Kit Textures\\u0XXX${stem}.dds`,
      "Team Note.txt: the kit's colours"], `Kits\\${stem}`, { swatch: [a, b] });
  }
  x.portraits = cell("Portraits\\player_XXXNN.dds", "Portraits\\NN.dds", { count: o.portraits ?? count });
  x.files.Note = cell(`${key} Note.txt`, "colors.txt, notes.txt");
  x.files.Logo = cell("Logo\\emblem_XXX_r_ll.png", "logo.png");
  x.files.Common = cell("Common", "Common");
  if (o.collars !== false) x.files.Collar = cell("Collars", "Collars");
  x.ids = cell("the boots and gloves folders' IDs", "legacy_ids.toml");
  return x;
}
const everyCell = x => [...Object.values(x.faces), ...Object.values(x.boots), ...Object.values(x.gloves),
  ...x.shared.map(s => s.cell), ...Object.values(x.kits), x.portraits, ...Object.values(x.files), x.ids];

const EGG_BOOTS = [["0915", "Crocs", [5]], ["0916", "Sandals", [7, 9]], ["0917", "Clogs", [11]],
  ["0918", "Skates", [14]], ["0920", "Wellies", [2, 3, 4]]];
const EGG_GLOVES = [["2635", "Mitts", [1]], ["2636", "Oven", [12, 23]]];
function sampleExports() {
  const egg = oldExport("EGG", "/egg/", { boots: EGG_BOOTS, gloves: EGG_GLOVES,
    left: [["Boots\\k0919 - Spare", "no player of the save wears boots 0919"],
      ["Faces\\XXX05 - Snuffy old", "a second face folder for player 05"],
      ["Other\\readme.txt", "a file the old compiler did not use"]] });
  mark(egg.faces[9], "warning", "roster_slot_empty", "The save's roster has no player on slot 09: the folder gets no settings.toml");
  const co = oldExport("co", "/co/", { old: "co Spring.zip", archive: true, tag: "Full",
    boots: [["1265", "Studs", [10]]], gloves: [["2985", "Latex", [1]]], collars: false,
    kits: [["GK", "#30a050", "#101010"], ["K1", "#f0f0f0", "#c02020"]] });
  const at = oldExport("@", "/@/", { old: "@  Aesthetics Export VGL26", special: "noTeam" });
  const a = oldExport("a", "/a/", { boots: [["0140", "Geta", [8, 9]]], tag: "Midcup", players: 11,
    guess: "the name says \"additions\": Midcup", old: "a additions VGL26" });
  mark(a.files.Note, "error", "unmigrated_content", "Line 14, \"Kit 2 colours: #GG0000\", does not read as colours");
  const an = oldExport("an", "/an/", { boots: [["0240", "Paws", [3]]] });
  for (const [n, c] of Object.entries(an.faces)) c.status = n <= 9 ? "fullOk" : n <= 16 ? "checking" : "unchecked";
  for (const c of everyCell(an)) if (!Object.values(an.faces).includes(c)) c.status = "unchecked";
  an.label.status = "checking";
  const b = oldExport("b", "/b/", { old: "b Full Aesthetics Export Autumn26", special: "newFormat" });
  const adv = oldExport("adv", "/adv/", { gloves: [["0740", "Gauntlets", [1]]] });
  mark(adv.kits.K2, "warning", "kit_without_config", "The Note gives this kit and Kit Configs holds no config for it: a placeholder the compiler fills");
  const biz = oldExport("biz", "/biz/", { boots: [["0890", "Loafers", [9, 10, 11]]], special: "exists" });
  const more = ["3", "aco", "bant", "d", "diy"].map((key, i) => oldExport(key, `/${key}/`, {
    boots: i % 2 ? [[`${pad(i)}40`, "Boots " + key, [4 + i]]] : [],
    gloves: i === 2 ? [[`${pad(i)}90`, "Gloves " + key, [1]]] : [] }));
  mark(more[3].portraits, "error", "portrait_conflict", "Portraits\\player_XXX04.dds and the face folder's own portrait differ");
  return { many: [more[0], a, more[1], adv, an, at, b, more[2], biz, co, more[3], more[4], egg], few: [egg, co, at] };
}

// ---- State ----
// D: the draft controls. S: the view's own state. L: the log panel's.
const D = { size: "many" };
const SETS = sampleExports();
// `view`: the look the member picked with the toggle, or null while the count
// of exports chooses (the grid from five on).
const S = { view: null, loose: false, savefile: "", savefileEdited: false, detail: null, highlight: null,
  run: null, last: null, finished: false, logScroll: 0,
  legendShut: localStorage.getItem("drafts.eu.legend_shut") === "1" };
const L = { log: [], logHeight: 160, logCollapsed: false, logFilter: "all", logStick: true, logNew: 0 };
const rows = () => D.size === "none" ? [] : SETS[D.size];
const DOCS = "C:\\Users\\you\\Documents\\KONAMI";
const savefile = () => S.savefileEdited ? S.savefile
  : `${DOCS}\\PRO EVOLUTION SOCCER ${shellPesVersion()}\\save\\EDIT00000000`;
const catalog = code => CATALOG["export-upgrader"].find(r => r.code === code);
const severity = code => (catalog(code) || {}).severity || "Error";
const messageOf = code => (catalog(code) ? catalog(code).condition.replaceAll("`", "") : code);

// What keeps an export from being read at all, by its `special`. sample wording.
const BLOCKS = {
  noTeam: x => `The save holds no team named ${x.name}`,
  newFormat: () => "Already in the UTW format",
  exists: x => `exports\\${outName(x)} already exists` };
// A team missing from the save needs no save under loose.
const blocked = x => x.special && !(x.special === "noTeam" && S.loose)
  ? BLOCKS[x.special](x) : null;
const hasError = x => everyCell(x).some(c => c.status === "error");
const eligible = x => !blocked(x) && !hasError(x) && !everyCell(x).some(c => ["unchecked", "checking"].includes(c.status));
const outName = x => {
  const [team, ...rest] = x.old.replace(/\.(zip|7z)$/, "").split(" ").filter(Boolean);
  const words = rest.filter(w => !/^(full|midcup|additions)$/i.test(w));
  return [team, x.tag, ...words].join(" ");
};
// What a cell shows: not written while its export cannot be (an error cell
// keeps its outline, so the eye finds what to fix), else the run's outcome,
// else what the reading found.
const shown = (x, c) => c === x.label ? labelOut(x) || c.status
  : blocked(x) || x.failed ? (c.status === "error" ? "error" : "blocked") : c.out || c.status;
function labelOut(x) {
  if (blocked(x)) return "error";
  if (x.failed) return "doneErr";
  const cells = everyCell(x);
  if (!cells.some(c => c.out) || cells.some(c => c.busy || !c.out)) return hasError(x) ? "error" : null;
  return cells.some(c => c.out === "doneWarn") || x.left.length ? "doneWarn" : "done";
}
function look(x, c) {
  const state = shown(x, c);
  const fill = OUTLINES[state] ? COLOURS.fullOk : COLOURS[state] || COLOURS.unchecked;
  return { state, fill, text: labelColour(fill), outline: OUTLINES[state] || null };
}
const lookStyle = l => `background:${l.fill}; color:${l.text}` + (l.outline ? `; --ol:${l.outline}` : "");
function cellTip(x, c) {
  const lines = [...c.src, "→ " + c.dst, STATE_NAMES[shown(x, c)]];
  if (blocked(x)) lines.push(blocked(x));
  else if (x.failed || hasError(x)) lines.push("An export with an Error is not written: fix the red cell first");
  if (c.note) lines.push(c.note);
  for (const code of c.codes) lines.push(`${severity(code)} - ${code}: ${messageOf(code)}`);
  return lines.join("\n");
}
const scopeOf = (x, what, slot) => `${x.name} ${what}${slot ? " " + slot : ""}`;

// ---- The grid (five exports or more) ----
function cellEl(x, c, text, what, slot, cls = "") {
  const l = look(x, c), scope = scopeOf(x, what, slot);
  const swatch = c.swatch && !["unchecked", "checking", "blocked"].includes(l.state);
  return h("div", { class: `tc-cell ${cls}` + (c.busy ? " busy" : "") + (l.state === "blocked" ? " blocked" : "")
      + (l.outline ? " ol" : "") + (swatch ? " kit" : "") + (S.highlight === scope ? " hl" : ""),
    style: lookStyle(l), title: cellTip(x, c), "data-scope": scope,
    onclick: () => { S.detail = { key: x.key, what, slot }; render(); } },
    text,
    swatch ? h("span", { class: "tc-sw", style: `background:linear-gradient(90deg, ${c.swatch[0]} 50%, ${c.swatch[1]} 50%)` }) : null);
}
const vline = () => h("td", { class: "tc-line" }, h("div", {}));
const gap = () => h("td", { class: "tc-gap" }, h("div", {}));
const slot = (kid, wide) => h("td", { class: "tc-slot" + (wide ? " wide" : "") }, kid || null);
// A wide cell over two kit columns.
const wideOver = kid => h("td", { colspan: "2" }, h("div", { class: "tc-slot wide" }, kid || null));
const RIGHT = 3 + KIT_SLOTS.length;

function labelCell(x, span) {
  const l = look(x, x.label);
  return h("td", { class: "tc-name mono" + (l.outline ? " ol" : "") + (S.highlight === x.name ? " hl" : ""),
    rowspan: String(span), style: lookStyle(l), "data-scope": x.name,
    title: [x.old + (x.archive ? " (an archive: extracted to read it)" : ""), "→ exports\\" + outName(x),
      STATE_NAMES[l.state], blocked(x) || ""].filter(Boolean).join("\n"),
    onclick: () => { S.detail = { key: x.key, what: "label" }; render(); } }, x.name);
}
// The row's own choice, right of the last line: Full or Midcup, prefilled with
// the name's guess. sample: a combo; the first draft had a pair of buttons.
function tagCell(x, span) {
  return h("td", { class: "tc-acts act", rowspan: String(span) },
    blocked(x) ? rowAct(x) : combo([["Full", "Full"], ["Midcup", "Midcup"]], x.tag,
      v => { x.tag = v; }, { title: "The upgraded export's name says Full or Midcup. Prefilled: " + x.guess, disabled: !!S.run }));
}
function rowAct(x) {
  if (x.special === "newFormat")
    return button("Move to exports", () => { SETS[D.size] = rows().filter(o => o !== x); render(); },
      { title: "It needs no upgrade: moves the folder into exports" });
  if (x.special === "exists")
    return button("Open the existing folder", () => window_("Open folder",
      h("p", {}, "The file manager opens here, on ", h("span", { class: "mono" }, `${EXE_FOLDER}\\exports\\${outName(x)}`), "."),
      button("Close", closeWindow), "open-folder"));
  return null;
}
function gridRows(x) {
  const why = blocked(x);
  if (why)
    return [h("tr", {}, labelCell(x, 1), h("td", { class: "eu-line" }), vline(),
      h("td", { class: "eu-msg", colspan: String(SLOTS + RIGHT), style: `color:${ERR}` }, why), vline(), tagCell(x, 1)),
      h("tr", { class: "space" })];
  const line = (name, kids) => [h("td", { class: "eu-line" }, name), vline(), ...kids, vline()];
  const item = kind => n => slot(x[kind][n]
    ? cellEl(x, x[kind][n], x[kind][n].id, kind, pad(n), "id4") : null);
  const each = fn => Array.from({ length: SLOTS }, (_, i) => fn(i + 1));
  const blank = n => Array.from({ length: n }, () => slot());
  const file = name => wideOver(x.files[name] ? cellEl(x, x.files[name], name, "file", name, "wide word") : null);
  const po = x.portraits;
  return [
    h("tr", {}, labelCell(x, 3), ...line("Face", [
      ...each(n => slot(x.faces[n] ? cellEl(x, x.faces[n], pad(n), "face", pad(n)) : null)), gap(),
      slot(cellEl(x, po, `${po.count}/${Object.keys(x.faces).length}`, "portraits", "", "wide"), true), gap(),
      ...KIT_SLOTS.map(k => slot(x.kits[k] ? cellEl(x, x.kits[k], k, "kit", k) : null))]), tagCell(x, 3)),
    h("tr", {}, ...line("Boots", [...each(item("boots")), gap(),
      slot(cellEl(x, x.files.Note, "Note", "file", "Note", "wide word"), true), gap(),
      file("Logo"), file("Common"), file("Collar"), ...blank(KIT_SLOTS.length - 6)])),
    h("tr", {}, ...line("Gloves", [...each(item("gloves")), gap(),
      slot(cellEl(x, x.ids, "IDs", "ids", "", "wide word"), true), gap(),
      // What the upgrade leaves out: one cell with the count, its list a click away.
      wideOver(x.left.length ? cellEl(x, leftCell(x), `${x.left.length} out`, "left", "", "wide word") : null),
      ...blank(KIT_SLOTS.length - 2)])),
    h("tr", { class: "space" })];
}
// The left-out entries as one cell: a Warning that drops nothing a player wore.
const leftCell = x => x.leftCell || (x.leftCell = { status: "warning", codes: ["leftover_dropped"],
  src: x.left.map(([path]) => path), dst: "not carried: listed here and in the log" });
function grid() {
  const th = (text, props = {}) => h("th", props, text);
  return h("table", { class: "tc-grid eu-grid", "data-panel": "grid" },
    h("tr", { class: "head" }, th("Team"), th(""), vline(), th("Players", { colspan: String(SLOTS) }), gap(),
      th(h("div", {}, "Portraits"), { class: "mid" }), gap(), th("Kits", { colspan: String(KIT_SLOTS.length) }),
      vline(), th("")),
    rows().flatMap(gridRows),
    h("tr", { class: "fill" }, h("td", { colspan: "2" }), vline(), h("td", { colspan: String(SLOTS + RIGHT) }), vline()));
}

// The legend: the compiler's, with this tool's states.
const LEGEND = [["unchecked", "checking"], ["warning", "error"], ["fullOk", "done"], ["doneWarn", "blocked"]];
const sq = (state, cls = "tc-cell") => {
  const fill = OUTLINES[state] ? COLOURS.fullOk : COLOURS[state] || "transparent";
  return h("span", { class: cls + (state === "blocked" ? " blocked" : "") + (OUTLINES[state] ? " ol" : ""),
    style: `background:${fill}` + (OUTLINES[state] ? `; --ol:${OUTLINES[state]}` : "") });
};
function legend() {
  const toggle = () => { S.legendShut = !S.legendShut;
    localStorage.setItem("drafts.eu.legend_shut", S.legendShut ? "1" : "0"); render(); };
  const title = h("span", { class: "both name", style: "cursor:pointer", onclick: toggle,
    title: S.legendShut ? "Show the legend" : "Hide the legend" },
    button(S.legendShut ? "▾" : "▴", null, { frame0: true }), "Legend");
  if (S.legendShut) return h("div", { class: "tc-legend shut top weak", "data-panel": "legend" }, title);
  return h("div", { class: "tc-legend top weak", "data-panel": "legend" }, title,
    LEGEND.flatMap(col => col.map(k => h("span", { class: "k" }, sq(k), STATE_NAMES[k]))));
}

// ---- The two columns (four exports or fewer) ----
// One row per destination: the old paths that end in it, the arrow, the new
// path and what it holds. Each row stands for the cells it is made of, so it
// turns as they do.
function pairRows(x) {
  const out = [];
  const sec = title => out.push({ sec: title });
  const add = (cells, what, slotName, dst, sub, src) =>
    out.push({ cells, what, slot: slotName, dst, sub, src: src || [...new Set(cells.flatMap(c => c.src))] });
  // sample: the sections' titles.
  sec("Players: a folder each, his looks read from the save into its settings.toml");
  for (const [n, face] of Object.entries(x.faces)) {
    const own = ["boots", "gloves"].map(k => x[k][n]).filter(Boolean);
    const links = own.filter(c => c.link).map(c => `${c.title}.${c.dst.split(".").pop()}`);
    const moved = own.filter(c => !c.link);
    add([face, ...own], "face", pad(n), face.dst,
      ["face", ...moved.map(c => c.dst === face.dst ? (x.boots[n] === c ? "boots" : "gloves") : "")].filter(Boolean).join(", ")
        + (links.length ? `; ${links.join(", ")}: a link to the shared folder` : ""),
      [face, ...moved].flatMap(c => c.src));
  }
  if (x.shared.length) {
    sec("Shared boots and gloves: worn by two or more players");
    for (const s of x.shared)
      add([s.cell], s.kind, pad(s.wearers[0]), s.cell.dst, "worn by " + s.wearers.map(pad).join(", "));
  }
  sec("Kits: texture, settings and colours together");
  for (const [k, c] of Object.entries(x.kits)) add([c], "kit", k, c.dst, k);
  sec("Team files");
  add([x.files.Note], "file", "Note", x.files.Note.dst, "team colours and notes, from the Note");
  add([x.portraits], "portraits", "", "Portraits\\01.dds … " + pad(x.portraits.count) + ".dds", "renamed by the player's number");
  for (const name of ["Logo", "Common", "Collar"])
    if (x.files[name]) add([x.files[name]], "file", name, x.files[name].dst, name === "Logo" ? "the team's emblem" : "as it was");
  add([x.ids], "ids", "", x.ids.dst, "the boots and gloves IDs the manager used");
  if (x.left.length) {
    sec("Not carried");
    for (const [path, why] of x.left) out.push({ cells: [leftCell(x)], what: "left", slot: "", src: [path], dst: "left out", sub: why, left: true });
  }
  return out;
}
// A row's state: the worst of its cells' while any is unread or wrong, the
// run's once every one of them is through it.
function rowState(x, r) {
  const states = r.cells.map(c => shown(x, c));
  for (const s of ["blocked", "error", "unchecked", "checking"]) if (states.includes(s)) return s;
  if (r.cells.some(c => c.busy)) return "busy";
  if (states.every(s => s === "done" || s === "doneWarn")) return states.includes("doneWarn") ? "doneWarn" : "done";
  return states.includes("warning") ? "warning" : "fullOk";
}
function pairBlock(x) {
  const why = blocked(x), l = look(x, x.label);
  const head = h("div", { class: "eu-head" },
    h("span", { class: "tc-name mono" + (l.outline ? " ol" : ""), style: lookStyle(l) + "; display:inline-flex; align-items:center; justify-content:center",
      title: STATE_NAMES[l.state] }, x.name),
    h("span", { class: "mono" }, x.old), weak("→"), h("span", { class: "mono" }, "exports\\" + outName(x)),
    why ? h("span", { style: `color:${ERR}` }, why) : null,
    h("span", { style: "flex:1" }),
    why ? rowAct(x) : combo([["Full", "Full"], ["Midcup", "Midcup"]], x.tag, v => { x.tag = v; },
      { title: "Prefilled: " + x.guess, disabled: !!S.run }));
  if (why) return h("div", { class: "eu-export", "data-panel": "export-" + x.key }, head);
  const trs = pairRows(x).map(r => {
    if (r.sec) return h("tr", { class: "sec" }, h("td", { colspan: "4" }, r.sec));
    const state = rowState(x, r), scope = scopeOf(x, r.what, r.slot);
    const cls = { done: "done", doneWarn: "done", busy: "busy", error: "err", blocked: "blocked" }[state] || "";
    return h("tr", { class: `item ${cls}` + (S.highlight === scope ? " hl" : ""), "data-scope": scope,
      title: r.cells.map(c => cellTip(x, c)).join("\n\n"),
      onclick: () => { S.detail = { key: x.key, what: r.what, slot: r.slot }; render(); } },
      h("td", {}, sq(state === "busy" ? "checking" : state, "eu-sq")),
      h("td", { class: "mono" }, r.src.map(p => h("div", {}, p))),
      h("td", { class: "mid" + (r.src.length > 1 ? " many" : "") }, r.left ? "✕" : "→"),
      h("td", { class: "mono dst" }, r.dst, r.sub ? h("span", { class: "sub" }, "  " + r.sub) : null));
  });
  return h("div", { class: "eu-export", "data-panel": "export-" + x.key }, head,
    h("table", { class: "eu-pair" },
      h("colgroup", {}, h("col", { class: "sq" }), h("col", {}), h("col", { class: "mid" }), h("col", {})),
      h("tr", { class: "sec" }, h("td", {}), h("td", {}, "Old export"), h("td", {}), h("td", {}, "Upgraded export")),
      trs));
}

// ---- The detail window: what a clicked cell is read from and written to ----
function detailPanel() {
  const d = S.detail, x = d && rows().find(o => o.key === d.key);
  if (!x) return null;
  const c = d.what === "label" ? null : d.what === "face" ? x.faces[+d.slot]
    : d.what === "boots" || d.what === "gloves" ? x[d.what][+d.slot]
    : d.what === "kit" ? x.kits[d.slot] : d.what === "file" ? x.files[d.slot]
    : d.what === "portraits" ? x.portraits : d.what === "ids" ? x.ids : leftCell(x);
  const title = d.what === "label" ? x.name : `${x.name} ${d.what}${d.slot ? " " + d.slot : ""}`;
  const paths = c ? [weak("From"), ...c.src.map(p => h("div", { class: "mono path" }, p)),
      weak("To"), h("div", { class: "mono path" }, c.dst)]
    : [weak("From"), h("div", { class: "mono path" }, "old_exports\\" + x.old),
      weak("To"), h("div", { class: "mono path" }, "exports\\" + outName(x))];
  const extra = [];
  if (c && c.wearers) extra.push(h("p", {}, c.wearers.length > 1
    ? `The save has players ${c.wearers.map(pad).join(", ")} wear them: the folder stays shared, a link in each player's folder.`
    : `The save has player ${pad(c.wearers[0])} alone wear them: they move into his folder.`));
  if (d.what === "left") extra.push(...x.left.map(([path, why]) => h("div", { class: "find" }, h("div", { class: "mono path" }, path), weak(why))));
  if (c && c.note) extra.push(h("p", {}, c.note));
  for (const code of c ? c.codes : x.label.codes)
    extra.push(h("div", { class: "find" }, strong(`${severity(code)} · ${code}`), h("div", {}, messageOf(code))));
  const state = c ? shown(x, c) : look(x, x.label).state;
  // sample: fixed at the panel's right; the compiler's is dragged by its bar.
  return h("div", { class: "tc-detail eu-detail", "data-panel": "detail" },
    h("div", { class: "bar" }, strong(title), h("span", { style: "flex:1" }),
      button("✕", () => { S.detail = null; render(); }, { frame0: true, title: "Close" })),
    h("div", { class: "body" }, h("p", {}, sq(state, "eu-sq"), " ", STATE_NAMES[state],
      blocked(x) ? h("div", { style: `color:${ERR}` }, blocked(x)) : null), ...paths, ...extra,
      row(button("Open the old folder", () => window_("Open folder",
        h("p", {}, "The file manager opens here, on ", h("span", { class: "mono" }, `${EXE_FOLDER}\\old_exports\\${x.old}`), "."),
        button("Close", closeWindow), "open-folder")))));
}

// ---- The arguments: the save the players' looks are read from, or none ----
const LOOSE_TIP = "Each boots or gloves folder goes to the player at its ID's position in the team's old block of 25 IDs, and no player's looks are read";
function argsRow() {
  const missing = !S.loose && rows().filter(x => x.special === "noTeam").map(x => x.name);
  const input = h("input", { type: "text", class: "mono", value: savefile(), disabled: S.loose || !!S.run,
    oninput: e => { S.savefile = e.target.value; S.savefileEdited = true; } });
  return h("div", { class: "eu-args top", "data-panel": "arguments" },
    h("span", {}, "Save"),
    h("span", { class: "fld" + (missing && missing.length ? " bad" : ""),
      title: S.loose ? "Not read while the box beside it is ticked"
        : missing && missing.length ? "It holds no team named " + missing.join(", ")
        : "The save the teams were made in: each player's skin, build and strip style, and who wears which boots and gloves, are read from it. It is never changed" }, input),
    button("Browse…", () => window_("Savefile", h("p", {}, "The system's file picker opens here, in the selected version's save folder."),
      button("Close", closeWindow), "pick-savefile"), { disabled: S.loose || !!S.run }),
    h("span", { title: LOOSE_TIP }, cb("Upgrade without the save", S.loose, v => { S.loose = v; })),
    weak("into"), h("span", { class: "mono", title: "Each upgraded export is written beside the others the Team compiler reads" },
      EXE_FOLDER + "\\exports"),
    // The look: the program's choice by the count of exports until the member
    // picks one, which then holds whatever the count (two selectable labels).
    // sample: the two names, and that the pick lasts for the session.
    h("span", { class: "eu-look", "data-panel": "look" },
      selectable("Grid", gridLook(), () => { S.view = "grid"; render(); },
        "One row of cells per export: the look for many exports"),
      selectable("Files", !gridLook(), () => { S.view = "files"; render(); },
        "Each export's old files beside where they go: the look for a few exports")));
}

// ---- The toolbar: the strip and the Upgrade button ----
function summary() {
  const all = rows(), count = (n, one, many = one) => n ? `${n} ${n === 1 ? one : many}` : null;
  return [count(all.length, "old export", "old exports"), count(all.filter(eligible).length, "ready"),
    count(all.filter(x => !blocked(x) && hasError(x)).length, "with errors"),
    count(all.filter(x => blocked(x)).length, "cannot be read", "cannot be read"),
    count(all.filter(x => eligible(x) && (x.left.length || everyCell(x).some(c => c.status === "warning"))).length, "with warnings")]
    .filter(Boolean).join(" · ");
}
function toolbar() {
  const run = S.run, can = rows().some(eligible);
  const strip = run ? `Upgrading · ${mmss(run.elapsed)} · ${run.done} of ${run.tasks.length} folders written`
    : [S.last, summary()].filter(Boolean).join("  —  ");
  const main = button(run ? "Cancel" : "Upgrade", run ? cancelRun : askRun,
    { disabled: !run && !can, title: !run && !can ? "No old export is ready to upgrade" : null });
  main.id = "upgrade-btn";
  const arrow = button("⏷", e => { e.stopPropagation(); moreMenu(e.currentTarget); }, { disabled: !!run });
  return h("div", { class: "tc-bar", "data-panel": "run" },
    h("span", { class: "strip mono", id: "upgrade-progress" }, strip),
    S.finished && !run ? button("Open the Team compiler", () => switchToTool("team-compiler"),
      { title: "The upgraded exports are in its exports folder: it checks them like any other" }) : null,
    h("div", { class: "tc-split" }, main, arrow));
}
function moreMenu(btn) {
  closeMenus();
  const r = btn.getBoundingClientRect(), [left, top] = inWin(r.right, r.top);
  const item = (text, what) => h("button", { onclick: ev => { ev.stopPropagation(); closeMenus();
    window_(text, h("p", {}, what), button("Close", closeWindow), "more"); } }, text);
  const m = h("div", { class: "menu", style: `left:${left}px; top:${top}px` },
    item("Open old_exports folder", `The file manager opens on ${EXE_FOLDER}\\old_exports.`),
    item("Open exports folder", `The file manager opens on ${EXE_FOLDER}\\exports.`),
    h("div", { class: "sep", style: "margin:5.5px 0" }),
    item("Upgrade model folders…", "Model-folder mode (a folder of old face, boots or gloves folders, upgraded on their own) is not drawn in this draft yet."));
  document.getElementById("win").append(m);
  m.style.left = left - m.offsetWidth + "px"; m.style.top = top - m.offsetHeight + "px";
}

// Upgrading without the save asks first, every time (the plan's dialog).
function askRun() {
  if (!S.loose) return startRun();
  let ok = false;
  const go = button("Upgrade", () => { closeWindow(); startRun(); }, { disabled: true });
  window_("Upgrade without the save",
    h("div", {}, h("p", {}, "The save is where the players' looks live: skin colour, build, strip style, the in-game faces. The old exports hold none of it."),
      h("p", {}, "Without the save none is carried, and a compile of the result gives those players the default look. To keep them, cancel and pick the save the teams were made in."),
      h("label", { class: "cb" }, h("input", { type: "checkbox", onchange: e => { ok = e.target.checked; go.disabled = !ok; } }),
        "I understand the players' looks will be lost")),
    [go, button("Cancel", closeWindow)], "loose-ask");
}
// The cells change scattered over the exports, a few at once, as the
// parallel tasks end; an export with an Error is not written at all.
const BUSY = 6;
function startRun() {
  if (S.run) return;
  for (const x of rows()) { delete x.failed; for (const c of everyCell(x)) { delete c.out; delete c.busy; } }
  const lines = [];
  for (const x of rows()) if (!blocked(x) && hasError(x)) { x.failed = true;
    lines.push({ severity: "Error", code: "migration_not_clean", scope: x.name,
      text: `Error - ${x.name} - migration_not_clean: nothing written; fix the Error and upgrade again` }); }
  const ready = rows().filter(eligible);
  const tasks = ready.flatMap(x => everyCell(x).map(c => ({ x, c })))
    .map((t, i) => ({ ...t, order: (i * 7919) % 211 })).sort((a, b) => a.order - b.order);
  const start = Date.now();
  S.finished = false; S.last = null;
  S.run = { tasks, at: 0, done: 0, elapsed: 0, exports: ready.length, failed: lines.length };
  appendLog([{ severity: "Info", code: null, scope: null, text: `Info - Upgrade: ${ready.length} exports` }, ...lines], L);
  S.run.timer = setInterval(() => {
    const run = S.run;
    if (!run) return;
    run.elapsed = Math.floor((Date.now() - start) / 1000);
    const ended = run.tasks[run.at - BUSY];
    if (ended) { delete ended.c.busy; ended.c.out = OUTCOME[ended.c.status]; run.done++; }
    const next = run.tasks[run.at++];
    if (next) next.c.busy = true;
    else if (run.at - BUSY >= run.tasks.length) return endRun();
    toolActivity("export-upgrader", { done: run.done, total: run.tasks.length, elapsed: run.elapsed });
    render();
  }, 40);
  render();
}
function stopRun() { clearInterval(S.run.timer); toolActivity("export-upgrader", null); }
function endRun() {
  const run = S.run;
  stopRun(); S.run = null; S.finished = true;
  S.last = `Upgrade finished in ${mmss(run.elapsed)} · ${run.exports} written to exports`
    + (run.failed ? ` · ${run.failed} not written` : "");
  appendLog(rows().filter(x => labelOut(x) === "done" || labelOut(x) === "doneWarn").flatMap(x => [
    ...x.left.map(([path, why]) => ({ severity: "Warning", code: "leftover_dropped", scope: scopeOf(x, "left"),
      text: `Warning - ${x.name} - leftover_dropped at ${path} (${why})` })),
    { severity: "Info", code: "migration_clean", scope: x.name, text: `Info - ${x.name} - migration_clean: exports\\${outName(x)}` }]), L);
  publishNotice(`Upgraded ${run.exports} exports`, { label: "Open exports folder", run: () => {} }, "success");
  render();
}
function cancelRun() {
  for (const x of rows()) { delete x.failed; for (const c of everyCell(x)) { delete c.busy; delete c.out; } }
  stopRun(); S.run = null;
  S.last = "Cancelled: the exports begun were removed";
  appendLog([{ severity: "Warning", code: null, scope: null, text: "Warning - The run was cancelled" }], L);
  render();
}

// The reading's findings as log lines, once: a click on one outlines its cell or row.
function seedLog() {
  L.log = [];
  for (const x of rows()) {
    if (blocked(x)) L.log.push({ severity: "Error", code: null, scope: x.name, text: `Error - ${x.name} - ${blocked(x)}` });
    const each = (c, scope) => { for (const code of c.codes) L.log.push({ severity: severity(code), code, scope,
      text: `${severity(code)} - ${scope} - ${code}: ${c.note || messageOf(code)}` }); };
    for (const [n, c] of Object.entries(x.faces)) each(c, scopeOf(x, "face", pad(n)));
    for (const [k, c] of Object.entries(x.kits)) each(c, scopeOf(x, "kit", k));
    for (const [k, c] of Object.entries(x.files)) each(c, scopeOf(x, "file", k));
    each(x.portraits, scopeOf(x, "portraits"));
  }
}
seedLog();
function gotoCell(scope) {
  S.highlight = scope;
  render();
  const el = document.querySelector(`#center [data-scope="${scope}"]`);
  if (el) el.scrollIntoView({ block: "nearest", inline: "nearest" });
}

// ---- The view ----
// sample: the count at which the look changes (five), and that it follows
// the folder with no switch of its own.
const GRID_FROM = 5;
const gridLook = () => S.view ? S.view === "grid" : rows().length >= GRID_FROM;
function drawView(center) {
  const oldLog = center.querySelector(".log-body");
  if (oldLog) S.logScroll = oldLog.scrollTop;
  const oldPairs = center.querySelector(".eu-pairs"), pairsTop = oldPairs ? oldPairs.scrollTop : 0;
  center.innerHTML = "";
  center.classList.add("tc-view");
  const all = rows(), many = gridLook();
  const body = !all.length
    ? h("div", { class: "tc-top" }, h("div", { class: "pipe-grid" }, h("div", { class: "tc-empty", "data-panel": "empty" },
        h("p", {}, strong("No old exports found")), h("p", { class: "mono" }, EXE_FOLDER + "\\old_exports"),
        h("p", {}, "Drop an old export here: a folder, a .zip or a .7z."))))
    : many ? h("div", { class: "tc-top" }, h("div", { class: "pipe-grid" }, grid()))
    : h("div", { class: "eu-pairs", "data-panel": "pairs" }, all.map(pairBlock));
  center.append(h("div", { class: "pipe" }, argsRow(), many ? legend() : null, body, toolbar(),
    named(logPanel(L, { tool: "export-upgrader", onLine: gotoCell }), "log"), detailPanel()));
  const lb = center.querySelector(".log-body");
  if (lb) lb.scrollTop = L.logStick ? lb.scrollHeight : S.logScroll;
  const pairs = center.querySelector(".eu-pairs");
  if (pairs) pairs.scrollTop = pairsTop;
}

// ---- Draft controls ----
function reset() {
  Object.assign(SETS, sampleExports());
  S.detail = null; S.highlight = null; S.last = null; S.finished = false; S.run = null;
  seedLog();
}
function draftControls() {
  return [
    h("label", { class: "cb" }, "old_exports holds",
      combo([["many", "13 exports (the grid)"], ["few", "3 exports (two columns)"], ["none", "nothing"]],
        D.size, v => { D.size = v; S.detail = null; seedLog(); })),
    button("Reset the samples", () => { reset(); render(); }),
    collapsing("export-upgrader/walk", "What to look at", () => WALK.map(([what, how]) => h("div", {}, strong(what), " ", how)))];
}
const WALK = [
  ["the grid", "three lines an export: a player's face folder, and under it the boots and gloves folders the save has him wear, by their four-digit ID (/egg/: 0916 under 07 and 09, shared; 0915 under 05, his alone)"],
  ["the team's files", "right of the portraits and the kits: the Note, the logo, Common, Collars, the IDs kept in legacy_ids.toml, and a count of what is left out"],
  ["an Error", "/a/'s Note has a line that does not read: outlined red; Upgrade writes nothing of /a/ and its cells turn to the dashed Not written"],
  ["cannot be read", "/@/ is not in the save (tick Upgrade without the save and it is read), /b/ is already upgraded, /biz/'s output already exists: one line each, with what to do"],
  ["Full or Midcup", "the combo at each row's right, prefilled from the name (/a/: Midcup, from \"additions\")"],
  ["the two columns", "set old_exports to 3 exports: a row per destination, the old paths left and the new right; Upgrade turns each row as it is written"],
  ["the toggle", "Grid and Files at the arguments' right end: either look for any number of exports; until one is picked the count chooses"],
  ["a click", "on a cell, a row or a log line: the detail window, with the old and new paths and the findings"],
];

const screen = () => "main";
function linkKeys() {
  const k = {};
  if (D.size !== "many") k.exports = D.size;
  if (S.view) k.look = S.view;
  if (S.loose) k.loose = "1";
  if (S.detail) k.detail = [S.detail.key, S.detail.what, S.detail.slot || ""].join(".");
  if (S.finished && !S.run) k.run = "done";
  return k;
}
const linkWarn = what => console.warn("export-upgrader link: " + what);
function linkOpen(scr, keys) {
  if (scr !== "main") linkWarn("unknown screen " + scr);
  if (keys.exports !== undefined) { if (["many", "few", "none"].includes(keys.exports)) { D.size = keys.exports; seedLog(); } else linkWarn("exports=" + keys.exports); }
  if (keys.look !== undefined) { if (["grid", "files"].includes(keys.look)) S.view = keys.look; else linkWarn("look=" + keys.look); }
  if (keys.loose !== undefined) S.loose = keys.loose === "1";
  if (keys.detail !== undefined) {
    const [key, what, slotName] = keys.detail.split(".");
    if (rows().some(x => x.key === key)) S.detail = { key, what, slot: slotName }; else linkWarn("detail=" + keys.detail);
  }
  // run=done: the finished run's colours, without the timer.
  if (keys.run === "done") {
    for (const x of rows()) {
      if (!blocked(x) && hasError(x)) x.failed = true;
      if (eligible(x)) for (const c of everyCell(x)) c.out = OUTCOME[c.status];
    }
    S.last = "Upgrade finished in 00:09 · " + rows().filter(eligible).length + " written to exports";
    S.finished = true;
  } else if (keys.run !== undefined) linkWarn("run=" + keys.run);
}

registerTool({ id: "export-upgrader", css: EXPORT_UPGRADER_CSS, draw: drawView,
  controls: draftControls, screen, linkKeys, linkOpen,
  // The screens scripts/drafts/draft_shots.py captures, as the links this view writes.
  shots: ["export_upgrader/main", "export_upgrader/main?run=done", "export_upgrader/main?detail=EGG.boots.07",
    "export_upgrader/main?exports=few", "export_upgrader/main?exports=few&run=done",
    "export_upgrader/main?exports=none"] });
})();
