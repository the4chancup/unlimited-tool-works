"""A draft change's before and after, side by side, as one page the maintainer reviews.

Run: python scripts/drafts/vet_drafts.py --before REV --after REV --link FRAGMENT --why TEXT
         [--caption TEXT] [--crop CSS] [--pad N] [--click TEXT] [--out PATH] [--exports DIR]

Each revision (a git revision of this repository, or `work` for the working tree's
`docs/prototypes`) is exported into a scratch folder, the repository's gitignored
`.tmp/vet_exports/` unless `--exports` names another: a
commit's export is reused by later runs, `work` is copied afresh every time. The drafts' page
opens at `index.html#FRAGMENT` (the host keeps the tool, screen and draft state in the
fragment, so a remark's link opens the exact state) in headless Chromium at device scale 1, a
1900 x 1000 viewport on a 1920 x 1080 screen; `--click` first clicks the first button whose
name holds that text (`--click "Live AATF"`). The crop is the first element `--crop` matches,
`--pad` points around it, or the window's client area without `--crop`. The page `--out` (by
default `.tmp/vet/<before>_<after>.html`) shows the caption and the reason in type large
enough to read, then the two crops side by side, each under its label, wrapping under each
other when the browser window is too narrow for both; the crops are saved beside it as
`<page>_before.png` and `<page>_after.png`, at their own size.

Playwright (`pip install playwright`, then `playwright install chromium`) and Pillow
(`pip install pillow`) are installed developer tools, like `just`, not project dependencies.
"""

import argparse
import html
import io
import re
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

from PIL import Image
from playwright.sync_api import Browser, sync_playwright

ROOT = Path(__file__).resolve().parents[2]
EXPORTS = ROOT / ".tmp" / "vet_exports"
# The drafts' window frame's grey, so a crop's edge reads against the page.
BACKGROUND = "#3a3f47"


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


def crop_path(page: Path, label: str) -> Path:
    """Where the page's crop under `label` is saved: `<page>_<label>.png` beside the page."""
    return page.with_name(f"{page.stem}_{label}.png")


def page_html(caption: str, why: str, crops: list[tuple[str, Path, Image.Image]]) -> str:
    """The page: the caption as its heading, the reason under it prefixed `Why:`, then each
    crop under its label, at its own pixel size (a scaled crop blurs the draft's 1-pixel
    lines). `crops` holds each crop's label, its saved file and its image; every text is
    escaped, as a caption can hold `<` (`Make <tier>`)."""
    figures = "\n".join(
        f'<figure><figcaption>{html.escape(label)}</figcaption>'
        f'<img src="{html.escape(path.name)}" width="{image.width}" height="{image.height}"'
        f' alt="{html.escape(label)}"></figure>'
        for label, path, image in crops)
    return f"""<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>{html.escape(caption)}</title>
<style>
body {{ background: {BACKGROUND}; color: #f0f0f0; font: 17px/1.5 system-ui, sans-serif;
  margin: 24px; }}
h1 {{ font-size: 24px; margin: 0 0 8px; }}
p.why {{ color: #c8dcf8; max-width: 90ch; margin: 0 0 20px; }}
.crops {{ display: flex; flex-wrap: wrap; gap: 24px; align-items: flex-start; }}
figure {{ margin: 0; }}
figcaption {{ color: #e6e68c; font-size: 18px; font-weight: 600; margin-bottom: 6px; }}
img {{ display: block; max-width: none; }}
</style></head><body>
<h1>{html.escape(caption)}</h1>
<p class="why">Why: {html.escape(why)}</p>
<div class="crops">
{figures}
</div>
</body></html>
"""


def default_out(before: str, after: str) -> Path:
    """`.tmp/vet/<before>_<after>.html`, each revision's characters outside `A-Za-z0-9._-`
    replaced by `_` (`HEAD~1` holds a `~`, `abc^` a `^`)."""
    safe = [re.sub(r"[^A-Za-z0-9._-]", "_", rev) for rev in (before, after)]
    return ROOT / ".tmp" / "vet" / f"{safe[0]}_{safe[1]}.html"


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
    caption = args.caption or f"{args.before} → {args.after}: {args.link}"
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
    out.parent.mkdir(parents=True, exist_ok=True)
    saved = []
    for label, image in crops:
        path = crop_path(out, label)
        image.save(path)
        saved.append((label, path, image))
    out.write_bytes(page_html(caption, args.why, saved).encode("utf-8"))
    print(out)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
