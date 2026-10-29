"""A draft change's before and after, side by side, as one image the maintainer reviews.

Run: python scripts/vet_drafts.py --before REV --after REV --link FRAGMENT --why TEXT
         [--caption TEXT] [--crop CSS] [--pad N] [--click TEXT] [--out PATH] [--exports DIR]

Each revision (a git revision of this repository, or `work` for the working tree's
`docs/prototypes`) is exported into a scratch folder, the repository's gitignored
`.tmp/vet_exports/` unless `--exports` names another: a
commit's export is reused by later runs, `work` is copied afresh every time. The drafts' page
opens at `index.html#FRAGMENT` (the host keeps the tool, screen and draft state in the
fragment, so a remark's link opens the exact state) in headless Chromium at device scale 1, a
1900 x 1000 viewport on a 1920 x 1080 screen; `--click` first clicks the first button whose
name holds that text (`--click "Live AATF"`). The crop is the first element `--crop` matches,
`--pad` points around it, or the window's client area without `--crop`. The two crops go side
by side (stacked when that would be too wide) under the caption and the reason, into `--out`
(by default `.tmp/vet/<before>_<after>.png`).

Playwright (`pip install playwright`, then `playwright install chromium`) and Pillow
(`pip install pillow`) are installed developer tools, like `just`, not project dependencies.
"""

import argparse
import io
import re
import shutil
import subprocess
import sys
import tarfile
import textwrap
from pathlib import Path

from PIL import Image, ImageDraw
from playwright.sync_api import Browser, sync_playwright

ROOT = Path(__file__).resolve().parents[1]
EXPORTS = ROOT / ".tmp" / "vet_exports"
BACKGROUND = (58, 63, 71)
GAP = 16


class VetError(Exception):
    pass


def git(*args: str) -> bytes:
    result = subprocess.run(["git", *args], cwd=ROOT, capture_output=True)
    if result.returncode != 0:
        raise VetError(f"git {' '.join(args)}: {result.stderr.decode(errors='replace').strip()}")
    return result.stdout


def remove_export(path: Path, exports: Path) -> None:
    """Deletes one export; refuses any path that is not inside the exports folder."""
    if exports.resolve() not in path.resolve().parents:
        raise ValueError(f"{path}: not inside the exports folder {exports}, not deleted")
    shutil.rmtree(path)


def export(rev: str, exports: Path) -> Path:
    """The drafts' `index.html` at `rev`, exported under `exports`."""
    if rev == "work":
        dest = exports / "work"
        if dest.exists():
            remove_export(dest, exports)
        shutil.copytree(ROOT / "docs" / "prototypes", dest / "docs" / "prototypes")
    else:
        dest = exports / git("rev-parse", "--short=8", rev).decode().strip()
        if not dest.exists():
            archive = git("archive", rev, "docs/prototypes")
            dest.mkdir(parents=True)
            with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
                tar.extractall(dest, filter="data")
    return dest / "docs" / "prototypes" / "index.html"


def capture(browser: Browser, index: Path, name: str, link: str, click: str | None,
            selector: str | None, pad: int) -> Image.Image:
    """The page at `index.html#link`, cropped to the selector's first match plus `pad`."""
    page = browser.new_page(viewport={"width": 1900, "height": 1000},
                            screen={"width": 1920, "height": 1080}, device_scale_factor=1)
    errors = []
    page.on("pageerror", lambda e: errors.append(str(e)))
    try:
        page.goto(index.as_uri() + "#" + link.removeprefix("#"))
        page.evaluate("() => document.fonts.ready")
        page.wait_for_timeout(1000)
        if click:
            # A substring match: the drafts' toggles start with a glyph (`▶ Live AATF`).
            button = page.get_by_role("button", name=click).first
            if not button.count():
                raise VetError(f"{name}: no button named {click!r}")
            button.click()
            page.wait_for_timeout(1200)
        element = page.locator(selector or "#win-body").first
        if not element.count():
            raise VetError(f"{name}: nothing matches {selector!r}")
        element.scroll_into_view_if_needed()
        box = element.bounding_box()
        shot = Image.open(io.BytesIO(page.screenshot()))
        return shot.crop((max(0, round(box["x"] - pad)), max(0, round(box["y"] - pad)),
                          min(shot.width, round(box["x"] + box["width"] + pad)),
                          min(shot.height, round(box["y"] + box["height"] + pad))))
    finally:
        for error in errors:
            print(f"{name}: page error: {error}")
        page.close()


def why_lines(why: str, crops: list[tuple[str, Image.Image]]) -> list[str]:
    """The reason, prefixed `Why:`, wrapped to the sheet's width: the narrower of the two
    layouts decides, never under 60 characters. PIL's default font is about 6 points a
    character at most."""
    wide_w = sum(c.width for _, c in crops) + GAP * (len(crops) + 1)
    narrow_w = max(c.width for _, c in crops) + 2 * GAP
    return textwrap.wrap("Why: " + why, max(60, (min(wide_w, narrow_w) - 2 * GAP) // 6))


def compose(caption: str, why: str, crops: list[tuple[str, Image.Image]]) -> Image.Image:
    """The crops side by side under the caption, each under its label; stacked instead when side
    by side the sheet would be more than four times as wide as it is tall (a title bar, a
    window's top strip), where the two would shrink to slivers in a viewer."""
    label_h = 16
    lines = why_lines(why, crops)
    wide_w = sum(c.width for _, c in crops) + GAP * (len(crops) + 1)
    head = 26 + 13 * len(lines) + 4
    wide_h = head + label_h + max(c.height for _, c in crops) + GAP
    stacked = wide_w > 4 * wide_h
    if stacked:
        w = max(c.width for _, c in crops) + 2 * GAP
        h = head + sum(label_h + c.height + GAP for _, c in crops)
    else:
        w, h = wide_w, wide_h
    sheet = Image.new("RGB", (w, h), BACKGROUND)
    pen = ImageDraw.Draw(sheet)
    pen.text((GAP, 6), caption, fill=(255, 255, 255))
    for i, line in enumerate(lines):
        pen.text((GAP, 24 + 13 * i), line, fill=(170, 200, 240))
    x, y = GAP, head
    for label, c in crops:
        pen.text((x, y), label, fill=(220, 220, 120))
        sheet.paste(c, (x, y + label_h))
        if stacked:
            y += label_h + c.height + GAP
        else:
            x += c.width + GAP
    return sheet


def default_out(before: str, after: str) -> Path:
    """`.tmp/vet/<before>_<after>.png`, each revision's characters outside `A-Za-z0-9._-`
    replaced by `_` (`HEAD~1` holds a `~`, `abc^` a `^`)."""
    safe = [re.sub(r"[^A-Za-z0-9._-]", "_", rev) for rev in (before, after)]
    return ROOT / ".tmp" / "vet" / f"{safe[0]}_{safe[1]}.png"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="A draft change's before and after, side by side.")
    parser.add_argument("--before", required=True, help="a git revision, or work")
    parser.add_argument("--after", required=True, help="a git revision, or work")
    parser.add_argument("--link", required=True, help="the drafts' URL fragment, the part after #")
    parser.add_argument("--why", required=True, help="the reason for the change")
    parser.add_argument("--caption", help="by default: <before> -> <after>: <link>")
    parser.add_argument("--crop", help="a CSS selector; by default the window's client area")
    parser.add_argument("--pad", type=int, default=8, help="points around the crop (default 8)")
    parser.add_argument("--click", help="a button to click before the capture, by part of its name")
    parser.add_argument("--out", type=Path)
    parser.add_argument("--exports", type=Path, default=EXPORTS)
    args = parser.parse_args(argv[1:])
    # A page error's text can hold characters the Windows console's code page cannot print.
    sys.stdout.reconfigure(errors="backslashreplace")
    # PIL's default font has no arrow: a `→` would draw as a box.
    caption = args.caption or f"{args.before} -> {args.after}: {args.link}"
    out = args.out or default_out(args.before, args.after)
    try:
        with sync_playwright() as pw:
            browser = pw.chromium.launch()
            crops = []
            for label, rev in [("before", args.before), ("after", args.after)]:
                index = export(rev, args.exports)
                crops.append((label, capture(browser, index, f"{label} ({rev})", args.link,
                                             args.click, args.crop, args.pad)))
            browser.close()
    except VetError as e:
        print(f"vet_drafts: {e}", file=sys.stderr)
        return 1
    sheet = compose(caption, args.why, crops)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    print(f"{out}: {sheet.width} x {sheet.height}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
