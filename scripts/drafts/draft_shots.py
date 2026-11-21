"""Capture the drafts' screens and list what changed: `python scripts/drafts/draft_shots.py [text ...]`.

One edit to the drafts' host page, `docs/prototypes/index.html`, changes every tool's draft at
once. Each tool script lists the screens worth a look in its view's `shots`, as the links the
draft itself writes for them (`save_editor/player?dialog=set-stats`). This script opens every
link in a fresh Chromium page at two window sizes, the app's default and the smallest default
a member meets (set as the draft's Window control sets them, then drawn again), and saves the
window's picture as `.tmp/draft_shots/<run>/<link made filesystem-safe>@<size>.png`, `<run>`
the time it started. With texts given it captures only the links holding one of them
(`save_editor/team`), and a filtered run's folder is named `<run>-<texts>` with the texts
made filesystem-safe as the links are. A text no link holds is an error before anything is
captured. An interrupted or failed run leaves no folder. Each shot is then compared with
its newest earlier copy, the same file name in the newest earlier run folder that has it (a
`baseline/` folder reads as older than every run), and one line prints per shot that changed
(with the share of its pixels that differ) or is new, and, on a full run only, per shot
missing: one the newest earlier full run had and this one has not.
Earlier folders whose shots all have a copy in a later run are then deleted, so each shot's
newest copy is always kept and the folder count stays small. The newest full run is never
deleted: the missing rows and the full-run reminder measure from it. Run it with a filter
after each draft edit and once without about once a day: a filtered run whose newest earlier
full run is more than a day old, or that has none, says so.

Every page's clock is frozen before the load (Playwright's `page.clock`, paused at one fixed
time and stepped by hand), so whatever moves with time, such as a long name scrolling on the
Tactics pitch, draws the same on every run. The page reports a 1920 × 1080 monitor, since the
default window's size depends on the monitor's. Chromium still antialiases a rounded border's
corners slightly differently between some captures, so a pixel counts as changed only when it
differs by more than one level in a channel. A line under 0.001% on the Team view's colour
swatch (`save_editor/team?selteam=718`) is that wobble, not an edit.

A shot whose link the draft does not write back once it is open (a key the draft ignores, a
dialog that opens under another name) is reported as an error, because its picture is not the
screen it names.

Needs Python's `playwright` with its Chromium (`playwright install chromium`) and Pillow,
installed developer tools; not a gate. Exit 2 on a text no link holds, 1 when a link is not
read back, otherwise 0.
"""

import re
import string
import sys
from datetime import datetime, timedelta
from functools import reduce
from pathlib import Path
from urllib.parse import unquote

from PIL import Image, ImageChops
from playwright.sync_api import Browser, sync_playwright

ROOT = Path(__file__).resolve().parents[2]
INDEX = ROOT / "docs" / "prototypes" / "index.html"
SHOTS = ROOT / ".tmp" / "draft_shots"
# A run folder's name: its start time; a filtered run's carries a `-` and its texts after it.
RUN_NAME = re.compile(r"\d{8}-\d{6}(-|$)")
# Names of the draft's Window control, which index.html's sizeWindow takes.
SIZES = ["default", "smallest"]
MONITOR = {"width": 1920, "height": 1080}
CLOCK_PAUSED = datetime(2026, 1, 1, 12, 0)
# A pixel counts as changed only past this difference in any channel: antialiasing on a
# rounded border's corner moves by one level, now and then two, between captures. Software
# rendering (`--disable-gpu`) and a fixed scale factor made it rarer, not gone, and cost 39%
# more time, so they were not kept (measured 2026-11-21).
PIXEL_STEP = 1
# A name keeps these characters. Every byte of any other, `~` included, becomes `~` and two hex
# digits, so two links never share a name and a name reads back as its link.
KEPT = frozenset(string.ascii_letters + string.digits + "_.-")


def safe_name(fragment: str) -> str:
    return "".join(chr(byte) if chr(byte) in KEPT else f"~{byte:02X}"
                   for byte in fragment.encode("utf-8"))


def fragment_of(name: str) -> str:
    """The link a `safe_name` stands for (a name holds no `%`, since `safe_name` escapes it)."""
    return unquote(name.replace("~", "%"))


def changed_share(old: Image.Image, new: Image.Image) -> float:
    """The share of the pixels that differ by more than `PIXEL_STEP` in any channel, 1.0 when
    the sizes differ."""
    if old.size != new.size:
        return 1.0
    difference = ImageChops.difference(old.convert("RGBA"), new.convert("RGBA"))
    changed = reduce(ImageChops.lighter, difference.split())
    pixels = changed.width * changed.height
    return (pixels - sum(changed.histogram()[:PIXEL_STEP + 1])) / pixels


def remove_shot_folder(folder: Path, root: Path) -> None:
    """Deletes a folder of shots, its PNG files and then the folder itself, which fails if
    anything else is in it. Refuses a folder that is not directly in `root`."""
    if folder.resolve().parent != root.resolve():
        raise ValueError(f"{folder} is not a folder of {root}")
    for path in folder.glob("*.png"):
        path.unlink()
    folder.rmdir()


def shot_folders(root: Path) -> list[Path]:
    """`root`'s shot folders, oldest first: `baseline/` before every run, the rest by name,
    which sorts by start time, a filtered run's `<time>-<texts>` included."""
    return sorted((folder for folder in root.iterdir() if folder.is_dir()
                   and (folder.name == "baseline" or RUN_NAME.match(folder.name))),
                  key=lambda folder: (folder.name != "baseline", folder.name))


def is_full_run(folder: Path) -> bool:
    """`folder` holds every shot: `baseline/` or a run named `<time>` alone, a filtered run's
    name carrying `-<texts>` after it."""
    return folder.name == "baseline" or RUN_NAME.fullmatch(folder.name) is not None


def match_shots(shots: list, texts: list[str]) -> tuple[list, list[str]]:
    """`shots`, the VIEWS dump of `[tool, links]` rows, filtered to the links holding one of
    `texts` (every link when it is empty), the tools left with none dropped; and the texts no
    link holds."""
    kept = [[tool, [link for link in links if not texts or any(text in link for text in texts)]]
            for tool, links in shots]
    unmatched = [text for text in texts
                 if not any(text in link for _, links in shots for link in links)]
    return [row for row in kept if row[1]], unmatched


def shot_rows(root: Path, run: Path) -> list[tuple[str, str, str]]:
    """One `(kind, shot, detail)` row per shot name `run` holds, plus, when `run` is a full
    run, a "missing" row per shot the newest earlier full run of `root` had and `run` has
    not. Kinds: "changed" (`detail` the share of pixels that differ) or "unchanged" against
    the shot's newest earlier copy, the same file name in the newest earlier folder that has
    it; "new" when no earlier folder has it."""
    new_names = {path.name for path in run.glob("*.png")}
    copies, last_full = {}, None
    for folder in shot_folders(root):
        if folder == run:
            continue
        for path in folder.glob("*.png"):
            copies[path.name] = path
        if is_full_run(folder):
            last_full = folder
    missing = ({path.name for path in last_full.glob("*.png")} - new_names
               if is_full_run(run) and last_full is not None else set())
    rows = []
    for name in sorted(new_names | missing):
        shot = fragment_of(name.removesuffix(".png"))
        if name in missing:
            rows.append(("missing", shot, ""))
        elif name not in copies:
            rows.append(("new", shot, ""))
        else:
            with Image.open(copies[name]) as old, Image.open(run / name) as new:
                share = changed_share(old, new)
            detail = f" ({share:.4%} of its pixels)" if share else ""
            rows.append(("changed" if share else "unchanged", shot, detail))
    return rows


def full_run_reminder(root: Path, run: Path) -> str | None:
    """The line a filtered run prints after its counts: "no full run yet..." when `root`
    holds none, or, when the newest earlier full run is more than a day old, its start time
    (a `baseline/`'s, whose name holds none, from its folder's modification time). `None` on
    a full run and on a recent enough one."""
    if is_full_run(run):
        return None
    folders = [folder for folder in shot_folders(root)
               if folder != run and is_full_run(folder)]
    if not folders:
        return "no full run yet; run once without a filter"
    last = folders[-1]
    stamp = (datetime.fromtimestamp(last.stat().st_mtime) if last.name == "baseline"
             else datetime.strptime(last.name, "%Y%m%d-%H%M%S"))
    if datetime.now() - stamp <= timedelta(days=1):
        return None
    return f"the last full run was {stamp:%Y-%m-%d %H:%M}; run once without a filter"


def prune_runs(root: Path) -> None:
    """Deletes every shot folder of `root` but the newest whose shots all have a copy in a
    later folder, so each shot's newest copy is kept and the folder count stays small. The
    newest full run is never deleted: "missing" rows and the full-run reminder measure from
    it, and filtered runs can cover its shot names between them."""
    folders = shot_folders(root)
    newer = {path.name for path in folders[-1].glob("*.png")} if folders else set()
    last_full_seen = bool(folders) and is_full_run(folders[-1])
    for folder in reversed(folders[:-1]):
        names = {path.name for path in folder.glob("*.png")}
        if names <= newer and (last_full_seen or not is_full_run(folder)):
            remove_shot_folder(folder, root)
        last_full_seen = last_full_seen or is_full_run(folder)
        newer |= names


def capture(browser: Browser, fragment: str, size: str, path: Path) -> str:
    """Saves the window's picture of one link at one size and returns the link the draft
    writes once it is open."""
    page = browser.new_page(screen=MONITOR, viewport=MONITOR)
    # An installed clock runs until it is paused, and it cannot be paused in its own past:
    # it starts an hour early, on a blank page with no timer to fire on the way.
    page.clock.install(time=CLOCK_PAUSED - timedelta(hours=1))
    page.clock.pause_at(CLOCK_PAUSED)
    page.goto(INDEX.as_uri() + "#" + fragment, wait_until="load")
    page.wait_for_function("document.fonts.status === 'loaded'")
    # With the clock paused, frames and timers run only when it is stepped: the first second
    # draws what waits for a frame (the pitch) and closes what waits for a timeout.
    page.clock.run_for(1000)
    page.evaluate("size => { sizeWindow(size); render(); }", size)
    page.clock.run_for(1000)
    page.locator("#win").screenshot(path=path)
    written = page.evaluate("location.hash.slice(1)")
    page.close()
    return written


def capture_all(run: Path, texts: list[str]) -> tuple[list[str], list[str]]:
    """Captures the registered views' shots whose link holds one of `texts` (every shot when
    it is empty) into `run`, which it creates only when every text matched a link and removes
    again when the capture is interrupted or fails. Returns `(unread, unmatched)`: the links
    the draft did not write back, the texts no link holds."""
    unread = []
    with sync_playwright() as pw:
        browser = pw.chromium.launch()
        page = browser.new_page()
        page.goto(INDEX.as_uri(), wait_until="load")
        shots = page.evaluate("Object.values(VIEWS).map(view => [view.id, view.shots || []])")
        page.close()
        kept, unmatched = match_shots(shots, texts)
        if unmatched:
            browser.close()
            return unread, unmatched
        run.mkdir(parents=True)
        try:
            for tool, fragments in kept:
                print(f"{tool}: {len(fragments)} shots at {len(SIZES)} sizes")
                for fragment in fragments:
                    for size in SIZES:
                        written = capture(browser, fragment, size, run / f"{safe_name(fragment)}@{size}.png")
                        if written != fragment:
                            unread.append(f"{fragment}@{size}: the draft writes {written}")
        except BaseException:
            # An interrupted or failed capture leaves no run folder: a partial one would
            # later read as a full run.
            remove_shot_folder(run, run.parent)
            raise
        browser.close()
    return unread, unmatched


def report(root: Path, run: Path) -> None:
    """Prints one line per `shot_rows` row of `run` that is not unchanged, then the counts."""
    counts = {"changed": 0, "new": 0, "missing": 0, "unchanged": 0}
    # A filtered run never checks for missing shots; its line leaves the count out.
    if not is_full_run(run):
        del counts["missing"]
    shots = 0
    for kind, shot, detail in shot_rows(root, run):
        counts[kind] += 1
        shots += kind != "missing"
        if kind != "unchanged":
            print(f"{kind:8} {shot}{detail}")
    print(f"{shots} shots: " + ", ".join(f"{count} {kind}" for kind, count in counts.items()))
    reminder = full_run_reminder(root, run)
    if reminder:
        print(reminder)


def main(argv: list[str]) -> int:
    texts = argv[1:]
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    name = stamp if not texts else stamp + "-" + "+".join(safe_name(text) for text in texts)
    run = SHOTS / name
    unread, unmatched = capture_all(run, texts)
    if unmatched:
        for text in unmatched:
            print(f'error: "{text}" matches no link', file=sys.stderr)
        return 2
    report(SHOTS, run)
    for line in unread:
        print(f"error: {line}")
    prune_runs(SHOTS)
    print(f"run: {run.relative_to(ROOT).as_posix()}")
    return 1 if unread else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
