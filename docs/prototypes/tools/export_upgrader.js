// The Export upgrader's draft (plans/export_upgrader.md "GUI"), a view of the
// drafts' host page, index.html. A second draft, for the maintainer to refine:
// the view reads the old_exports folder beside the app and writes each upgraded
// export into exports, in one of two looks by how many old exports it finds.
// - Five or more: the Team compiler's grid, three lines an export (each
//   player's face folder, and under it the boots and the gloves folder the save
//   has him wear), then the portraits, the kits and the team's files.
// - Four or fewer: each export as the explanatory page's map (docs/web/
//   upgrade_explained.html), a box per folder of the old export on the left, a
//   box per folder of the upgraded one on the right and a line from each to
//   where it goes, lighting up as the run writes it.
// Both draw the same cells: a click on either opens the cell's detail. A toggle
// picks either look whatever the count.
// The first draft (two drop targets, one input, the left-behind list; kept in
// git history) is replaced. Not drawn yet from it: a referee export.
// Model-folder mode is removed (plans/export_upgrader.md "Model-folder mode").
// Marked "sample": wording and looks the plan does not fix. The rows are
// samples built to show every state, not recorded runs; team names are
// teams_list.txt's, /egg/ a real team the bundled list does not hold.
// Drawn only with what egui can draw (docs/plans/core/gui.md "Prototypes
// before implementation"). Widget mapping: the grid, its cells, the legend, the
// toolbar and the detail window are the Team compiler's widgets (its draft's
// .tc- rules, one shared widget in the app); .eu-args = a Panel::top's row of
// a TextEdit, two buttons and a Checkbox; .eu-box = a rect the Painter fills
// and strokes with its text painted over it, Sense::click and hover; an
// .edge = a CubicBezierShape the Painter draws under the boxes; .eu-sq = a
// rect the Painter fills; <select> = a ComboBox.
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
  button.eu-next { background: var(--sel-bg); color: var(--sel-fg); }
  .eu-msg .row { flex-wrap: nowrap; }
  /* A line's name, between the team's label and the first line. */
  .eu-line { width: 44px; padding: 0 4px !important; color: #6e6e6e; }
  .eu-grid tr.space { height: 6px; }
  .eu-grid .tc-cell.id4 { font-size: 9px; }
  .eu-grid .tc-cell.word { font-size: 10px; }
  .eu-msg { padding: 0 6px !important; }
  /* The files look. Each export is a block: its two names, then its map.
     The two columns' labels are one row that stays at the top while the
     exports scroll under it (egui: a row drawn above the scroll area). */
  .eu-pairs { flex: 1; min-height: 0; overflow: auto; padding: 0 8px 8px; }
  .eu-export { margin-bottom: 20px; }
  .eu-head { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
  .eu-cols { display: flex; justify-content: space-between; position: sticky; top: 0; z-index: 2;
    background: var(--panel); padding: 8px 0 4px; }
  .eu-cols span { width: 38%; }
  .eu-flow { position: relative; --players: #3987e5; --boots: #d95926; --kits: #199e70;
    --plain: #8a8a8a; --out: #8c2a2a; }
  .eu-flow svg { position: absolute; left: 0; top: 0; width: 100%; height: 100%; }
  /* A line rests dim, lights once its destination is written, and is thick
     while it is being written. */
  .eu-flow .edge { fill: none; stroke-width: 1.5; opacity: 0.4; vector-effect: non-scaling-stroke; }
  .eu-flow .edge.players { stroke: var(--players); } .eu-flow .edge.boots { stroke: var(--boots); }
  .eu-flow .edge.kits { stroke: var(--kits); } .eu-flow .edge.plain { stroke: var(--plain); }
  .eu-flow .edge.out { stroke: var(--out); stroke-dasharray: 5 4; }
  .eu-flow .edge.info { stroke-dasharray: 6 5; }
  .eu-flow .edge.lit { opacity: 1; } .eu-flow .edge.busy { opacity: 1; stroke-width: 3; }
  /* A box: the panel's fill, a stroke, a stripe in its lines' colour. */
  .eu-box { position: absolute; width: 38%; height: 22px; box-sizing: border-box; display: flex;
    align-items: center; gap: 6px; padding: 0 6px 0 10px; background: var(--faint);
    border: 1px solid var(--stroke); border-radius: 2px; cursor: pointer; overflow: hidden;
    white-space: nowrap; }
  .eu-box.left { left: 0; } .eu-box.right { right: 0; }
  .eu-box::before { content: ""; position: absolute; left: 0; top: 0; bottom: 0; width: 4px;
    background: var(--plain); }
  .eu-box.players::before { background: var(--players); } .eu-box.boots::before { background: var(--boots); }
  .eu-box.kits::before { background: var(--kits); }
  .eu-box.out::before { background: var(--out); }
  .eu-box.out { border-style: dashed; }
  .eu-box .text { overflow: hidden; text-overflow: ellipsis; }
  .eu-box .sub { color: #6e6e6e; overflow: hidden; text-overflow: ellipsis; flex: 1; text-align: right; }
  /* A destination not written yet is weak; written, it takes the text colour. */
  .eu-box.right .text { color: #6e6e6e; } .eu-box.right.lit .text { color: var(--text); }
  .eu-box.busy { border-color: var(--strong); } .eu-box.busy .text { font-weight: 700; color: var(--strong); }
  .eu-box:hover { border-color: var(--sel-fg); }
  .eu-box.hl { outline: 2px solid var(--sel-fg); }
  .eu-flow.focus .eu-box:not(.on), .eu-flow.focus .edge:not(.on) { opacity: 0.15; }
  .eu-flow.focus .edge.on { opacity: 1; stroke-width: 2.5; }
  .eu-flow-legend { display: flex; gap: 16px; padding: 4px 8px; }
  .eu-flow-legend .k { display: inline-flex; align-items: center; gap: 4px; }
  .eu-ln { display: inline-block; width: 22px; height: 0; border-top: 2px solid #8a8a8a; }
  .eu-ln.players { border-color: #3987e5; } .eu-ln.boots { border-color: #d95926; }
  .eu-ln.kits { border-color: #199e70; } .eu-ln.out { border-color: #8c2a2a; border-top-style: dashed; }
  .eu-ln.info { border-top-style: dashed; }
  .eu-sq { display: inline-block; width: 14px; height: 14px; border-radius: 2px;
    box-sizing: border-box; vertical-align: middle; position: relative; }
  .eu-sq.blocked { background: transparent !important; border: 1px dashed #8c2a2a; }
  .eu-sq.ol::after { content: ""; position: absolute; inset: 0; border-radius: 2px;
    box-shadow: inset 0 0 0 3px var(--ol); }
  .eu-detail { right: 16px; top: 64px; }
  .eu-detail .path { margin: 2px 0; white-space: normal; word-break: break-all; }
`;

// ---- Cell states: the Team compiler's colours, the upgrader's words ----
const WARN = "#d29922", ERR = "#f85149";
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
  // 125, not the usual 140: the error red (#f85149) weighs 130 and takes the
  // dark grey, light text on it reading poorly; no other state colour lies between.
  return 0.299 * r + 0.587 * g + 0.114 * b > 125 ? "#1b1b1b" : "#dcdcdc";
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
    combo([["Full", "Full"], ["Midcup", "Midcup"]], x.tag,
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
      h("td", { class: "eu-msg", colspan: String(SLOTS + RIGHT) },
        row(h("span", { style: `color:${ERR}` }, why), rowAct(x))), vline(), h("td", {})),
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
      vline(), th("Export type")),
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

// ---- The files look (four exports or fewer): boxes and lines ----
// The explanatory page's map, drawn for the export at hand: a box per folder
// or file of the old export on the left, a box per folder or file of the
// upgraded one on the right, and a line from each box to where it goes, in
// the colour of what it carries. A box under the pointer keeps its own lines
// and dims the rest. During a run a destination and its lines light up as it
// is written.
const BOX_H = 22, BOX_GAP = 3, SEC_GAP = 12;
// One export's boxes and lines. A node: its side, text, kind (the line's
// colour), the cells it stands for (its state), and what a click opens.
function flowOf(x) {
  const L = [], R = [], edges = [];
  const node = (side, id, text, kind, cells, open, more = {}) => {
    const n = { id: `${x.key}:${side === L ? "L" : "R"}:${id}`, text, kind, cells, open, ...more };
    side.push(n);
    return n;
  };
  const edge = (from, to, kind, info) => edges.push({ a: from, b: to, kind, info });
  const gapAfter = side => { if (side.length) side[side.length - 1].gap = true; };
  // The players: a face folder each, to his own folder.
  const P = {};
  for (const [n, face] of Object.entries(x.faces)) {
    const own = ["boots", "gloves"].map(k => x[k][n]).filter(Boolean);
    const links = own.filter(c => c.link).map(c => `${c.title}.${c.dst.split(".").pop()}`);
    const moved = own.filter(c => !c.link).map(c => (x.boots[n] === c ? "boots" : "gloves"));
    P[n] = node(R, "p" + n, face.dst, "players", [face, ...own], { what: "face", slot: pad(n) },
      { sub: ["face", ...moved, ...links].join(", ") });
    edge(node(L, "f" + n, face.src[0], "players", [face], { what: "face", slot: pad(n) }), P[n], "players");
  }
  gapAfter(L); gapAfter(R);
  // The boots and gloves folders: to each wearer, to the shared folder when
  // two or more wear them, and their IDs to legacy_ids.toml.
  const items = {};
  for (const kind of ["boots", "gloves"])
    for (const [n, c] of Object.entries(x[kind])) {
      const key = c.src[0];
      if (!items[key]) items[key] = node(L, key, key, "boots", [], { what: kind, slot: pad(n) });
      items[key].cells.push(c);
      edge(items[key], P[n], "boots");
    }
  for (const s of x.shared) {
    items[s.cell.src[0]].cells.push(s.cell);
    edge(items[s.cell.src[0]], node(R, "s" + s.id, s.cell.dst, "boots", [s.cell],
      { what: s.kind, slot: pad(s.wearers[0]) }, { sub: "worn by " + s.wearers.map(pad).join(", ") }), "boots");
  }
  gapAfter(L); gapAfter(R);
  // The kits: configs, textures and the Note's colours, into a folder a kit.
  const kitCells = Object.values(x.kits), anyKit = { what: "kit", slot: Object.keys(x.kits)[0] };
  const configs = node(L, "kc", "Kit Configs\\", "kits", kitCells, anyKit);
  const textures = node(L, "kt", "Kit Textures\\", "kits", kitCells, anyKit);
  const note = node(L, "note", x.files.Note.src[0], "plain", [x.files.Note], { what: "file", slot: "Note" });
  for (const [k, c] of Object.entries(x.kits)) {
    const kitBox = node(R, "k" + k, c.dst, "kits", [c], { what: "kit", slot: k }, { sub: k + ": texture, settings, colours" });
    edge(configs, kitBox, "kits"); edge(textures, kitBox, "kits"); edge(note, kitBox, "plain", true);
  }
  gapAfter(R);
  edge(note, node(R, "colors", x.files.Note.dst, "plain", [x.files.Note], { what: "file", slot: "Note" }), "plain");
  edge(node(L, "po", "Portraits\\", "plain", [x.portraits], { what: "portraits", slot: "" }),
    node(R, "po", `Portraits\\01.dds … ${pad(x.portraits.count)}.dds`, "plain", [x.portraits], { what: "portraits", slot: "" }), "plain");
  for (const name of ["Logo", "Common", "Collar"]) if (x.files[name])
    edge(node(L, name, x.files[name].src[0], "plain", [x.files[name]], { what: "file", slot: name }),
      node(R, name, x.files[name].dst, "plain", [x.files[name]], { what: "file", slot: name }), "plain");
  if (Object.keys(items).length) {
    const ids = node(R, "ids", x.ids.dst, "boots", [x.ids], { what: "ids", slot: "" }, { sub: "the boots and gloves IDs" });
    for (const item of Object.values(items)) edge(item, ids, "boots", true);
  }
  // What is left out: dashed boxes, to one dashed box.
  if (x.left.length) {
    gapAfter(L); gapAfter(R);
    const out = node(R, "left", "Not carried", "out", [leftCell(x)], { what: "left", slot: "" }, { sub: "left out, listed in the log" });
    for (const [path] of x.left) edge(node(L, "x" + path, path, "out", [leftCell(x)], { what: "left", slot: "" }), out, "out");
  }
  let y = 0;
  for (const side of [L, R]) { y = 0; for (const n of side) { n.y = y; y += BOX_H + BOX_GAP + (n.gap ? SEC_GAP : 0); } side.height = y; }
  return { L, R, edges, height: Math.max(L.height, R.height) };
}
// A box's state: the worst of its cells' while any is unread or wrong, the
// run's once every one of them is through it.
function nodeState(x, n) {
  const states = n.cells.map(c => shown(x, c));
  for (const s of ["blocked", "error", "unchecked", "checking"]) if (states.includes(s)) return s;
  if (n.cells.some(c => c.busy)) return "busy";
  if (states.every(s => s === "done" || s === "doneWarn")) return states.includes("doneWarn") ? "doneWarn" : "done";
  return states.includes("warning") ? "warning" : "fullOk";
}
const SVG = "http://www.w3.org/2000/svg";
function flowBlock(x) {
  const why = blocked(x), l = look(x, x.label);
  const head = h("div", { class: "eu-head" },
    h("span", { class: "tc-name mono" + (l.outline ? " ol" : ""), style: lookStyle(l) + "; display:inline-flex; align-items:center; justify-content:center",
      title: STATE_NAMES[l.state] }, x.name),
    h("span", { class: "mono" }, x.old), weak("→"), h("span", { class: "mono" }, "exports\\" + outName(x)),
    why ? h("span", { style: `color:${ERR}` }, why) : null, why ? rowAct(x) : null,
    h("span", { style: "flex:1" }),
    why ? null : combo([["Full", "Full"], ["Midcup", "Midcup"]], x.tag, v => { x.tag = v; },
      { title: "Prefilled: " + x.guess, disabled: !!S.run }));
  if (why) return h("div", { class: "eu-export", "data-panel": "export-" + x.key }, head);
  const flow = flowOf(x);
  const map = h("div", { class: "eu-flow", style: `height:${flow.height}px` });
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("viewBox", `0 0 1000 ${flow.height}`);
  svg.setAttribute("preserveAspectRatio", "none");
  const mid = n => n.y + BOX_H / 2;
  const written = n => ["done", "doneWarn"].includes(nodeState(x, n));
  for (const e of flow.edges) {
    const path = document.createElementNS(SVG, "path");
    path.setAttribute("d", `M380 ${mid(e.a)} C500 ${mid(e.a)} 500 ${mid(e.b)} 620 ${mid(e.b)}`);
    path.setAttribute("class", `edge ${e.kind}` + (e.info ? " info" : "") + (written(e.b) ? " lit" : "")
      + (nodeState(x, e.b) === "busy" ? " busy" : ""));
    path.dataset.a = e.a.id; path.dataset.b = e.b.id;
    svg.append(path);
  }
  map.append(svg);
  // The pointer on a box: its lines and the boxes at their other ends stay,
  // the rest dims (egui: the hovered box's id, read while painting).
  // Leaving waits a moment, as on the explanatory page, so moving from one
  // box to the next does not flicker.
  let leave = null;
  const focus = id => {
    clearTimeout(leave);
    const on = new Set([id]);
    for (const path of svg.children) {
      const mine = path.dataset.a === id || path.dataset.b === id;
      path.classList.toggle("on", mine);
      if (mine) { on.add(path.dataset.a); on.add(path.dataset.b); }
    }
    for (const box of map.querySelectorAll(".eu-box")) box.classList.toggle("on", on.has(box.dataset.node));
    map.classList.add("focus");
  };
  const box = (n, right) => {
    const state = nodeState(x, n), scope = scopeOf(x, n.open.what, n.open.slot);
    return h("div", { class: `eu-box ${n.kind} ${right ? "right" : "left"}` + (right && written(n) ? " lit" : "")
        + (state === "busy" ? " busy" : "") + (S.highlight === scope ? " hl" : ""),
      style: `top:${n.y}px`, "data-node": n.id, "data-scope": scope,
      title: [...new Set(n.cells.map(c => cellTip(x, c)))].slice(0, 3).join("\n\n"),
      onmouseenter: () => focus(n.id), onmouseleave: () => { leave = setTimeout(() => map.classList.remove("focus"), 120); },
      onclick: () => { S.detail = { key: x.key, ...n.open }; render(); } },
      sq(state === "busy" ? "checking" : state, "eu-sq"), h("span", { class: "mono text" }, n.text),
      n.sub ? h("span", { class: "sub" }, n.sub) : null);
  };
  map.append(...flow.L.map(n => box(n, false)), ...flow.R.map(n => box(n, true)));
  return h("div", { class: "eu-export", "data-panel": "export-" + x.key }, head, map);
}
// The lines' colours, under the files: what each carries.
function flowLegend() {
  const line = (kind, text, info) => h("span", { class: "k" },
    h("span", { class: `eu-ln ${kind}` + (info ? " info" : "") }), text);
  return h("div", { class: "eu-flow-legend top weak", "data-panel": "legend" },
    line("players", "players"), line("boots", "boots and gloves"), line("kits", "kits"),
    line("plain", "team files"), line("out", "not carried"), line("plain", "information read", true));
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
    weak("into"), h("span", { class: "mono", title: "Each upgraded export is written beside the others the Compiler reads" },
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
    // The step after an upgrade. Always there; once a run has finished it
    // turns the accent blue (egui: the button's fill set to the selection
    // colour), the view's one blue button, until the next run starts.
    named(Object.assign(button("Open the Compiler", () => switchToTool("team-compiler"),
      { disabled: !!run, title: "The upgraded exports are in its exports folder: it checks them like any other" }),
      { className: S.finished && !run ? "eu-next" : "" }), "next"),
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
    item("Copy report", "The run's lines, as the console prints them, are put on the clipboard."));
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
    : h("div", { class: "eu-pairs", "data-panel": "pairs" },
        h("div", { class: "eu-cols weak" }, h("span", {}, "Old export"), h("span", {}, "Upgraded export")),
        all.map(flowBlock));
  center.append(h("div", { class: "pipe" }, argsRow(), !all.length ? null : many ? legend() : flowLegend(), body, toolbar(),
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
  ["the files look", "set old_exports to 3 exports: a box per old folder left, a box per new one right, a line where each goes; point at a box to follow its lines (/egg/'s Boots\\k0920 goes to players 02, 03 and 04, the shared folder and the IDs); Upgrade lights each destination as it is written"],
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
