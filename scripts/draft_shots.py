"""Capture every screen of the drafts and list what changed: `python scripts/draft_shots.py`.

One edit to the drafts' host page, `docs/prototypes/index.html`, changes every tool's draft at
once. Each tool script lists the screens worth a look in its view's `shots`, as the links the
draft itself writes for them (`save_editor/player?dialog=set-stats`). This script opens every
link in a fresh Chromium page at two window sizes, the app's default and the smallest default
a member meets (set as the draft's Window control sets them, then drawn again), and saves the
window's picture as `.tmp/draft_shots/<run>/<link made filesystem-safe>@<size>.png`, `<run>`
the time it started. It then compares each picture with the one of the same name in
`.tmp/draft_shots/baseline/` and prints one line per shot that changed (with the share of its
pixels that differ), is new or is missing, then a summary line. With `--update` the run then
becomes the baseline.

Every page's clock is frozen before the load (Playwright's `page.clock`, paused at one fixed
time and stepped by hand), so whatever moves with time, such as a long name scrolling on the
Tactics pitch, draws the same on every run. The page reports a 1920 × 1080 monitor, since the
default window's size depends on the monitor's.

A shot whose link the draft does not write back once it is open (a key the draft ignores, a
dialog that opens under another name) is reported as an error, because its picture is not the
screen it names.

Needs Python's `playwright` with its Chromium (`playwright install chromium`) and Pillow,
installed developer tools; not a gate. Exit 1 when a link is not read back, otherwise 0.
"""

import os
import string
import sys
from datetime import datetime, timedelta
from functools import reduce
from pathlib import Path
from urllib.parse import unquote

from PIL import Image, ImageChops
from playwright.sync_api import Browser, sync_playwright

ROOT = Path(__file__).resolve().parents[1]
INDEX = ROOT / "docs" / "prototypes" / "index.html"
SHOTS = ROOT / ".tmp" / "draft_shots"
BASELINE = SHOTS / "baseline"
# Names of the draft's Window control, which index.html's sizeWindow takes.
SIZES = ["default", "smallest"]
MONITOR = {"width": 1920, "height": 1080}
CLOCK_PAUSED = datetime(2026, 1, 1, 12, 0)
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
    """The share of the pixels that differ in any channel, 1.0 when the sizes differ."""
    if old.size != new.size:
        return 1.0
    difference = ImageChops.difference(old.convert("RGBA"), new.convert("RGBA"))
    changed = reduce(ImageChops.lighter, difference.split())
    pixels = changed.width * changed.height
    return (pixels - changed.histogram()[0]) / pixels


def remove_shot_folder(folder: Path) -> None:
    """Deletes a folder of shots, its PNG files and then the folder itself, which fails if
    anything else is in it. Refuses a folder that is not directly in `.tmp/draft_shots/`."""
    if folder.resolve().parent != SHOTS.resolve():
        raise ValueError(f"{folder} is not a folder of {SHOTS}")
    for path in folder.glob("*.png"):
        path.unlink()
    folder.rmdir()


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


def capture_all(run: Path) -> list[str]:
    """Captures every registered view's shots into `run`; returns the links not read back."""
    unread = []
    with sync_playwright() as pw:
        browser = pw.chromium.launch()
        page = browser.new_page()
        page.goto(INDEX.as_uri(), wait_until="load")
        shots = page.evaluate("Object.values(VIEWS).map(view => [view.id, view.shots || []])")
        page.close()
        for tool, fragments in shots:
            print(f"{tool}: {len(fragments)} shots at {len(SIZES)} sizes")
            for fragment in fragments:
                for size in SIZES:
                    written = capture(browser, fragment, size, run / f"{safe_name(fragment)}@{size}.png")
                    if written != fragment:
                        unread.append(f"{fragment}@{size}: the draft writes {written}")
        browser.close()
    return unread


def report(run: Path, baseline: Path) -> None:
    """Prints a line per shot of `run` changed, new or missing against `baseline`, then the
    counts."""
    new_names = {path.name for path in run.glob("*.png")}
    old_names = {path.name for path in baseline.glob("*.png")}
    counts = {"changed": 0, "new": 0, "missing": 0, "unchanged": 0}
    for name in sorted(new_names | old_names):
        shot = fragment_of(name.removesuffix(".png"))
        if name not in old_names:
            kind, detail = "new", ""
        elif name not in new_names:
            kind, detail = "missing", ""
        else:
            with Image.open(baseline / name) as old, Image.open(run / name) as new:
                share = changed_share(old, new)
            kind, detail = ("changed", f" ({share:.4%} of its pixels)") if share else ("unchanged", "")
        counts[kind] += 1
        if kind != "unchanged":
            print(f"{kind:8} {shot}{detail}")
    print(f"{len(new_names)} shots against {len(old_names)} in the baseline: "
          + ", ".join(f"{count} {kind}" for kind, count in counts.items()))


def main(argv: list[str]) -> int:
    if argv[1:] not in ([], ["--update"]):
        print("usage: python scripts/draft_shots.py [--update]", file=sys.stderr)
        return 2
    run = SHOTS / datetime.now().strftime("%Y%m%d-%H%M%S")
    run.mkdir(parents=True)
    unread = capture_all(run)
    report(run, BASELINE)
    for line in unread:
        print(f"error: {line}")
    if argv[1:] == ["--update"]:
        # The new baseline takes the old one's place in one rename; the old one is deleted
        # only after.
        previous = SHOTS / "baseline.old"
        if BASELINE.exists():
            os.replace(BASELINE, previous)
        os.replace(run, BASELINE)
        if previous.exists():
            remove_shot_folder(previous)
        print(f"baseline: {BASELINE.relative_to(ROOT).as_posix()}")
    else:
        print(f"run: {run.relative_to(ROOT).as_posix()}")
    return 1 if unread else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
