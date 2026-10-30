"""Writes `docs/crate_map.html`: the workspace's crates as an SVG of layers, every
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
for p in meta["packages"]:
    path = p["manifest_path"].replace("\\", "/").split("/crates/")[-1]
    if path.startswith("dev/"):
        continue
    folder[p["name"]] = path.split("/")[0]
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

BANDS = {"studio": "#e8e0f5", "tools": "#dcecf9", "widget": "#e2f3e4", "core": "#fbecd6",
         "libs": "#eeeeee"}


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
               f'fill="{BANDS[band]}" opacity="0.55"/>')
    svg.append(f'<text x="22" y="{y + 30}" font-size="15" font-weight="600" '
               f'fill="#444">{html.escape(label)}</text>')
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
                   f'fill="none" stroke="#7a7a7a" stroke-width="1.2" marker-end="url(#arrow)"/>')
svg.append('</g><g id="nodes">')
for name, (x, y) in pos.items():
    outside = name not in reach
    fill = "#ffffff" if not outside else "#f6f6f6"
    stroke = "#555" if not outside else "#aaa"
    dash = ' stroke-dasharray="4 3"' if outside else ""
    svg.append(f'<g class="node" data-name="{name}"><title>{name}: uses '
               f'{", ".join(deps[name]) or "no workspace crate"}</title>'
               f'<rect x="{x}" y="{y}" width="{BOX_W}" height="{BOX_H}" rx="5" fill="{fill}" '
               f'stroke="{stroke}"{dash}/>'
               f'<text x="{x + BOX_W / 2}" y="{y + 20}" font-size="12" text-anchor="middle" '
               f'fill="{"#222" if not outside else "#888"}">{name}</text></g>')
svg.append("</g></svg>")

page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Unlimited Tool Works: crate layers</title>
<style>
body {{ font-family: "Segoe UI", sans-serif; margin: 24px; color: #222; background: #fff; }}
h1 {{ font-size: 20px; margin: 0 0 6px; }}
p {{ margin: 4px 0; font-size: 14px; color: #444; max-width: 1100px; }}
.edge {{ opacity: 0.22; transition: opacity .1s; }}
svg.focus .edge, svg.focus .node {{ opacity: 0.12; }}
svg.focus .edge.uses {{ opacity: 1; stroke: #1f6fd1; stroke-width: 2; }}
svg.focus .edge.usedby {{ opacity: 1; stroke: #d9822b; stroke-width: 2; }}
svg.focus .node.on {{ opacity: 1; }}
.node {{ cursor: default; }}
.key span {{ display: inline-block; width: 22px; height: 3px; vertical-align: middle;
  margin: 0 4px 0 14px; }}
</style></head><body>
<h1>Unlimited Tool Works: how the crates depend on each other</h1>
<p>Every arrow points from a crate to one it uses (normal dependencies from the manifests,
dev-dependencies left out). Rows run from the binary at the top to the libs that use nothing
at the bottom; within the libs, a row uses only rows below it.
Dashed crates are not linked into <code>UTW</code>.</p>
<p class="key">Hover a crate (or open the page with <code>#save_editor</code> after its
name):<span style="background:#1f6fd1"></span>what it uses
<span style="background:#d9822b"></span>what uses it</p>
{"".join(svg)}
<script>
const svg = document.querySelector("svg");
const edges = [...svg.querySelectorAll(".edge")];
function focusOn(name) {{
    svg.classList.add("focus");
    const on = new Set([name]);
    for (const edge of edges) {{
      if (edge.dataset.from === name) {{ edge.classList.add("uses"); on.add(edge.dataset.to); }}
      if (edge.dataset.to === name) {{ edge.classList.add("usedby"); on.add(edge.dataset.from); }}
    }}
    for (const other of svg.querySelectorAll(".node"))
      other.classList.toggle("on", on.has(other.dataset.name));
}}
function clearFocus() {{
    svg.classList.remove("focus");
    for (const edge of edges) edge.classList.remove("uses", "usedby");
}}
for (const node of svg.querySelectorAll(".node")) {{
  node.addEventListener("mouseenter", () => {{ clearFocus(); focusOn(node.dataset.name); }});
  node.addEventListener("mouseleave", clearFocus);
}}
// A link ending in #<crate> opens with that crate highlighted.
if (location.hash) focusOn(decodeURIComponent(location.hash.slice(1)));
</script></body></html>
"""
OUT = "docs/crate_map.html"
with open(OUT + ".new", "w", encoding="utf-8", newline="\n") as f:
    f.write(page)
os.replace(OUT + ".new", OUT)
print("wrote", OUT, f"{len(pos)} crates,", sum(map(len, deps.values())), "arrows")
