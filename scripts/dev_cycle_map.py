"""Draw docs/web/dev_cycle.html: how work moves through the project, from a plan to a release.

The stages and arrows are the tables below; edit them when AGENTS.md or RUNBOOK.md changes
the process, then run `python scripts/dev_cycle_map.py`. Each row of the chart is one
participant (the maintainer, the documents, the lead, the sidekick, the reviewers, the checks,
the mutation runs), each column one stage, left to right.
"""

import html
import os

STAGES = ["Plan", "Accept", "Brief", "Build", "Review", "Measure", "Close", "Release"]

# (key, label, colour variable)
BANDS = [
    ("maintainer", "Maintainer", "--b-maint"),
    ("docs", "Documents", "--b-docs"),
    ("lead", "Lead", "--b-lead"),
    ("sidekick", "Sidekicks", "--b-side"),
    ("reviewers", "Reviewers", "--b-rev"),
    ("checks", "Tests and gates", "--b-check"),
    ("mutation", "Mutation testing", "--b-mut"),
]

# (id, label, band, stage column, row within the band from 0, what it is). A band is as
# tall as its rows. Review holds what reads the work and Measure what runs it: the three
# reviewers stand in one column, and so do the gates with the other runs, and the three
# mutation runs.
NODES = [
    ("directives", "Directives", "maintainer", 0, 0,
     "Rules and choices the maintainer sets; each lands in a plan, AGENTS.md or RUNBOOK.md "
     "and a DECISIONS.md entry, never in chat alone."),
    ("look", "Draft look", "maintainer", 1, 0,
     "The maintainer looks at a UI draft rendered from docs/prototypes/ before anything a "
     "member sees is briefed."),
    ("answers", "Answers and rulings", "maintainer", 1, 1,
     "Answers to QUESTIONS.md (in-game behavior, cup practice, preferences) and vetoes or "
     "approvals of PROPOSALS.md; an answer is open to discussion when the lead sees a reason "
     "against it."),
    ("ingame_ok", "In-game look", "maintainer", 5, 0,
     "The maintainer confirms what only the game can show; Edit mode suffices for most looks. "
     "The proof becomes a 'manual: checked' line in the worklog."),
    ("release", "Release and push", "maintainer", 7, 0,
     "Pushes and release cuts need the maintainer's OK, and a push is asked for only at a "
     "gated commit; the lead reads CI after it. During 0.x.x a release does not wait for "
     "whole-crate mutation runs: their findings ship in the next release or a patch."),
    ("plans", "Plans", "docs", 0, 0,
     "docs/plans/: the design of what is not built yet, its rationale ('we do X, not Y, because "
     "Y causes Z') and the open phase's Acceptance sections."),
    ("decisions", "DECISIONS.md", "docs", 0, 1,
     "Append-only: every choice made where the plan was silent, written right after the plan "
     "edit it explains."),
    ("acceptance", "Acceptance section", "docs", 1, 0,
     "Stable-ID GIVEN/WHEN/THEN scenarios for what a tool phase delivers, written just in time "
     "for that phase, never for the whole project up front."),
    ("questions", "QUESTIONS / PROPOSALS", "docs", 1, 1,
     "QUESTIONS.md: what only the maintainer can answer. PROPOSALS.md: usability ideas nobody "
     "asked for, unbuilt until ruled."),
    ("worklog", "WORKLOG.md", "docs", 2, 0,
     "Where we are: the release's blocking path, the open steps with their verify criteria, "
     "the looks still owed in the game or the built window, and open issues. A step "
     "collapses to one line when it closes."),
    ("spec", "Spec", "docs", 6, 0,
     "docs/spec/: what the suite does now. A closed phase's sections move here from the plans, "
     "acceptance IDs included, as the behavior contract."),
    ("log", "CHANGELOG, README", "docs", 7, 0,
     "CHANGELOG.md's [Unreleased] entry for what a member would notice, and the README "
     "brought to the closed phase. The dated log is git log: an event is written in one "
     "place."),
    ("plan_write", "Plan and decide", "lead", 0, 0,
     "The lead writes plans and applies its clear recommendations (plan edit, then a decision "
     "entry); only a truly open choice goes to QUESTIONS.md."),
    ("draft", "UI draft", "lead", 1, 0,
     "Anything a member sees is drafted first under docs/prototypes/, rendered and critiqued; "
     "a brief names the draft function it builds."),
    ("brief", "Brief", "lead", 2, 0,
     "One reviewable slice (about 500 lines at most): reading pointers, exact deliverables, a "
     "Verify list whose every check can fail, and a closing 'Errors hit' section. The "
     "release's blocking path is briefed first; two or three sidekicks run at once."),
    ("critical", "Correctness-critical code", "lead", 3, 0,
     "What the lead writes itself because its wrong version passes: fixtures, golden files, "
     "lints, CI, measurements."),
    ("diff_review", "Diff review and sweeps", "lead", 4, 0,
     "The lead reads the whole diff, reruns the tests, clippy and fmt, and runs the honesty "
     "sweep (hidden failures) and the design sweep (code that passes and is worse)."),
    ("rulings", "Rulings", "lead", 4, 1,
     "Every reviewer concern is accepted or rejected with a reason. Being true is not enough: "
     "an accepted concern changes a real export, contradicts a rule, or fixes a wrong fact."),
    ("commit", "Commit", "lead", 6, 0,
     "The changed crates' tests, clippy and fmt run, the worklog step collapses to one "
     "line, then scripts/commit.py checks the message and commits: the message is the log."),
    ("converge", "Converge audit", "lead", 6, 1,
     "Closing a phase: every plan section and acceptance ID checked against the code, then a "
     "design-health pass (sweeps, whole-crate mutation runs, citation audit, pub items). "
     "Every gap becomes a new worklog step, and the phase stays open until those are done."),
    ("implement", "Implement, red first", "sidekick", 3, 0,
     "An Opus sidekick in its own worktree (real slices) or SWE-2 (small ones) implements "
     "the brief in the files it owns: each new test fails first on the old code, and it "
     "never commits."),
    ("report", "Report", "sidekick", 4, 0,
     "Per item the change, its test and red run, the Verify output, Contradictions with the "
     "brief, and 'Errors hit', every suppression declared."),
    ("rework", "Rework", "sidekick", 4, 1,
     "A rework brief resumes the same session: numbered findings, each with its location, fix "
     "and test. Two failed attempts make the brief suspect."),
    ("ingame_run", "In-game run", "sidekick", 5, 0,
     "A SWE-2 sidekick drives PES through the pad and OBS harness (scripts/ingame/), working "
     "through the worklog's pending looks, and writes its findings and captures to files "
     "for the lead and the maintainer."),
    ("swe", "SWE-2 rounds", "reviewers", 4, 0,
     "The free first layer, on every surface: rounds until one accepts fewer than five "
     "concerns (one round for scripts and documents). It runs again after a dearer "
     "layer's fixes."),
    ("argon", "Carbon rounds", "reviewers", 4, 1,
     "Gemini 4 Carbon High, the middle layer: better than SWE-2, much cheaper than Astra. "
     "Shipped code gets one paid round; only what writes game files or saves gets both "
     "paid loops."),
    ("astra", "Astra rounds", "reviewers", 4, 2,
     "GPT-6 Astra, the last layer, on what the cheaper ones could not see. Each layer loops "
     "until a round accepts fewer than five; a loop not started within a week folds into "
     "the converge audit."),
    ("unit", "Unit & integration tests", "checks", 3, 0,
     "Each crate's tests, written red first; a test fails, it never hangs."),
    ("accept_tests", "Acceptance tests", "checks", 3, 1,
     "Tests citing a scenario's ID; the acceptance gate lists every ID no test or "
     "'manual: checked' line proves."),
    ("gates", "Gates", "checks", 5, 0,
     "just gates: fmt, clippy with warnings as errors, every test, line endings, comment "
     "references and doc pointers. just checkpoint runs them over everything since the last "
     "gated commit, with mutants-diff, and moves that commit: at a loop's end, a step's "
     "close, a merge, and before a push is asked for."),
    ("citation", "Citation audit", "checks", 4, 0,
     "Flags tests whose cited scenario they may not prove; every flag is ruled before a crate's "
     "review."),
    ("parity", "Library parity vs Red", "checks", 5, 1,
     "A phase that changes the Team compiler or the Export upgrader recompiles the library and "
     "diagnoses each new difference from Red's output."),
    ("census", "Census", "checks", 5, 2,
     "A format crate's close reads every file of its format on the machine, round trips and "
     "checks each, and tallies the outcomes. A kept index of the real exports answers a "
     "reviewer's 'does any file do this' in seconds, beside a control count."),
    ("ci", "CI on the push", "checks", 7, 0,
     "Every push runs just gates on Linux and on Windows, and a version tag builds the "
     "release's bundles with their checksums. just checkpoint-status prints its verdict on "
     "the pushed commit."),
    ("mdiff", "mutants-diff", "mutation", 5, 0,
     "Part of just checkpoint, over the commits since the last gated one: each surviving "
     "mutant is an assertion nobody wrote. It runs on the lead's PC, so a whole-crate run "
     "can hold the VPS meanwhile."),
    ("extra", "Extra mutant kinds", "mutation", 5, 1,
     "bool, literal, normalize and order mutants cargo-mutants does not make, run before a "
     "whole-crate run into the same survivor list."),
    ("whole", "Whole-crate run (VPS)", "mutation", 5, 2,
     "Every mutant of a crate, on the VPS alone: twice per crate, in the lead's audit and at "
     "the phase's close. The next phase starts while it runs."),
    ("triage", "Survivor triage", "mutation", 6, 1,
     "Each survivor is a missing test (into one rework brief) or an equivalent mutant, ruled "
     "once and recorded."),
]

# (from, to): the first one feeds the second.
EDGES = [
    ("directives", "plans"), ("directives", "decisions"),
    ("plan_write", "plans"), ("plan_write", "decisions"), ("plan_write", "questions"),
    ("questions", "answers"), ("answers", "plans"), ("answers", "decisions"),
    ("plans", "acceptance"), ("acceptance", "swe"), ("plans", "draft"),
    ("draft", "look"), ("look", "brief"),
    ("plans", "brief"), ("acceptance", "brief"), ("worklog", "brief"),
    ("brief", "implement"), ("brief", "ingame_run"),
    ("implement", "unit"), ("implement", "accept_tests"), ("implement", "report"),
    ("critical", "unit"), ("critical", "gates"),
    ("unit", "gates"), ("accept_tests", "gates"),
    ("report", "diff_review"), ("diff_review", "rework"), ("rework", "report"),
    ("diff_review", "commit"),
    ("commit", "worklog"), ("commit", "log"),
    ("commit", "swe"), ("swe", "argon"), ("argon", "astra"), ("astra", "swe"),
    ("swe", "rulings"), ("argon", "rulings"), ("astra", "rulings"),
    ("rulings", "rework"), ("rulings", "gates"), ("rulings", "mdiff"), ("mdiff", "triage"),
    ("ingame_run", "ingame_ok"), ("ingame_ok", "worklog"),
    ("plans", "converge"), ("acceptance", "converge"), ("commit", "converge"),
    ("converge", "worklog"), ("converge", "astra"), ("converge", "whole"), ("converge", "citation"),
    ("converge", "parity"), ("converge", "census"), ("converge", "spec"),
    ("extra", "whole"), ("whole", "triage"), ("triage", "rework"),
    ("spec", "release"), ("converge", "release"), ("converge", "log"),
    # What a slice is built from and checked by.
    ("draft", "brief"), ("spec", "brief"), ("decisions", "brief"), ("brief", "critical"),
    ("acceptance", "accept_tests"), ("accept_tests", "citation"), ("citation", "swe"),
    # The reviewers between themselves, and what settles a concern.
    ("argon", "swe"), ("swe", "astra"), ("census", "rulings"), ("census", "spec"),
    ("rulings", "plans"), ("rulings", "decisions"),
    # The checkpoint: the commits since the last gated one, and the push it allows.
    ("commit", "gates"), ("commit", "mdiff"), ("unit", "mdiff"), ("unit", "whole"),
    ("gates", "release"), ("log", "release"),
    # The looks owed, and what the measurements send back.
    ("worklog", "ingame_run"), ("ingame_run", "worklog"), ("parity", "worklog"),
    ("triage", "worklog"),
    ("directives", "plan_write"), ("worklog", "plan_write"), ("converge", "extra"),
    # A push starts CI; its bundles are what a release publishes.
    ("release", "ci"), ("ci", "release"),
]

LEFT, TOP, COL_W, BOX_W, BOX_H, ROW_TOP, ROW_STEP = 150, 40, 205, 176, 30, 14, 40
BOW = 0.18
WIDTH = LEFT + len(STAGES) * COL_W + 10
BAND_ROWS = {key: 1 + max(n[4] for n in NODES if n[2] == key) for key, _, _ in BANDS}
BAND_H = {key: ROW_TOP + rows * ROW_STEP - 2 for key, rows in BAND_ROWS.items()}
BAND_Y = {}
HEIGHT = TOP
for key, _, _ in BANDS:
    BAND_Y[key] = HEIGHT
    HEIGHT += BAND_H[key]
HEIGHT += 10

ids = [n[0] for n in NODES]
assert len(ids) == len(set(ids)), "a node id appears twice"
spots = [(n[2], n[3], n[4]) for n in NODES]
assert len(spots) == len(set(spots)), "two nodes share a place"
for source, target in EDGES:
    assert source in ids and target in ids, f"unknown node in edge {source} -> {target}"
assert len(EDGES) == len(set(EDGES)), "an edge appears twice"
for n in NODES:
    assert n[2] in BAND_Y and 0 <= n[3] < len(STAGES) and n[4] >= 0, n[0]
    assert len(n[1]) * 6.5 <= BOX_W - 8, f"label too long for its box: {n[1]}"


def box(node):
    _, _, band, col, sub, _ = node
    x = LEFT + col * COL_W + (COL_W - BOX_W) / 2
    y = BAND_Y[band] + ROW_TOP + sub * ROW_STEP
    return x, y


CENTER = {n[0]: (box(n)[0] + BOX_W / 2, box(n)[1] + BOX_H / 2) for n in NODES}


def border(cx, cy, dx, dy):
    """Where the ray from a box's centre along (dx, dy) leaves the box."""
    scale = min(BOX_W / 2 / abs(dx) if dx else float("inf"),
                BOX_H / 2 / abs(dy) if dy else float("inf"))
    return cx + dx * scale, cy + dy * scale


svg = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {WIDTH} {HEIGHT}" '
       f'width="{WIDTH}" height="{HEIGHT}" font-family="Segoe UI, sans-serif">',
       '<defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" '
       'markerHeight="6" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" '
       'fill="context-stroke"/></marker></defs>']
for i, stage in enumerate(STAGES):
    x = LEFT + i * COL_W + COL_W / 2
    svg.append(f'<text class="stage" x="{x:.0f}" y="{TOP - 14}" font-size="13" '
               f'text-anchor="middle">{html.escape(stage)}</text>')
for key, label, colour in BANDS:
    y, band_h = BAND_Y[key], BAND_H[key]
    svg.append(f'<rect class="band" x="0" y="{y}" width="{WIDTH}" height="{band_h - 4}" '
               f'rx="6" style="fill: var({colour})"/>')
    svg.append(f'<text class="band-label" x="12" y="{y + band_h / 2 + 2:.0f}" '
               f'font-size="13">{html.escape(label)}</text>')
for source, target in EDGES:
    (ax, ay), (bx, by) = CENTER[source], CENTER[target]
    # Bowed to the left of its direction, not straight: a straight arrow along a row ran
    # through the boxes between its ends, and two arrows between one pair overlapped.
    cx = (ax + bx) / 2 + (by - ay) * BOW
    cy = (ay + by) / 2 - (bx - ax) * BOW
    sx, sy = border(ax, ay, cx - ax, cy - ay)
    ex, ey = border(bx, by, cx - bx, cy - by)
    svg.append(f'<path class="edge" data-from="{source}" data-to="{target}" '
               f'd="M{sx:.1f},{sy:.1f} Q{cx:.1f},{cy:.1f} {ex:.1f},{ey:.1f}" fill="none" '
               f'stroke-width="1.2" marker-end="url(#arrow)"/>')
svg.append('<g>')
for node in NODES:
    key, label, _, _, _, about = node
    x, y = box(node)
    svg.append(f'<g class="node" data-name="{key}" data-label="{html.escape(label)}" '
               f'data-about="{html.escape(about)}"><title>{html.escape(label)}: '
               f'{html.escape(about)}</title><rect x="{x:.1f}" y="{y}" width="{BOX_W}" '
               f'height="{BOX_H}" rx="5"/><text x="{x + BOX_W / 2:.1f}" y="{y + 20}" '
               f'font-size="12" text-anchor="middle">{html.escape(label)}</text></g>')
svg.append('</g></svg>')

PAGE = """<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Unlimited Tool Works: the development cycle</title>
<style>
/* Dark by default; the button switches to light, remembered in this browser. */
:root { --page: #1b1d21; --text: #e2e2e2; --soft: #a8abb0; --hint: #7d8187;
  --b-maint: #4a3b22; --b-docs: #2e3034; --b-lead: #3b3352; --b-side: #24394f;
  --b-rev: #26402c; --b-check: #42292c; --b-mut: #233f42;
  --band-label: #c4c7cc; --edge: #9a9da3; --edge-rest: 0.3; --stage: #c4c7cc;
  --box: #2a2d33; --box-line: #9aa0a8; --box-text: #ececec;
  --uses: #4c9aff; --usedby: #f0a040; --code: #2a2d33; --line: #5f636a; }
:root[data-theme="light"] { --page: #fff; --text: #222; --soft: #444; --hint: #888;
  --b-maint: #fbecd6; --b-docs: #eeeeee; --b-lead: #e8e0f5; --b-side: #dcecf9;
  --b-rev: #e2f3e4; --b-check: #f8e0e0; --b-mut: #dcf2f1;
  --band-label: #444; --edge: #7a7a7a; --edge-rest: 0.22; --stage: #444;
  --box: #fff; --box-line: #555; --box-text: #222;
  --uses: #1f6fd1; --usedby: #d9822b; --code: #f0f0f0; --line: #aaa; }
body { font-family: "Segoe UI", sans-serif; margin: 24px; color: var(--text);
  background: var(--page); }
h1 { font-size: 20px; margin: 0 0 6px; }
p { margin: 4px 0; font-size: 14px; color: var(--soft); max-width: 1100px; }
code { background: var(--code); padding: 0 3px; border-radius: 3px; }
#theme { position: absolute; top: 24px; right: 24px; font: inherit; font-size: 13px;
  color: var(--text); background: var(--code); border: 1px solid var(--line);
  border-radius: 4px; padding: 3px 10px; cursor: pointer; }
.band { opacity: 0.55; }
:root:not([data-theme="light"]) .band { opacity: 0.35; }
.band-label { fill: var(--band-label); font-weight: 600; }
.stage { fill: var(--stage); font-weight: 600; }
.edge { stroke: var(--edge); opacity: var(--edge-rest); transition: opacity .1s; }
.node rect { fill: var(--box); stroke: var(--box-line); }
.node text { fill: var(--box-text); }
svg.focus .edge, svg.focus .node { opacity: 0.12; }
svg.focus .edge.uses { opacity: 1; stroke: var(--uses); stroke-width: 2; }
svg.focus .edge.usedby { opacity: 1; stroke: var(--usedby); stroke-width: 2; }
svg.focus .node.on { opacity: 1; }
.node { cursor: pointer; }
.node.pinned rect { stroke: var(--uses); stroke-width: 2.5; }
#about { min-height: 2.8em; font-size: 15px; color: var(--text); width: WIDTHpx;
  max-width: none; }
#about b { margin-right: 6px; }
#about .hint { color: var(--hint); }
.key span { display: inline-block; width: 22px; height: 3px; vertical-align: middle;
  margin: 0 4px 0 14px; }
.key .uses { background: var(--uses); } .key .usedby { background: var(--usedby); }
</style>
<script>
// Before the body draws, so a light choice never flashes dark first.
if (localStorage.getItem("crate_map_theme") === "light")
  document.documentElement.dataset.theme = "light";
</script></head><body>
<button id="theme" type="button"></button>
<h1>Unlimited Tool Works: how work moves from a plan to a release</h1>
<p>Each row is one participant, each column one stage. The lead plans, briefs, reviews
and talks to the maintainer; sidekicks implement, two or three at once, the release's blocking
path first; up to three reviewer families critique a surface, cheapest first, as deep as a
defect in it would cost; tests, gates and mutation runs measure what reading cannot.
Every arrow points from a step to one it feeds. Details live in <code>AGENTS.md</code>,
<code>docs/CONTRIBUTING.md</code> and <code>docs/RUNBOOK.md</code>.</p>
<p class="key">Click a step to keep it highlighted and put it in the address
(<code>#brief</code>), again to let go:<span class="uses"></span>what it feeds
<span class="usedby"></span>what feeds it</p>
<p id="about"><span class="hint">Hover over a step to see what it is.</span></p>
SVG
<script>
const themeButton = document.getElementById("theme");
function labelTheme() {
  const light = document.documentElement.dataset.theme === "light";
  themeButton.textContent = light ? "Dark theme" : "Light theme";
}
themeButton.addEventListener("click", () => {
  const light = document.documentElement.dataset.theme !== "light";
  if (light) document.documentElement.dataset.theme = "light";
  else delete document.documentElement.dataset.theme;
  localStorage.setItem("crate_map_theme", light ? "light" : "dark");
  labelTheme();
});
labelTheme();
const svg = document.querySelector("svg");
const edges = [...svg.querySelectorAll(".edge")];
const nodes = [...svg.querySelectorAll(".node")];
const about = document.getElementById("about");
const hint = about.firstElementChild;
let pinned = null;
// Leaving a box waits before the map falls back to the pinned one (or to none): moving
// from one box to the next crossed the gap between them, and the whole map flickered.
const HOVER_OFF_MS = 150;
let leaving = null;
function show(name) {
  for (const edge of edges) edge.classList.remove("uses", "usedby");
  const node = nodes.find(n => n.dataset.name === name);
  if (!node) {
    svg.classList.remove("focus");
    about.replaceChildren(hint);
    return;
  }
  svg.classList.add("focus");
  const on = new Set([name]);
  for (const edge of edges) {
    if (edge.dataset.from === name) { edge.classList.add("uses"); on.add(edge.dataset.to); }
    if (edge.dataset.to === name) { edge.classList.add("usedby"); on.add(edge.dataset.from); }
  }
  for (const other of nodes) other.classList.toggle("on", on.has(other.dataset.name));
  about.innerHTML = "";
  const title = document.createElement("b");
  title.textContent = node.dataset.label;
  about.append(title, node.dataset.about);
}
function pin(name) {
  pinned = nodes.some(n => n.dataset.name === name) ? name : null;
  for (const node of nodes) node.classList.toggle("pinned", node.dataset.name === pinned);
  show(pinned);
  const address = location.href.split("#")[0] + (pinned ? "#" + pinned : "");
  history.replaceState(null, "", address);
}
for (const node of nodes) {
  node.addEventListener("mouseenter", () => {
    clearTimeout(leaving);
    show(node.dataset.name);
  });
  node.addEventListener("mouseleave", () => {
    leaving = setTimeout(() => show(pinned), HOVER_OFF_MS);
  });
  node.addEventListener("click", event => {
    event.stopPropagation();
    pin(node.dataset.name === pinned ? null : node.dataset.name);
  });
}
document.addEventListener("click", event => {
  if (!event.target.closest("button, a")) pin(null);
});
const fromAddress = () => decodeURIComponent(location.hash.slice(1));
window.addEventListener("hashchange", () => pin(fromAddress()));
pin(fromAddress());
</script></body></html>
"""

page = PAGE.replace("WIDTHpx", f"{WIDTH}px").replace("SVG", "".join(svg), 1)
OUT = "docs/web/dev_cycle.html"
with open(OUT + ".new", "w", encoding="utf-8", newline="\n") as f:
    f.write(page)
os.replace(OUT + ".new", OUT)
print(f"wrote {OUT}: {len(NODES)} steps, {len(EDGES)} arrows")
