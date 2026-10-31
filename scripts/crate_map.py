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
import sys

meta = json.loads(subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1"],
                                 capture_output=True, text=True, check=True).stdout)
names = {p["name"] for p in meta["packages"]}
deps = {}
folder = {}
description = {}
manifest = {}
for p in meta["packages"]:
    path = p["manifest_path"].replace("\\", "/").split("/crates/")[-1]
    if path.startswith("dev/"):
        continue
    folder[p["name"]] = path.split("/")[0]
    description[p["name"]] = p.get("description") or ""
    manifest[p["name"]] = "crates/" + path
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
    # Longest path down to a lib that uses no other lib: each lib as low as it can go.
    below = [d for d in deps[name] if d in LIBS]
    return 1 + max(map(lib_level, below)) if below else 0


def lib_depth(name):
    # Longest path up to a lib no other lib uses: each lib as high as it can go, one row under
    # the lowest lib that uses it.
    above = [n for n in LIBS if name in deps[n]]
    return 1 + max(map(lib_depth, above)) if above else 0


def with_libs(lib_rows):
    head = [("The binary", ["studio"]), ("Tools", TOOLS), ("Widget libs", WIDGET_LIBS),
            ("Platform", ["studio_core"])]
    return head + [("Libs" if index == 0 else "", row) for index, row in enumerate(lib_rows)]


# The page's button switches between the two; "low" is the one it opens on.
lowest, deepest = max(map(lib_level, LIBS)), max(map(lib_depth, LIBS))
LAYOUTS = {
    "low": with_libs([sorted(n for n in LIBS if lib_level(n) == level)
                      for level in range(lowest, -1, -1)]),
    "high": with_libs([sorted(n for n in LIBS if lib_depth(n) == depth)
                       for depth in range(deepest + 1)]),
}

BOX_W, BOX_H, GAP, ROW_H, LEFT, TOP = 104, 30, 10, 84, 130, 40
WIDTH = LEFT + max(len(r) for rows in LAYOUTS.values() for _, r in rows) * (BOX_W + GAP) + 30
HEIGHT = TOP + max(map(len, LAYOUTS.values())) * ROW_H + 20

# A crate's description is drawn on one line above the chart, as wide as it: a longer one
# wraps and pushes the chart down while the pointer moves. Measured in Chromium with Segoe UI
# at 15 px, the name and description drew at 6.6 to 7.3 px a character; 9 leaves room for a
# wider fallback font. The fix goes to the crate's `description` in its Cargo.toml.
PX_PER_CHAR = 9
limit = WIDTH // PX_PER_CHAR
lengths = {name: len(name) + 1 + len(text) for name, text in description.items()}
too_long = [name for name, length in lengths.items() if length > limit]
for name in too_long:
    print(f"crate_map: {name}'s description is {lengths[name]} characters with its name, over "
          f"the {limit} that fit on one line: shorten `description` in {manifest[name]}",
          file=sys.stderr)
if too_long:
    sys.exit(1)


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


def chart(layout, rows):
    pos = {}
    for index, (_, row) in enumerate(rows):
        span = len(row) * BOX_W + (len(row) - 1) * GAP
        x0 = LEFT + ((WIDTH - LEFT - 30) - span) / 2
        for column, name in enumerate(row):
            pos[name] = (x0 + column * (BOX_W + GAP), TOP + index * ROW_H)
    # Each chart its own marker id: a url(#id) shared by two charts resolves into the first
    # one, which is hidden while the other shows.
    marker = f"arrow-{layout}"
    svg = [f'<svg data-libs="{layout}" xmlns="http://www.w3.org/2000/svg" '
           f'viewBox="0 0 {WIDTH} {HEIGHT}" width="{WIDTH}" height="{HEIGHT}" '
           'font-family="Segoe UI, sans-serif">',
           f'<defs><marker id="{marker}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" '
           'markerHeight="6" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" '
           'fill="context-stroke"/></marker></defs>']
    # One band per label, the lib rows under "Libs" sharing theirs.
    bands = []
    for index, (label, row) in enumerate(rows):
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
    svg.append('<g class="edges">')
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
                       f'fill="none" stroke-width="1.2" marker-end="url(#{marker})"/>')
    svg.append('</g><g class="nodes">')
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
    return "".join(svg)


charts = "".join(chart(layout, rows) for layout, rows in LAYOUTS.items())

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
#controls {{ position: absolute; top: 24px; right: 24px; display: flex; gap: 8px; }}
#controls button {{ font: inherit; font-size: 13px;
  color: var(--text); background: var(--code); border: 1px solid var(--out-line);
  border-radius: 4px; padding: 3px 10px; cursor: pointer; }}
/* One chart per lib layout, the button showing the other. */
:root:not([data-libs="high"]) svg[data-libs="high"],
:root[data-libs="high"] svg[data-libs="low"] {{ display: none; }}
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
/* As wide as the chart, not the intro's measure: a description that wrapped pushed the chart
   down a line while the pointer moved over it. */
#about {{ min-height: 1.4em; font-size: 15px; color: var(--text); width: {WIDTH}px;
  max-width: none; }}
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
if (localStorage.getItem("crate_map_libs") === "high")
  document.documentElement.dataset.libs = "high";
</script></head><body>
<div id="controls"><button id="libs" type="button"></button>
<button id="theme" type="button"></button></div>
<h1>Unlimited Tool Works: how the crates depend on each other</h1>
<p>Every arrow points from a crate to one it uses (normal dependencies from the manifests,
dev-dependencies left out). Rows run from the binary at the top to the libs at the bottom;
within the libs, a row uses only rows below it. Each lib sits as low as it can go, the libs
that use nothing on the bottom row, or, with the button, as high as it can go, one row under
the lowest lib that uses it. Dashed crates are not linked into <code>UTW</code>.</p>
<p class="key">Click a crate to keep it highlighted and put it in the address
(<code>#save_editor</code>), again to let go:<span class="uses"></span>what it uses
<span class="usedby"></span>what uses it</p>
<p id="about"><span class="hint">Hover over a crate to see what it does.</span></p>
{charts}
<script>
const libsButton = document.getElementById("libs");
function labelLibs() {{
  const high = document.documentElement.dataset.libs === "high";
  libsButton.textContent = high ? "Libs as low as they go" : "Libs as high as they go";
}}
libsButton.addEventListener("click", () => {{
  const high = document.documentElement.dataset.libs !== "high";
  if (high) document.documentElement.dataset.libs = "high";
  else delete document.documentElement.dataset.libs;
  localStorage.setItem("crate_map_libs", high ? "high" : "low");
  labelLibs();
}});
labelLibs();
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
// Both charts follow the hover and the pin, so the switch keeps what is highlighted.
const charts = [...document.querySelectorAll("svg")];
const edges = [...document.querySelectorAll("svg .edge")];
const nodes = [...document.querySelectorAll("svg .node")];
const about = document.getElementById("about");
const hint = about.firstElementChild;
// The crate a click or the address chose; a hover shows another only while it lasts.
let pinned = null;
function show(name) {{
  for (const edge of edges) edge.classList.remove("uses", "usedby");
  const node = nodes.find(n => n.dataset.name === name);
  for (const chart of charts) chart.classList.toggle("focus", !!node);
  if (!node) {{
    about.replaceChildren(hint);
    return;
  }}
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
// A click anywhere else on the page lets go, but not one on a control with a job of its own
// (the theme button, a link).
document.addEventListener("click", event => {{
  if (!event.target.closest("button, a")) pin(null);
}});
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
print("wrote", OUT, f"{len(deps)} crates,", sum(map(len, deps.values())), "arrows")
