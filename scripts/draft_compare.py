"""Measure a built window's screen against its draft: `python scripts/draft_compare.py SCREEN`.

Every named element of one screen is measured on both sides and matched by its text, and each
pair's offset is sorted into a band of the drafts' tolerance (`plans/core/gui.md` "Prototypes
before implementation": a pixel accepted, two where matching closer would complicate the
implementation, more a mismatch to fix or rule):

- **over 2 px**: a mismatch;
- **1 to 2 px**: accepted only where matching closer costs code, so each is looked at;
- **within 1 px**: accepted (an offset of exactly 1 included).

A pair's offset is the largest of its left, top, width and height differences. A pair whose
draft side is a text's ink and whose app side is a widget holding it (a collapsing header's
button, its arrow included) is listed apart, under "box kinds differ": the two boxes differ by
design, so its offset says nothing; look at it in the two captures instead. A checkbox's
width is not compared: the draft's label spans its grid cell, egui's checkbox its content.

- The app's side: the running build's UI Automation tree (`uia_dump.ps1`: control type, name,
  screen rectangle; egui publishes its widgets through AccessKit, which Windows exposes there),
  moved into client coordinates. The build is the one process named `UTW.exe` with a visible
  titled window, or the one whose path holds `--app FRAGMENT` when several run.
- The draft's side: the screen's link (`SCREENS`) opened in Chromium at the default window,
  every element whose own text or value names it: a text's ink box (a DOM Range over its text
  node), an input's, select's or button's box, a selectable label's or a checkbox label's
  whole box (one widget in egui); in coordinates of the window body (`#win-body`, the app's
  client area).
- Matching: by text (a combo's first line; trimmed). A text met n times on both sides is paired
  in reading order (top to bottom, then left to right); a text with different counts on the
  two sides is listed apart, unmatched.

- Ink: every text is also measured in both captures (the draft's page screenshot, the build's
  PrintWindow capture): its glyphs' core colour and its ink mass (`ink`). A colour more than
  `COLOUR_STEP` levels apart, or a weight more than `WEIGHT_STEP` off the screen's median
  ratio (the two rasterizers differ by a steady amount, egui's about 15% heavier), is listed
  under "text colour differs" or "text weight differs": a colour reads as weight to the eye,
  and a box's place says nothing of either.

Prints the count per band, then every pair over 2 and every pair from 1 to 2, largest first,
then the ink findings and the unmatched texts; writes every pair with its band, and a text's
colours, step and weight, to `.tmp/draft_compare/<screen>.tsv`.
Open the screen in the built window first (the default 1280 x 780 client area, the draft's).

    python scripts/draft_compare.py SCREEN [--modal CSS] [--app FRAGMENT]

--modal compares only a dialog: CSS selects the draft's dialog element (`#overlay .window`,
`.help-win`), and both sides are measured from the dialog's title text.

Needs Windows (UI Automation), Python's `playwright` with its Chromium and `psutil`; a
developer tool, not a gate. Its pure parts are tested by `draft_compare_test.py`.
"""

import io
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DRAFTS = (ROOT / "docs" / "prototypes" / "index.html").as_uri()
OUT = ROOT / ".tmp" / "draft_compare"

SAVE = "save=pes21&pes=2021"
# The screens measured so far: each one's draft link. The built window shows the same screen
# with the PES 21 sandbox save open (`EDIT17` for `aatf`, whose draft rows are recorded for the
# PES 2017 sample only).
SCREENS = {
    "firstrun": "#save_editor?settings=none&pes=2021",
    "empty": "#save_editor?pes=2021",
    "about": "#save_editor?pes=2021&win=about",
    "settings": "#save_editor?pes=2021&win=settings",
    "help": "#save_editor?pes=2021&help=save-editor/{first_topic}",
    "player": f"#save_editor/player?{SAVE}&player=701.1",
    "team": f"#save_editor/team?{SAVE}&selteam=701",
    "comparator": "#save_editor/player?pes=2021&player=701.1&panel=comparator",
    "aatf": "#save_editor/player?pes=2021&player=718.1",
    "set_stats": f"#save_editor/player?{SAVE}&player=701.1&dialog=set-stats",
    "kit_slots": f"#save_editor/team?{SAVE}&selteam=701&dialog=kit-slots",
}
# Scripts run before a screen's page loads: Live AATF is a tool setting the draft keeps in
# localStorage, not a link key.
INIT = {"aatf": "localStorage.setItem('save_editor.live_aatf', 'true')"}

BANDS = ("over 2 px", "1 to 2 px", "within 1 px")
KINDS_DIFFER = "box kinds differ"
# A text's ink differs when its core colour is more than this many levels apart on a channel
# (the two rasterizers leave body text about 10 apart in its stems), or its weight more than
# this share off its screen's median.
COLOUR_STEP = 20
WEIGHT_STEP = 0.2

# A dump row's last column, the rectangle: `x,y wxh`, negative on a monitor left of the main one.
RECT = re.compile(r"(-?\d+),(-?\d+) (\d+)x(\d+)")

# The draft side's elements: a leaf text's ink box per text node, and the boxes of controls.
DRAFT_JS = """(sel) => {
  const body = document.getElementById('win-body').getBoundingClientRect();
  const out = [];
  const visible = r => r.width > 0 && r.height > 0;
  const push = (name, r, kind) => {
    name = (name || '').trim().split('\\n')[0].trim();
    if (!name || !visible(r)) return;
    out.push([kind, name, r.x - body.x, r.y - body.y, r.width, r.height]);
  };
  const root = sel ? document.querySelector(sel) : document.getElementById('win-body');
  for (const el of root.querySelectorAll('input, select, button, textarea')) {
    const st = getComputedStyle(el);
    if (st.visibility === 'hidden' || st.display === 'none') continue;
    const name = el.tagName === 'SELECT' ? el.options[el.selectedIndex]?.text
      : el.tagName === 'BUTTON' ? el.textContent : (el.value || el.placeholder);
    push(name, el.getBoundingClientRect(), el.tagName.toLowerCase());
  }
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    if (!n.textContent.trim()) continue;
    const p = n.parentElement;
    if (p.closest('button, select, option, textarea')) continue;
    const st = getComputedStyle(p);
    if (st.visibility === 'hidden' || st.display === 'none') continue;
    const sel = p.closest('.sel');
    const label = p.closest('label');
    if (sel) { push(n.textContent, sel.getBoundingClientRect(), 'sel'); continue; }
    if (label && label.querySelector('input[type=checkbox], input[type=radio]')) {
      push(n.textContent, label.getBoundingClientRect(), 'check'); continue;
    }
    const range = document.createRange();
    range.selectNodeContents(n);
    push(n.textContent, range.getBoundingClientRect(), 'text');
  }
  return out;
}"""


def band(draft_kind, app_kind, delta):
    """The band of a pair whose left, top, width and height differ by `delta`; `KINDS_DIFFER`
    when the draft measured a text's ink and the app a widget holding it (a collapsing
    header's button, its arrow included; the rail's logo button), whose boxes differ by
    design however well the two line up."""
    if draft_kind == "text" and app_kind != "Text":
        return KINDS_DIFFER
    # A checkbox's label in the draft spans its grid cell, where egui's checkbox is as wide as
    # its box and text: its width says nothing, its place and height do.
    if draft_kind == "check":
        delta = [delta[0], delta[1], 0.0, delta[3]]
    worst = max(abs(v) for v in delta)
    if worst > 2:
        return BANDS[0]
    if worst > 1:
        return BANDS[1]
    return BANDS[2]


def parse_dump(text, origin):
    """`uia_dump.ps1`'s lines as `[kind, name, x, y, w, h]` rows in client coordinates, the
    client area's screen origin `origin`; a name's first line only, nameless rows dropped."""
    ox, oy = origin
    rows = []
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        # A name with a line break (a combo's value and its hover text) continues on the next
        # lines until the rectangle's column, told by its whole shape: a value such as
        # `4cc Autumn 26` starts with a digit too.
        while " | " in line and not RECT.fullmatch(line.rstrip().split(" | ")[-1]) and i + 1 < len(lines):
            i += 1
            line = line + "\n" + lines[i]
        i += 1
        parts = line.split(" | ")
        rect = RECT.fullmatch(parts[-1].rstrip())
        if len(parts) < 3 or not parts[0].startswith("ControlType.") or not rect:
            continue
        kind, name = parts[0][len("ControlType."):], " | ".join(parts[1:-1])
        x, y, w, h = (int(v) for v in rect.groups())
        name = name.split("\n")[0].strip()
        if name:
            rows.append([kind, name, x - ox, y - oy, w, h])
    return rows


def pair(draft, app):
    """Match draft and app rows by name: `(pairs, unmatched)`, each pair `(name, draft row, app
    row, [dx, dy, dw, dh])`, each unmatched `(name, draft count, app count)`."""
    by_name = defaultdict(lambda: ([], []))
    for row in draft:
        by_name[row[1]][0].append(row)
    for row in app:
        by_name[row[1]][1].append(row)
    pairs, unmatched = [], []

    # Reading order: rows within 4 points of one another count as one line.
    def order(row):
        return (round(row[3] / 4), row[2])

    for name, (ds, apps) in by_name.items():
        if len(ds) != len(apps):
            unmatched.append((name, len(ds), len(apps)))
            continue
        for d, a in zip(sorted(ds, key=order), sorted(apps, key=order)):
            pairs.append((name, d, a, [a[k] - d[k] for k in (2, 3, 4, 5)]))
    return pairs, unmatched


def luminance(pixel):
    return 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2]


def ink(image, box):
    """A text's ink inside `box` (x, y, w, h) of `image`: `(core colour, mass)`, the core colour
    the mean of the pixels at 80% or more of the box's strongest contrast against its
    background (the box's most common colour), the glyphs' stems rather than their
    anti-aliased edges; the mass the summed contrast. `None` for an empty or blank box."""
    x0, y0 = round(box[0]), round(box[1])
    x1, y1 = round(box[0] + box[2]), round(box[1] + box[3])
    pixels = [image.getpixel((x, y)) for y in range(max(y0, 0), min(y1, image.height))
              for x in range(max(x0, 0), min(x1, image.width))]
    if not pixels:
        return None
    background = luminance(Counter(pixels).most_common(1)[0][0])
    contrast = [abs(luminance(p) - background) for p in pixels]
    peak = max(contrast)
    if peak == 0:
        return None
    core = [p for p, c in zip(pixels, contrast) if c >= 0.8 * peak]
    colour = tuple(round(sum(p[i] for p in core) / len(core)) for i in range(3))
    return colour, sum(contrast)


def ink_findings(pairs, draft_image, built_image, draft_shift=(0, 0), app_shift=(0, 0)):
    """Every pair whose draft side is a text, its ink on both sides: `{pair index: (draft
    colour, built colour, colour step, weight)}`, the step the largest channel difference and
    the weight the built side's ink mass over the draft's, divided by the screen's median
    ratio: a browser and egui raster a font apart (egui's about 15% heavier at 12.5 points), so
    only a text heavier or lighter than its screen's others is a finding. The built side is
    measured in its own box, or in the draft's where the app's box is the widget holding the
    text (box kinds differ), whose text lines up. The shifts put a dialog's rows, measured from
    its title, back into each capture's coordinates."""
    found = {}
    for index, (_, d, a, _) in enumerate(pairs):
        if d[0] != "text":
            continue
        draft_box = (d[2] + draft_shift[0], d[3] + draft_shift[1], d[4], d[5])
        app_box = (a if a[0] == "Text" else d)[2:6]
        app_box = (app_box[0] + app_shift[0], app_box[1] + app_shift[1], app_box[2], app_box[3])
        on_draft, on_app = ink(draft_image, draft_box), ink(built_image, app_box)
        if on_draft and on_app:
            step = max(abs(x - y) for x, y in zip(on_draft[0], on_app[0]))
            found[index] = (on_draft[0], on_app[0], step, on_app[1] / on_draft[1])
    ratios = sorted(f[3] for f in found.values())
    median = ratios[len(ratios) // 2] if ratios else 1.0
    return {i: (dc, ac, step, ratio / median) for i, (dc, ac, step, ratio) in found.items()}


def to_dialog(draft, app):
    """Both sides relative to the dialog's title (the draft's first text), the app's rows kept
    only when the draft names them and they stand inside the dialog: `(draft, app, line)`,
    `line` the title's placement difference to print, then `shifts`, each side's title
    position, which puts a row back into its capture's coordinates."""
    title = next(r for r in draft if r[0] == "text")
    app_titles = [r for r in app if r[1] == title[1] and r[0] == "Text"]
    assert app_titles, f"the app has no text {title[1]!r}"
    anchor = max(app_titles, key=lambda r: r[2])
    shifts = ((title[2], title[3]), (anchor[2], anchor[3]))
    line = (f"title {title[1]!r}: draft at {title[2]:.1f},{title[3]:.1f}, app at "
            f"{anchor[2]},{anchor[3]} (d x{anchor[2] - title[2]:+.1f} y{anchor[3] - title[3]:+.1f})")
    names = {r[1] for r in draft}
    # The sidebar's rail repeats some names: only rows right of and below the title, less a
    # margin.
    app = [r for r in app if r[1] in names and r[2] >= anchor[2] - 30 and r[3] >= anchor[3] - 30]
    # A labelled control carries its label's name in the app (the draft names it by its
    # value): kept apart, so the label's text matches the draft's.
    texts = {r[1] for r in app if r[0] == "Text"}
    app = [r for r in app if r[0] == "Text" or r[1] not in texts]
    draft = [[r[0], r[1], r[2] - title[2], r[3] - title[3], r[4], r[5]] for r in draft]
    app = [[r[0], r[1], r[2] - anchor[2], r[3] - anchor[3], r[4], r[5]] for r in app]
    return draft, app, line, shifts


def built_window(fragment):
    """The build's process id and its client area's screen origin."""
    import ctypes
    import ctypes.wintypes as wt

    import psutil

    user32 = ctypes.windll.user32
    pids = {p.pid: p.info["exe"] for p in psutil.process_iter(["exe", "name"])
            if (p.info["name"] or "").lower() == "utw.exe"
            and (fragment is None or fragment in (p.info["exe"] or ""))}
    windows = []
    proto = ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)

    def collect(hwnd, _):
        owner = wt.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value in pids and user32.IsWindowVisible(hwnd) and user32.GetWindowTextLengthW(hwnd):
            windows.append((owner.value, hwnd))
        return True

    user32.EnumWindows(proto(collect), 0)
    owners = {pid for pid, _ in windows}
    if len(owners) != 1:
        found = ", ".join(sorted(pids[pid] for pid in owners)) or "none"
        sys.exit(f"draft_compare: one built window wanted, found {len(owners)} ({found}); "
                 "pass --app with a fragment of the wanted one's path")
    pid, hwnd = windows[0]
    point = wt.POINT(0, 0)
    user32.ClientToScreen(hwnd, ctypes.byref(point))
    return pid, hwnd, (point.x, point.y)


def capture(hwnd):
    """The window's client area as an image, through PrintWindow: the window's own drawing,
    whatever covers it on the screen."""
    import ctypes
    import ctypes.wintypes as wt

    from PIL import Image

    user32, gdi32 = ctypes.windll.user32, ctypes.windll.gdi32
    rect = wt.RECT()
    user32.GetClientRect(hwnd, ctypes.byref(rect))
    w, h = rect.right, rect.bottom
    hdc = user32.GetDC(hwnd)
    mem = gdi32.CreateCompatibleDC(hdc)
    bitmap = gdi32.CreateCompatibleBitmap(hdc, w, h)
    gdi32.SelectObject(mem, bitmap)
    # PW_CLIENTONLY | PW_RENDERFULLCONTENT
    assert user32.PrintWindow(hwnd, mem, 3), "PrintWindow failed"
    buffer = ctypes.create_string_buffer(w * h * 4)

    class BitmapInfo(ctypes.Structure):
        _fields_ = [("biSize", wt.DWORD), ("biWidth", wt.LONG), ("biHeight", wt.LONG),
                    ("biPlanes", wt.WORD), ("biBitCount", wt.WORD), ("biCompression", wt.DWORD),
                    ("biSizeImage", wt.DWORD), ("biXPelsPerMeter", wt.LONG),
                    ("biYPelsPerMeter", wt.LONG), ("biClrUsed", wt.DWORD),
                    ("biClrImportant", wt.DWORD)]

    # A negative height reads the rows top-down.
    info = BitmapInfo(ctypes.sizeof(BitmapInfo), w, -h, 1, 32, 0, 0, 0, 0, 0, 0)
    gdi32.GetDIBits(mem, bitmap, 0, h, buffer, ctypes.byref(info), 0)
    image = Image.frombuffer("RGBA", (w, h), buffer, "raw", "BGRA", 0, 1).convert("RGB")
    gdi32.DeleteObject(bitmap)
    gdi32.DeleteDC(mem)
    user32.ReleaseDC(hwnd, hdc)
    return image


def app_side(fragment):
    """The build's named elements in client coordinates, and its client area's capture."""
    pid, hwnd, origin = built_window(fragment)
    dump = subprocess.run(
        ["powershell", "-NoProfile", "-File", str(ROOT / "scripts" / "uia_dump.ps1"), str(pid)],
        capture_output=True, text=True, encoding="utf-8", errors="replace", check=True,
    ).stdout
    return parse_dump(dump, origin), capture(hwnd)


def draft_side(screen, modal):
    from playwright.sync_api import sync_playwright

    with sync_playwright() as pw:
        browser = pw.chromium.launch()
        page = browser.new_page(viewport={"width": 1900, "height": 1000},
                                screen={"width": 1920, "height": 1080}, device_scale_factor=1)
        if screen in INIT:
            page.add_init_script(INIT[screen])
        link = SCREENS[screen]
        if "{first_topic}" in link:
            page.goto(DRAFTS + "#save_editor")
            page.wait_for_timeout(400)
            first = page.evaluate("() => HELP.find(c => c.id === 'save-editor').topics[0].slug")
            link = link.replace("{first_topic}", first)
        page.goto(DRAFTS + link)
        page.evaluate("() => document.fonts.ready")
        page.wait_for_timeout(800)
        rows = page.evaluate(DRAFT_JS, modal)
        body = page.evaluate("() => { const r = document.getElementById('win-body')"
                             ".getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; }")
        shot = page.screenshot(clip={"x": body[0], "y": body[1], "width": body[2], "height": body[3]})
        browser.close()
    from PIL import Image

    return rows, Image.open(io.BytesIO(shot)).convert("RGB")


def option(name):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else None


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in SCREENS:
        sys.exit(f"usage: draft_compare.py SCREEN [--modal CSS] [--app FRAGMENT]; SCREEN one of "
                 f"{', '.join(SCREENS)}")
    screen, modal = sys.argv[1], option("--modal")
    (app, built_image), (draft, draft_image) = app_side(option("--app")), draft_side(screen, modal)
    shifts = ((0, 0), (0, 0))
    if modal:
        draft, app, line, shifts = to_dialog(draft, app)
        print(line)
    pairs, unmatched = pair(draft, app)
    inks = ink_findings(pairs, draft_image, built_image, *shifts)
    banded = defaultdict(list)
    for p in pairs:
        banded[band(p[1][0], p[2][0], p[3])].append(p)
    OUT.mkdir(parents=True, exist_ok=True)
    with open(OUT / f"{screen}.tsv", "w", encoding="utf-8", newline="\n") as handle:
        handle.write("band\tname\tdraft kind\tapp kind\tdraft x\ty\tw\th\tapp x\ty\tw\th\tdx\tdy\tdw\tdh"
                     "\tdraft colour\tbuilt colour\tcolour step\tweight\n")
        order = sorted(range(len(pairs)), key=lambda i: (pairs[i][1][3], pairs[i][1][2]))
        for i in order:
            name, d, a, delta = pairs[i]
            cells = [band(d[0], a[0], delta), name, d[0], a[0]] + [f"{v:.1f}" for v in d[2:] + a[2:] + delta]
            if i in inks:
                dc, ac, step, weight = inks[i]
                cells += ["#%02x%02x%02x" % dc, "#%02x%02x%02x" % ac, str(step), f"{weight:.2f}"]
            handle.write("\t".join(cells) + "\n")
    counts = ", ".join(f"{len(banded[b])} {b}" for b in BANDS + (KINDS_DIFFER,))
    print(f"{screen}: {len(pairs)} matched ({counts}), {len(unmatched)} texts unmatched")
    for name in BANDS[:2] + (KINDS_DIFFER,):
        if banded[name]:
            print(f"{name}:")
        for text, d, a, delta in sorted(banded[name], key=lambda p: -max(abs(v) for v in p[3])):
            print(f"  {text[:40]:40} draft {d[2]:6.1f},{d[3]:6.1f} {d[4]:5.1f}x{d[5]:4.1f}  "
                  f"d x{delta[0]:+5.1f} y{delta[1]:+5.1f} w{delta[2]:+5.1f} h{delta[3]:+5.1f}  "
                  f"({d[0]}/{a[0]})")
    colour = [(i, f) for i, f in inks.items() if f[2] > COLOUR_STEP]
    weight = [(i, f) for i, f in inks.items() if abs(f[3] - 1) > WEIGHT_STEP]
    print(f"text ink: {len(inks)} texts measured, {len(colour)} colours more than {COLOUR_STEP} "
          f"apart, {len(weight)} weights more than {WEIGHT_STEP:.0%} off the screen's")
    for title, found in (("text colour differs", colour), ("text weight differs", weight)):
        if found:
            print(f"{title}:")
        for i, (dc, ac, step, ratio) in sorted(found, key=lambda f: -max(f[1][2], 100 * abs(f[1][3] - 1))):
            print(f"  {pairs[i][0][:40]:40} draft #{dc[0]:02x}{dc[1]:02x}{dc[2]:02x} built "
                  f"#{ac[0]:02x}{ac[1]:02x}{ac[2]:02x} (step {step}), weight {ratio:.2f}")
    if unmatched:
        print("unmatched (text: draft count, app count):")
        for text, nd, na in sorted(unmatched):
            print(f"  {text[:50]!r}: {nd}, {na}")
    print(f"table: {OUT / (screen + '.tsv')}")

if __name__ == "__main__":
    main()
