"""The drafts as one HTML file: `python scripts/bundle_drafts.py <output.html>`.

The drafts live as the host page `docs/prototypes/index.html` plus one script per tool under
`tools/` and image files under `assets/` (plans/core/gui.md "Prototypes before implementation");
an image's path in a source is a double-quoted literal starting `assets/`, which is how this
script finds the images. A draft that must travel as one attachment is the host
with every `<script src>` replaced by the file's text and every `"assets/..."` literal replaced
by a `data:` URI of the file's bytes, so the output opens alone, from any folder, and draws what
the host draws from the repository.

The output is written through a temporary file next to it and moved over it, never written in
place. Problems (a missing script or image, a script holding `</script`, an extension without a
MIME type, an `src=` or `assets/` left over) are reported as `BundleError` and printed on stderr.
"""

import base64
import os
import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class BundleError(Exception):
    pass


# The built window's captures folder, which the host's Overlay control names once by this
# literal and completes at run time: it is kept as it is, since the captures do not travel
# with an attachment (the bundled overlay shows its missing-file line).
CAPTURES = "assets/captures/"
SCRIPT_TAG = re.compile(r'^[ \t]*<script src="([^"]+)"></script>[ \t]*$', re.M)
ASSET = re.compile(r'"(assets/[^"\n]+)"')
SRC_ATTR = re.compile(r'src="([^"]*)"')
QUOTED_ASSET = re.compile(r'"[^"\n]*assets/[^"\n]*"')
# A single-quoted or template literal: a quote right before "assets/" (a data URI's base64 is
# preceded by base64 characters, never by a quote).
SINGLE_QUOTED_ASSET = re.compile(r"['\x60]assets/[^'\"\x60\n]*")
MIME = {
    ".webp": "image/webp",
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".jpeg": "image/jpeg",
    ".gif": "image/gif",
    ".svg": "image/svg+xml",
    ".ttf": "font/ttf",
}


def bundle(drafts: Path) -> tuple[str, int, int]:
    """`drafts/index.html` with scripts and images inlined; (text, scripts, images)."""
    text = (drafts / "index.html").read_text(encoding="utf-8")

    scripts = 0

    def inline_script(match: re.Match) -> str:
        nonlocal scripts
        name = match.group(1)
        path = drafts / name
        if not path.is_file():
            raise BundleError(f"{name}: no such script under the drafts folder")
        body = path.read_text(encoding="utf-8")
        if "</script" in body.lower():
            raise BundleError(f"{name}: the script contains </script and would end early inline")
        scripts += 1
        return "<script>\n" + body + "</script>"

    text = SCRIPT_TAG.sub(inline_script, text)

    images = 0

    def inline_asset(match: re.Match) -> str:
        nonlocal images
        literal = match.group(1)
        if literal == CAPTURES:
            return match.group(0)
        mime = MIME.get(Path(literal).suffix.lower())
        if mime is None:
            raise BundleError(f'"{literal}": an extension with no MIME type')
        path = drafts / literal
        if not path.is_file():
            raise BundleError(f'"{literal}": no such file under the drafts folder')
        images += 1
        data = base64.b64encode(path.read_bytes()).decode("ascii")
        return f'"data:{mime};base64,{data}"'

    text = ASSET.sub(inline_asset, text)

    for match in SRC_ATTR.finditer(text):
        value = match.group(1)
        if not value.startswith(("data:", "http:", "https:")):
            raise BundleError(f'src="{value}": a source left that was not inlined')
    for match in QUOTED_ASSET.finditer(text):
        # A data URI's base64 can spell "assets/" by chance; only a real literal is an error.
        if not match.group(0).startswith('"data:') and match.group(0) != f'"{CAPTURES}"':
            raise BundleError(f"{match.group(0)}: an assets/ path left in a literal")
    match = SINGLE_QUOTED_ASSET.search(text)
    if match:
        raise BundleError(f"{match.group(0)}: an assets/ path left in a literal")
    return text, scripts, images


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: python scripts/bundle_drafts.py <output.html>", file=sys.stderr)
        return 2
    out = Path(argv[1])
    try:
        text, scripts, images = bundle(ROOT / "docs" / "prototypes")
    except BundleError as e:
        print(f"bundle_drafts: {e}", file=sys.stderr)
        return 1
    tmp = tempfile.NamedTemporaryFile(
        "w", encoding="utf-8", newline="\n", dir=out.parent,
        prefix=out.name + ".", suffix=".tmp", delete=False)
    try:
        with tmp:
            tmp.write(text)
        os.replace(tmp.name, out)
    finally:
        if os.path.exists(tmp.name):
            os.unlink(tmp.name)
    size = out.stat().st_size / 1024
    print(f"{out}: {size:.0f} KB, {scripts} scripts and {images} images inlined")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
