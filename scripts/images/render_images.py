"""Render the suite's own drawn images to the PNGs the binary embeds.
`python scripts/images/render_images.py`.

Each image below is drawn as an SVG and committed beside its PNG; the PNG is what the code
embeds (`include_bytes!`), since the app has no SVG renderer. Run this after editing an SVG,
then commit both. It renders each SVG at its own `width` x `height` in Playwright's Chromium,
the background transparent where the SVG paints none, and refuses to write a PNG whose size is
not the SVG's.

- `resources/portraits/player-dummy_{15,16,19}.svg`: the Tactics pitch's placeholder portraits,
  drawings of each game's `player-dummy.png` (`resources/portraits/README.md`);
- `resources/thumbnails/team_export.svg`: the Team Export file's thumbnail, the 4cc clover
  (`resources/thumbnails/README.md`).

Needs Python's `playwright` with its Chromium (`playwright install chromium`) and Pillow; a
development tool, not a gate.
"""

import io
import os
import re
import sys
from pathlib import Path

from PIL import Image
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
SVGS = [
    ROOT / "resources" / "portraits" / "player-dummy_15.svg",
    ROOT / "resources" / "portraits" / "player-dummy_16.svg",
    ROOT / "resources" / "portraits" / "player-dummy_19.svg",
    ROOT / "resources" / "thumbnails" / "team_export.svg",
]


def size_of(svg: str) -> tuple[int, int]:
    """The root element's `width` and `height`, in pixels."""
    root = re.search(r"<svg\b[^>]*>", svg)
    if root is None:
        raise ValueError("no <svg> element")
    width = re.search(r'\bwidth="(\d+)"', root.group(0))
    height = re.search(r'\bheight="(\d+)"', root.group(0))
    if width is None or height is None:
        raise ValueError("the <svg> element needs a whole width and height")
    return int(width.group(1)), int(height.group(1))


def main() -> None:
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        page = browser.new_page(device_scale_factor=1)
        for path in SVGS:
            svg = path.read_text(encoding="utf-8")
            width, height = size_of(svg)
            page.set_viewport_size({"width": width, "height": height})
            page.set_content(f"<html><body style='margin:0;background:transparent'>{svg}</body></html>")
            png = page.locator("svg").first.screenshot(omit_background=True)
            rendered = Image.open(io.BytesIO(png)).size
            if rendered != (width, height):
                sys.exit(f"{path.name}: rendered {rendered}, expected {(width, height)}")
            out = path.with_suffix(".png")
            temp = out.with_suffix(".png.tmp")
            temp.write_bytes(png)
            os.replace(temp, out)
            print(f"{out.relative_to(ROOT)}: {width} x {height}, {len(png)} bytes")
        browser.close()


if __name__ == "__main__":
    main()
