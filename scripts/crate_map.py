"""Writes `docs/web/crate_map.html`: the workspace's crates as an SVG of layers, every
dependency between workspace crates as an arrow (normal dependencies only, not dev or build),
read from `cargo metadata` so the picture is the manifests' truth. Hovering a crate highlights
what it uses and what uses it. Run from the repository root (`just crate-map`) when a crate
is added or its workspace dependencies change. No gate checks that the page is current: it is
a map to keep a picture of the project by, not a contract."""
import html
import json
import os
import subprocess

meta = json.loads(subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1"],
                                 capture_output=True, text=True, check=True).stdout)
names = {p["name"] for p in meta["packages"]}
deps = {}
folder = {}
description = {}
for p in meta["packages"]:
    path = p["manifest_path"].replace("\\", "/").split("/crates/")[-1]
    if path.startswith("dev/"):
        continue
    folder[p["name"]] = path.split("/")[0]
    description[p["name"]] = p.get("description") or ""
    deps[p["name"]] = sorted({d["name"] for d in p["dependencies"]
                              if d["name"] in names and d["kind"] is None})

# What the binary links: everything reachable from `studio`.
reach, todo = set(), ["studio"]
while todo:
    name = todo.pop()
    if name not in reach:
        reach.add(name)
        todo.extend(deps[name])

WIDGET_LIBS = ["color_tools", "team_widgets"]
TOOLS = sorted(n for n, f in folder.items() if f == "tools")
LIBS = sorted(n for n, f in folder.items() if f == "libs" and n not in WIDGET_LIBS)


def lib_level(name):
    # Longest path down to a lib that uses no other lib: the libs' own rows.
    below = [d for d in deps[name] if d in LIBS]
    return 1 + max(map(lib_level, below)) if below else 0


ROWS = [
    ("The binary", ["studio"]),
    ("Tools", TOOLS),
    ("Widget libs", WIDGET_LIBS),
    ("Platform", ["studio_core"]),
]
levels = max(map(lib_level, LIBS))
for level in range(levels, -1, -1):
    label = "Libs" if level == levels else ""
    ROWS.append((label, sorted(n for n in LIBS if lib_level(n) == level)))

BOX_W, BOX_H, GAP, ROW_H, LEFT, TOP = 104, 30, 10, 84, 130, 40
WIDTH = LEFT + max(len(r) for _, r in ROWS) * (BOX_W + GAP) + 30
HEIGHT = TOP + len(ROWS) * ROW_H + 20
pos = {}
for index, (_, row) in enumerate(ROWS):
    span = len(row) * BOX_W + (len(row) - 1) * GAP
    x0 = LEFT + ((WIDTH - LEFT - 30) - span) / 2
    for column, name in enumerate(row):
        pos[name] = (x0 + column * (BOX_W + GAP), TOP + index * ROW_H)


def band_of(name):
    if name == "studio":
        return "studio"
    if name in TOOLS:
        return "tools"
    if name in WIDGET_LIBS:
        return "widget"
    if name == "studio_core":
        return "core"
    return "libs"


svg = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {WIDTH} {HEIGHT}" '
       f'width="{WIDTH}" height="{HEIGHT}" font-family="Segoe UI, sans-serif">',
       '<defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" '
       'markerHeight="6" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" '
       'fill="context-stroke"/></marker></defs>']
# One band per label, the lib rows under "Libs" sharing theirs.
bands = []
for index, (label, row) in enumerate(ROWS):
    if label:
        bands.append([label, band_of(row[0]), index, index])
    else:
        bands[-1][3] = index
for label, band, first, last in bands:
    y = TOP + first * ROW_H - 18
    height = (last - first + 1) * ROW_H - 10
    svg.append(f'<rect x="10" y="{y}" width="{WIDTH - 20}" height="{height}" rx="8" '
               f'class="band {band}"/>')
    svg.append(f'<text class="band-label" x="22" y="{y + 30}" font-size="15" '
               f'font-weight="600">{html.escape(label)}</text>')
svg.append('<g id="edges">')
for source, targets in deps.items():
    sx, sy = pos[source]
    for target in targets:
        tx, ty = pos[target]
        x1, y1 = sx + BOX_W / 2, sy + BOX_H
        x2, y2 = tx + BOX_W / 2, ty
        if ty == sy:
            # Same row (a lib using a lib beside it): arc under the row.
            y1 = y2 = sy + BOX_H
            d = f"M{x1},{y1} C{x1},{y1 + 40} {x2},{y2 + 40} {x2},{y2}"
        else:
            mid = (y1 + y2) / 2
            d = f"M{x1},{y1} C{x1},{mid} {x2},{mid} {x2},{y2}"
        svg.append(f'<path class="edge" data-from="{source}" data-to="{target}" d="{d}" '
                   f'fill="none" stroke-width="1.2" marker-end="url(#arrow)"/>')
svg.append('</g><g id="nodes">')
for name, (x, y) in pos.items():
    outside = name not in reach
    dash = ' stroke-dasharray="4 3"' if outside else ""
    about = html.escape(description[name], quote=True)
    svg.append(f'<g class="node{" outside" if outside else ""}" data-name="{name}" '
               f'data-about="{about}">'
               f'<title>{name}: {about}</title>'
               f'<rect x="{x}" y="{y}" width="{BOX_W}" height="{BOX_H}" rx="5"{dash}/>'
               f'<text x="{x + BOX_W / 2}" y="{y + 20}" font-size="12" '
               f'text-anchor="middle">{name}</text></g>')
svg.append("</g></svg>")

page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Unlimited Tool Works: crate layers</title>
<style>
/* Dark by default; the button switches to light, remembered in this browser. */
:root {{ --page: #1b1d21; --text: #e2e2e2; --soft: #a8abb0; --hint: #7d8187;
  --studio: #3b3352; --tools: #24394f; --widget: #26402c; --core: #4a3b22; --libs: #2e3034;
  --band-label: #c4c7cc; --edge: #9a9da3; --edge-rest: 0.3;
  --box: #2a2d33; --box-line: #9aa0a8; --box-text: #ececec;
  --out-box: #222428; --out-line: #5f636a; --out-text: #8a8e94;
  --uses: #4c9aff; --usedby: #f0a040; --code: #2a2d33; }}
:root[data-theme="light"] {{ --page: #fff; --text: #222; --soft: #444; --hint: #888;
  --studio: #e8e0f5; --tools: #dcecf9; --widget: #e2f3e4; --core: #fbecd6; --libs: #eeeeee;
  --band-label: #444; --edge: #7a7a7a; --edge-rest: 0.22;
  --box: #fff; --box-line: #555; --box-text: #222;
  --out-box: #f6f6f6; --out-line: #aaa; --out-text: #888;
  --uses: #1f6fd1; --usedby: #d9822b; --code: #f0f0f0; }}
body {{ font-family: "Segoe UI", sans-serif; margin: 24px; color: var(--text);
  background: var(--page); }}
h1 {{ font-size: 20px; margin: 0 0 6px; }}
p {{ margin: 4px 0; font-size: 14px; color: var(--soft); max-width: 1100px; }}
code {{ background: var(--code); padding: 0 3px; border-radius: 3px; }}
#theme {{ position: absolute; top: 24px; right: 24px; font: inherit; font-size: 13px;
  color: var(--text); background: var(--code); border: 1px solid var(--out-line);
  border-radius: 4px; padding: 3px 10px; cursor: pointer; }}
.band {{ opacity: 0.55; }}
/* The dark bands at the light ones' distance from their page (CIELAB delta E 3 to 9 in
   both themes): brighter, they outweighed the arrows. */
:root:not([data-theme="light"]) .band {{ opacity: 0.35; }}
.band.studio {{ fill: var(--studio); }} .band.tools {{ fill: var(--tools); }}
.band.widget {{ fill: var(--widget); }} .band.core {{ fill: var(--core); }}
.band.libs {{ fill: var(--libs); }}
.band-label {{ fill: var(--band-label); }}
.edge {{ stroke: var(--edge); opacity: var(--edge-rest); transition: opacity .1s; }}
.node rect {{ fill: var(--box); stroke: var(--box-line); }}
.node text {{ fill: var(--box-text); }}
.node.outside rect {{ fill: var(--out-box); stroke: var(--out-line); }}
.node.outside text {{ fill: var(--out-text); }}
svg.focus .edge, svg.focus .node {{ opacity: 0.12; }}
svg.focus .edge.uses {{ opacity: 1; stroke: var(--uses); stroke-width: 2; }}
svg.focus .edge.usedby {{ opacity: 1; stroke: var(--usedby); stroke-width: 2; }}
svg.focus .node.on {{ opacity: 1; }}
.node {{ cursor: pointer; }}
.node.pinned rect {{ stroke: var(--uses); stroke-width: 2.5; }}
#about {{ min-height: 1.4em; font-size: 15px; color: var(--text); }}
#about b {{ margin-right: 6px; }}
#about .hint {{ color: var(--hint); }}
.key span {{ display: inline-block; width: 22px; height: 3px; vertical-align: middle;
  margin: 0 4px 0 14px; }}
.key .uses {{ background: var(--uses); }} .key .usedby {{ background: var(--usedby); }}
</style>
<script>
// Before the body draws, so a light choice never flashes dark first.
if (localStorage.getItem("crate_map_theme") === "light")
  document.documentElement.dataset.theme = "light";
</script></head><body>
<button id="theme" type="button"></button>
<h1>Unlimited Tool Works: how the crates depend on each other</h1>
<p>Every arrow points from a crate to one it uses (normal dependencies from the manifests,
dev-dependencies left out). Rows run from the binary at the top to the libs that use nothing
at the bottom; within the libs, a row uses only rows below it.
Dashed crates are not linked into <code>UTW</code>.</p>
<p class="key">Click a crate to keep it highlighted and put it in the address
(<code>#save_editor</code>), again to let go:<span class="uses"></span>what it uses
<span class="usedby"></span>what uses it</p>
<p id="about"><span class="hint">Hover a crate for what it is.</span></p>
{"".join(svg)}
<script>
const themeButton = document.getElementById("theme");
function labelTheme() {{
  const light = document.documentElement.dataset.theme === "light";
  themeButton.textContent = light ? "Dark theme" : "Light theme";
}}
themeButton.addEventListener("click", () => {{
  const light = document.documentElement.dataset.theme !== "light";
  if (light) document.documentElement.dataset.theme = "light";
  else delete document.documentElement.dataset.theme;
  localStorage.setItem("crate_map_theme", light ? "light" : "dark");
  labelTheme();
}});
labelTheme();
const svg = document.querySelector("svg");
const edges = [...svg.querySelectorAll(".edge")];
const nodes = [...svg.querySelectorAll(".node")];
const about = document.getElementById("about");
const hint = about.firstElementChild;
// The crate a click or the address chose; a hover shows another only while it lasts.
let pinned = null;
function show(name) {{
  for (const edge of edges) edge.classList.remove("uses", "usedby");
  const node = nodes.find(n => n.dataset.name === name);
  if (!node) {{
    svg.classList.remove("focus");
    about.replaceChildren(hint);
    return;
  }}
  svg.classList.add("focus");
  const on = new Set([name]);
  for (const edge of edges) {{
    if (edge.dataset.from === name) {{ edge.classList.add("uses"); on.add(edge.dataset.to); }}
    if (edge.dataset.to === name) {{ edge.classList.add("usedby"); on.add(edge.dataset.from); }}
  }}
  for (const other of nodes) other.classList.toggle("on", on.has(other.dataset.name));
  about.innerHTML = "";
  const title = document.createElement("b");
  title.textContent = name;
  about.append(title, node.dataset.about);
}}
function pin(name) {{
  pinned = nodes.some(n => n.dataset.name === name) ? name : null;
  for (const node of nodes) node.classList.toggle("pinned", node.dataset.name === pinned);
  show(pinned);
  // replaceState, not location.hash: a click is no page to go back to. The address is
  // absolute: a viewer that adds a <base> (htmlpreview.github.io points it at the raw file)
  // resolves a relative one against another origin, which replaceState refuses.
  const address = location.href.split("#")[0] + (pinned ? "#" + pinned : "");
  history.replaceState(null, "", address);
}}
for (const node of nodes) {{
  node.addEventListener("mouseenter", () => show(node.dataset.name));
  node.addEventListener("mouseleave", () => show(pinned));
  node.addEventListener("click", event => {{
    event.stopPropagation();
    pin(node.dataset.name === pinned ? null : node.dataset.name);
  }});
}}
svg.addEventListener("click", () => pin(null));
const fromAddress = () => decodeURIComponent(location.hash.slice(1));
// A crate typed after # in an open tab changes only the hash: no reload, so listen for it.
window.addEventListener("hashchange", () => pin(fromAddress()));
pin(fromAddress());
</script></body></html>
"""
OUT = "docs/web/crate_map.html"
with open(OUT + ".new", "w", encoding="utf-8", newline="\n") as f:
    f.write(page)
os.replace(OUT + ".new", OUT)
print("wrote", OUT, f"{len(pos)} crates,", sum(map(len, deps.values())), "arrows")
